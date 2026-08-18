//! User PATH surgery, plus the shared toolchain root.
//!
//! Windows: current-user Environment key (no admin) and `WM_SETTINGCHANGE`.
//! Unix: `~/.elixir-install/env.sh` (same layout as elixir-install) sourced
//! from the usual shell profiles. Never asks for root.

use crate::error::{AppError, AppResult};
use crate::services::host::{join_path, path_key, path_sep, split_path};
use std::path::{Path, PathBuf};

#[cfg(not(windows))]
const ENV_BEGIN: &str = "# >>> elin PATH >>>";
#[cfg(not(windows))]
const ENV_END: &str = "# <<< elin PATH <<<";

/// Official elixir-install layout, so Elin and `install.sh` / `install.bat` can share installs.
pub fn managed_root() -> AppResult<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| AppError::msg("Could not resolve the user home directory"))?;
    Ok(home.join(".elixir-install").join("installs"))
}

/// Marker used to recognize PATH entries Elin previously wrote for toolchains.
pub fn is_managed_path(entry: &str) -> bool {
    path_key(entry).contains("/.elixir-install/installs/")
}

/// Folder that contains `elin` (install dir, or target/debug while developing).
pub fn elin_install_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

pub fn elin_on_user_path() -> bool {
    let Some(dir) = elin_install_dir() else {
        return false;
    };
    path_contains_dir(&user_path(), &dir) || path_contains_dir(&machine_path(), &dir)
}

/// Prepend the Elin app directory. Never uses the elixir-install marker.
pub fn add_elin_to_path() -> AppResult<String> {
    let dir = elin_install_dir().ok_or_else(|| AppError::msg("Could not resolve Elin's folder."))?;
    if elin_on_user_path() {
        return Ok(format!(
            "`elin` is already on PATH from {}.",
            dir.display()
        ));
    }
    let added = prepend_user_path_dirs(&[dir.clone()])?;
    if added.is_empty() {
        return Ok(format!(
            "{} is already listed on PATH. Open a new terminal.",
            dir.display()
        ));
    }
    Ok(format!(
        "Added {} to the user PATH. Open a new terminal, then run `elin --help`.",
        dir.display()
    ))
}

/// Prepend OTP + Elixir bin folders and drop stale Elin-managed toolchain entries.
pub fn set_user_path_entries(bins: &[PathBuf]) -> AppResult<()> {
    #[cfg(windows)]
    {
        windows_set_path(bins)?;
    }
    #[cfg(not(windows))]
    {
        unix_set_path(bins)?;
    }
    apply_process_path(bins, true);
    Ok(())
}

#[cfg(windows)]
fn windows_set_path(bins: &[PathBuf]) -> AppResult<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|e| AppError::msg(format!("Could not open user Environment key: {e}")))?;

    let current: String = env.get_value("Path").unwrap_or_default();
    let mut parts: Vec<String> = current
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !is_managed_path(s))
        .collect();

    for bin in bins.iter().rev() {
        let value = bin.to_string_lossy().to_string();
        parts.retain(|p| !paths_equal(p, &value));
        parts.insert(0, value);
    }

    let joined = parts.join(";");
    env.set_value("Path", &joined)
        .map_err(|e| AppError::msg(format!("Could not write user PATH: {e}")))?;
    broadcast_setting_change();
    Ok(())
}

fn paths_equal(a: &str, b: &str) -> bool {
    path_key(a) == path_key(b)
}

#[cfg(windows)]
fn broadcast_setting_change() {
    const HWND_BROADCAST: isize = 0xffff;
    const WM_SETTINGCHANGE: u32 = 0x001A;
    const SMTO_ABORTIFHUNG: u32 = 0x0002;
    #[link(name = "user32")]
    extern "system" {
        fn SendMessageTimeoutW(
            hwnd: isize,
            msg: u32,
            wparam: usize,
            lparam: *const u16,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> isize;
    }
    let env: Vec<u16> = "Environment\0".encode_utf16().collect();
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            env.as_ptr(),
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        );
    }
}

/// Read the current user PATH (Windows registry / Unix env.sh) or process PATH.
pub fn user_path() -> String {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(env) = hkcu.open_subkey_with_flags("Environment", KEY_READ) {
            if let Ok(path) = env.get_value::<String, _>("Path") {
                return path;
            }
        }
        std::env::var("PATH").unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        if let Some(managed) = read_unix_env_path() {
            return managed;
        }
        std::env::var("PATH").unwrap_or_default()
    }
}

/// Machine (system) PATH. New consoles see machine PATH + user PATH.
pub fn machine_path() -> String {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
        use winreg::RegKey;
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(env) = hklm.open_subkey_with_flags(
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            KEY_READ,
        ) {
            if let Ok(path) = env.get_value::<String, _>("Path") {
                return path;
            }
        }
        String::new()
    }
    #[cfg(not(windows))]
    {
        String::new()
    }
}

/// PATH a brand-new console actually gets.
pub fn console_path() -> String {
    let machine = expand_env_vars(&machine_path());
    let user = expand_env_vars(&user_path());
    if machine.is_empty() {
        user
    } else if user.is_empty() {
        machine
    } else {
        format!("{}{}{}", machine, path_sep(), user)
    }
}

/// Expand `%VAR%` (Windows) and `$VAR` / `${VAR}` (Unix) the way a new console does.
pub fn expand_env_vars(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(end) = raw[i + 1..].find('%') {
                let name = &raw[i + 1..i + 1 + end];
                if !name.is_empty() {
                    if let Ok(val) = std::env::var(name) {
                        out.push_str(&val);
                        i += name.len() + 2;
                        continue;
                    }
                }
            }
        }
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'{' {
                if let Some(end) = raw[i + 2..].find('}') {
                    let name = &raw[i + 2..i + 2 + end];
                    if let Ok(val) = std::env::var(name) {
                        out.push_str(&val);
                        i += name.len() + 3;
                        continue;
                    }
                }
            } else if bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_' {
                let mut j = i + 1;
                while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                    j += 1;
                }
                let name = &raw[i + 1..j];
                if let Ok(val) = std::env::var(name) {
                    out.push_str(&val);
                    i = j;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

pub fn path_contains_dir(path_var: &str, dir: &Path) -> bool {
    let needle = normalize_dir(dir);
    if needle.is_empty() {
        return false;
    }
    split_path(&expand_env_vars(path_var)).iter().any(|entry| {
        let trimmed = entry.trim().trim_matches('"');
        !trimmed.is_empty() && normalize_dir(Path::new(trimmed)) == needle
    })
}

fn normalize_dir(path: &Path) -> String {
    path_key(&path.to_string_lossy())
}

/// Prepend folders to the user PATH without dropping existing Elin entries.
/// Skips folders already present on user or machine PATH.
pub fn prepend_user_path_dirs(dirs: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
    let added = {
        #[cfg(windows)]
        {
            windows_prepend_path(dirs)?
        }
        #[cfg(not(windows))]
        {
            unix_prepend_path(dirs)?
        }
    };
    if !added.is_empty() {
        apply_process_path(&added, false);
    }
    Ok(added)
}

#[cfg(windows)]
fn windows_prepend_path(dirs: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    let machine = machine_path();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|e| AppError::msg(format!("Could not open user Environment key: {e}")))?;
    let current: String = env.get_value("Path").unwrap_or_default();
    let mut parts: Vec<String> = current
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut added = Vec::new();
    for dir in dirs.iter().rev() {
        if !dir.exists() {
            continue;
        }
        let value = dir.to_string_lossy().to_string();
        if path_contains_dir(&current, dir) || path_contains_dir(&machine, dir) {
            continue;
        }
        parts.retain(|p| !paths_equal(p, &value));
        parts.insert(0, value);
        added.push(dir.clone());
    }

    if added.is_empty() {
        return Ok(added);
    }
    let joined = parts.join(";");
    env.set_value("Path", &joined)
        .map_err(|e| AppError::msg(format!("Could not write user PATH: {e}")))?;
    broadcast_setting_change();
    Ok(added)
}

fn apply_process_path(bins: &[PathBuf], drop_managed: bool) {
    let rest = std::env::var("PATH").unwrap_or_default();
    let mut parts: Vec<String> = Vec::new();
    for bin in bins {
        let value = bin.to_string_lossy().to_string();
        parts.retain(|p| !paths_equal(p, &value));
        parts.push(value);
    }
    for entry in split_path(&rest) {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        if drop_managed && is_managed_path(trimmed) {
            continue;
        }
        if parts.iter().any(|p| paths_equal(p, trimmed)) {
            continue;
        }
        parts.push(trimmed.to_string());
    }
    std::env::set_var("PATH", join_path(&parts));
}

#[cfg(not(windows))]
fn unix_env_sh() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".elixir-install").join("env.sh"))
}

#[cfg(not(windows))]
fn unix_env_fish() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".elixir-install").join("env.fish"))
}

#[cfg(not(windows))]
fn read_unix_env_path() -> Option<String> {
    let path = unix_env_sh()?;
    let text = std::fs::read_to_string(path).ok()?;
    parse_unix_env_path(&text)
}

#[cfg(not(windows))]
fn parse_unix_env_path(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let rest = line.strip_prefix("export PATH=")?;
        let rest = rest.trim().trim_matches('"').trim_matches('\'');
        let prefix = rest
            .trim_end_matches(":$PATH")
            .trim_end_matches("$PATH")
            .trim_end_matches(':');
        if !prefix.is_empty() {
            return Some(prefix.to_string());
        }
    }
    None
}

#[cfg(not(windows))]
fn unix_set_path(bins: &[PathBuf]) -> AppResult<()> {
    let current = read_unix_env_path().unwrap_or_default();
    let mut parts: Vec<String> = split_path(&current)
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !is_managed_path(s))
        .collect();
    for bin in bins.iter().rev() {
        let value = bin.to_string_lossy().to_string();
        parts.retain(|p| !paths_equal(p, &value));
        parts.insert(0, value);
    }
    write_unix_env_files(&parts)?;
    ensure_unix_profiles()?;
    Ok(())
}

#[cfg(not(windows))]
fn unix_prepend_path(dirs: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
    let current = user_path();
    let mut parts: Vec<String> = split_path(&current)
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut added = Vec::new();
    for dir in dirs.iter().rev() {
        if !dir.exists() {
            continue;
        }
        if path_contains_dir(&current, dir) {
            continue;
        }
        let value = dir.to_string_lossy().to_string();
        parts.retain(|p| !paths_equal(p, &value));
        parts.insert(0, value);
        added.push(dir.clone());
    }
    if added.is_empty() {
        return Ok(added);
    }
    write_unix_env_files(&parts)?;
    ensure_unix_profiles()?;
    Ok(added)
}

#[cfg(not(windows))]
fn write_unix_env_files(parts: &[String]) -> AppResult<()> {
    let sh = unix_env_sh().ok_or_else(|| AppError::msg("Could not resolve home directory"))?;
    if let Some(parent) = sh.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let joined = parts.join(":");
    let sh_body = format!(
        "# Generated by Elin. Safe to delete; Elin will rewrite it.\n\
         # https://github.com/belurgas/elin\n\
         export PATH=\"{joined}:$PATH\"\n"
    );
    std::fs::write(&sh, sh_body)?;
    if let Some(fish) = unix_env_fish() {
        let fish_parts = parts.join(" ");
        let fish_body = format!(
            "# Generated by Elin.\n\
             fish_add_path -P {fish_parts}\n"
        );
        let _ = std::fs::write(fish, fish_body);
    }
    Ok(())
}

#[cfg(not(windows))]
fn ensure_unix_profiles() -> AppResult<()> {
    let home = dirs::home_dir().ok_or_else(|| AppError::msg("Could not resolve home directory"))?;
    let snippet = format!(
        "{ENV_BEGIN}\n[ -f \"$HOME/.elixir-install/env.sh\" ] && . \"$HOME/.elixir-install/env.sh\"\n{ENV_END}\n"
    );
    let shell = std::env::var("SHELL").unwrap_or_default();
    let candidates = [
        (".profile", true),
        (".zprofile", shell.contains("zsh") || home.join(".zprofile").exists() || home.join(".zshrc").exists()),
        (".zshrc", home.join(".zshrc").exists()),
        (".bash_profile", shell.contains("bash") || home.join(".bash_profile").exists()),
        (".bashrc", home.join(".bashrc").exists()),
    ];
    for (name, write) in candidates {
        if !write {
            continue;
        }
        upsert_block(&home.join(name), &snippet)?;
    }
    let fish_dir = home.join(".config").join("fish").join("conf.d");
    if fish_dir.exists() || shell.contains("fish") {
        let _ = std::fs::create_dir_all(&fish_dir);
        let fish_snippet = format!(
            "{ENV_BEGIN}\nif test -f $HOME/.elixir-install/env.fish\n    source $HOME/.elixir-install/env.fish\nend\n{ENV_END}\n"
        );
        let _ = std::fs::write(fish_dir.join("elin.fish"), fish_snippet);
    }
    Ok(())
}

#[cfg(not(windows))]
fn upsert_block(path: &Path, block: &str) -> AppResult<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let next = if existing.contains(ENV_BEGIN) {
        replace_block(&existing, block)
    } else if existing.is_empty() {
        block.to_string()
    } else {
        let mut out = existing;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(block);
        out
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, next)?;
    Ok(())
}

#[cfg(not(windows))]
fn replace_block(existing: &str, block: &str) -> String {
    let Some(start) = existing.find(ENV_BEGIN) else {
        return format!("{existing}\n{block}");
    };
    let after = &existing[start..];
    let end = after
        .find(ENV_END)
        .map(|i| start + i + ENV_END.len())
        .unwrap_or(existing.len());
    let mut out = String::new();
    out.push_str(&existing[..start]);
    out.push_str(block);
    let rest = existing[end..].trim_start_matches('\n');
    if !rest.is_empty() {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(rest);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[cfg(windows)]
    #[test]
    fn expands_percent_vars() {
        let expanded = expand_env_vars(r"%SystemRoot%\system32");
        assert!(!expanded.contains('%'), "{expanded}");
        assert!(expanded.to_lowercase().contains("system32"));
    }

    #[test]
    fn expands_dollar_home() {
        std::env::set_var("ELIN_EXPAND_TEST", "ok-value");
        assert_eq!(expand_env_vars("$ELIN_EXPAND_TEST/bin"), "ok-value/bin");
        assert_eq!(expand_env_vars("${ELIN_EXPAND_TEST}/bin"), "ok-value/bin");
        std::env::remove_var("ELIN_EXPAND_TEST");
    }

    #[cfg(windows)]
    #[test]
    fn path_contains_ignores_slash_and_case() {
        let var = r"C:\Program Files\Git\cmd;C:\Windows";
        assert!(path_contains_dir(var, Path::new(r"C:\Program Files\Git\cmd\")));
        assert!(!path_contains_dir(var, Path::new(r"C:\Program Files\Git\bin")));
    }

    #[cfg(not(windows))]
    #[test]
    fn path_contains_unix_colon() {
        let var = "/usr/bin:/opt/homebrew/bin";
        assert!(path_contains_dir(var, Path::new("/opt/homebrew/bin")));
        assert!(!path_contains_dir(var, Path::new("/opt/homebrew")));
    }

    #[test]
    fn elixir_install_marker_does_not_match_elin_app_dir() {
        assert!(is_managed_path(r"C:\Users\me\.elixir-install\installs\otp\27.0\bin"));
        assert!(is_managed_path("/Users/me/.elixir-install/installs/otp/27.0/bin"));
        assert!(!is_managed_path(r"C:\Users\me\AppData\Local\Elin"));
        assert!(!is_managed_path(r"D:\elin\src-tauri\target\debug"));
        assert!(!is_managed_path("/Applications/Elin.app/Contents/MacOS"));
    }

    #[cfg(not(windows))]
    #[test]
    fn parses_env_sh_export() {
        let text = "export PATH=\"/opt/otp/bin:/opt/elixir/bin:$PATH\"\n";
        assert_eq!(
            parse_unix_env_path(text).as_deref(),
            Some("/opt/otp/bin:/opt/elixir/bin")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn replace_block_rewrites_managed_section() {
        let existing = "export FOO=1\n# >>> elin PATH >>>\nold\n# <<< elin PATH <<<\nexport BAR=2\n";
        let next = replace_block(existing, "# >>> elin PATH >>>\nnew\n# <<< elin PATH <<<\n");
        assert!(next.contains("new"));
        assert!(!next.contains("old"));
        assert!(next.contains("export FOO=1"));
        assert!(next.contains("export BAR=2"));
    }
}
