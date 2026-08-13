//! User PATH surgery on Windows, plus the shared toolchain root.
//!
//! Elin writes to the current-user Environment key (no admin required) and
//! broadcasts `WM_SETTINGCHANGE` so new terminals pick up the change.

use crate::error::{AppError, AppResult};
use std::path::PathBuf;

/// Official elixir-install layout, so Elin and `install.bat` can share installs.
pub fn managed_root() -> AppResult<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| AppError::msg("Could not resolve the user home directory"))?;
    Ok(home.join(".elixir-install").join("installs"))
}

/// Marker used to recognize PATH entries Elin previously wrote for toolchains.
pub fn is_managed_path(entry: &str) -> bool {
    let lower = entry.replace('/', "\\").to_lowercase();
    lower.contains("\\.elixir-install\\installs\\")
}

/// Folder that contains `elin.exe` (install dir, or target/debug while developing).
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

/// Prepend OTP + Elixir bin folders and drop stale Elin entries.
pub fn set_user_path_entries(bins: &[PathBuf]) -> AppResult<()> {
    #[cfg(windows)]
    {
        windows_set_path(bins)
    }
    #[cfg(not(windows))]
    {
        let _ = bins;
        Err(AppError::msg(
            "PATH editing is implemented for Windows in this build.",
        ))
    }
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
    a.replace('/', "\\").eq_ignore_ascii_case(&b.replace('/', "\\"))
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

/// Read the current user PATH (Windows) or process PATH (elsewhere).
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

/// PATH a brand-new `cmd.exe` actually gets: expanded machine + user.
pub fn console_path() -> String {
    let machine = expand_env_vars(&machine_path());
    let user = expand_env_vars(&user_path());
    if machine.is_empty() {
        user
    } else if user.is_empty() {
        machine
    } else {
        format!("{machine};{user}")
    }
}

/// Expand `%VAR%` sequences the way a new console does.
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
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

pub fn path_contains_dir(path_var: &str, dir: &std::path::Path) -> bool {
    let needle = normalize_dir(dir);
    if needle.is_empty() {
        return false;
    }
    expand_env_vars(path_var).split(';').any(|entry| {
        let trimmed = entry.trim().trim_matches('"');
        !trimmed.is_empty() && normalize_dir(std::path::Path::new(trimmed)) == needle
    })
}

fn normalize_dir(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_lowercase()
}

/// Prepend folders to the user PATH without dropping existing Elin entries.
/// Skips folders already present on user or machine PATH.
pub fn prepend_user_path_dirs(dirs: &[PathBuf]) -> AppResult<Vec<PathBuf>> {
    #[cfg(windows)]
    {
        windows_prepend_path(dirs)
    }
    #[cfg(not(windows))]
    {
        let _ = dirs;
        Err(AppError::msg(
            "PATH editing is implemented for Windows in this build.",
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn expands_percent_vars() {
        let expanded = expand_env_vars(r"%SystemRoot%\system32");
        assert!(!expanded.contains('%'), "{expanded}");
        assert!(expanded.to_lowercase().contains("system32"));
    }

    #[test]
    fn path_contains_ignores_slash_and_case() {
        let var = r"C:\Program Files\Git\cmd;C:\Windows";
        assert!(path_contains_dir(var, Path::new(r"C:\Program Files\Git\cmd\")));
        assert!(!path_contains_dir(var, Path::new(r"C:\Program Files\Git\bin")));
    }

    #[test]
    fn elixir_install_marker_does_not_match_elin_app_dir() {
        assert!(is_managed_path(r"C:\Users\me\.elixir-install\installs\otp\27.0\bin"));
        assert!(!is_managed_path(r"C:\Users\me\AppData\Local\Elin"));
        assert!(!is_managed_path(r"D:\elin\src-tauri\target\debug"));
    }
}
