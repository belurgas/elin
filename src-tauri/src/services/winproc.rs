//! Run Mix / Elixir without quote-hell, and decode console text.
//!
//! On Windows the toolchain is `.bat` files that must go through `cmd /C`.
//! On Unix they are ordinary scripts with a shebang.

use crate::services::host::{join_path, path_key, split_path};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Do not flash a terminal when a GUI Elin starts `cmd`, Mix, git, or PowerShell.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Keep a spawned editor alive if Elin later dies.
pub const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
/// Detach so a child (the updater) survives `app.exit`.
pub const DETACHED_PROCESS: u32 = 0x0000_0008;

pub fn hide_console(cmd: &mut Command) {
    hide_console_ex(cmd, 0);
}

pub fn hide_console_ex(cmd: &mut Command, extra: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW | extra);
    }
    #[cfg(not(windows))]
    {
        let _ = extra;
        let _ = cmd;
    }
}

pub fn is_shell_script(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("bat") || e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("ps1"))
        .unwrap_or(false)
}

fn tool_command(script: &Path) -> Command {
    #[cfg(windows)]
    {
        if is_shell_script(script) {
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/D").arg("/C").arg(script);
            return cmd;
        }
    }
    Command::new(script)
}

/// Decode process output. Windows mix/elixir often print OEM (CP866), not UTF-8.
pub fn decode_console(bytes: &[u8]) -> String {
    decode_bytes(bytes).trim().to_string()
}

/// Same decode, keeping leading indent so Mix logs still nest in the console.
pub fn decode_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if let Ok(utf) = std::str::from_utf8(bytes) {
        return utf.to_string();
    }
    #[cfg(windows)]
    {
        oem_to_string(bytes)
    }
    #[cfg(not(windows))]
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(windows)]
fn oem_to_string(bytes: &[u8]) -> String {
    const CP_OEMCP: u32 = 1;
    const CP_ACP: u32 = 0;
    decode_codepage(CP_OEMCP, bytes)
        .or_else(|| decode_codepage(CP_ACP, bytes))
        .unwrap_or_else(|| String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(windows)]
fn decode_codepage(cp: u32, bytes: &[u8]) -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            cp: u32,
            flags: u32,
            src: *const u8,
            cb: i32,
            dst: *mut u16,
            cc: i32,
        ) -> i32;
    }
    unsafe {
        let n = MultiByteToWideChar(cp, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0);
        if n <= 0 {
            return None;
        }
        let mut buf = vec![0u16; n as usize];
        let n = MultiByteToWideChar(cp, 0, bytes.as_ptr(), bytes.len() as i32, buf.as_mut_ptr(), n);
        if n <= 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..n as usize]))
    }
}

fn looks_like_elixir_install(entry: &str) -> bool {
    let l = path_key(entry);
    l.contains("/elixir")
        || l.contains("/erlang")
        || l.contains("/erl-")
        || l.contains("/.elixir-install")
}

/// PATH with this pair first, and other Elixir/OTP installs stripped so they cannot shadow it.
pub fn isolated_path(otp_bin: &Path, elixir_bin: &Path) -> String {
    let rest = std::env::var("PATH").unwrap_or_default();
    let filtered: Vec<&str> = split_path(&rest)
        .into_iter()
        .filter(|p| !looks_like_elixir_install(p))
        .collect();
    let mut parts = Vec::new();
    parts.push(otp_bin.to_string_lossy().into_owned());
    if let Some(home) = erlang_home(otp_bin) {
        let home_bin = home.join("bin");
        if home_bin != otp_bin && home_bin.exists() {
            parts.push(home_bin.to_string_lossy().into_owned());
        }
    }
    parts.push(elixir_bin.to_string_lossy().into_owned());
    parts.extend(filtered.into_iter().map(str::to_string));
    join_path(&parts)
}

pub fn erlang_home(otp_bin: &Path) -> Option<PathBuf> {
    let parent = otp_bin.parent()?;
    let name = parent.file_name()?.to_string_lossy();
    if name.to_lowercase().starts_with("erts") {
        parent.parent().map(|p| p.to_path_buf())
    } else {
        Some(parent.to_path_buf())
    }
}

fn apply_toolchain_env(cmd: &mut Command, path: &str, erlang_home: Option<&Path>) {
    cmd.env("PATH", path)
        .env("TERM", "dumb")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(home) = erlang_home {
        cmd.env("ERLANG_HOME", home);
    }
    hide_console(cmd);
}

/// Run a toolchain script (`mix.bat` / `mix`, `elixir.bat` / `elixir`).
pub fn run_bat(bat: &Path, args: &[&str], path: &str, erlang_home: Option<&Path>) -> std::io::Result<Output> {
    let mut cmd = tool_command(bat);
    cmd.args(args);
    apply_toolchain_env(&mut cmd, path, erlang_home);
    cmd.output()
}

/// Spawn a toolchain script with stdout/stderr piped. Same quoting rules as [`run_bat`].
pub fn spawn_bat(
    bat: &Path,
    args: &[&str],
    path: &str,
    erlang_home: Option<&Path>,
    cwd: Option<&Path>,
) -> std::io::Result<std::process::Child> {
    let mut cmd = tool_command(bat);
    cmd.args(args);
    apply_toolchain_env(&mut cmd, path, erlang_home);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    cmd.spawn()
}

pub fn output_text(output: &Output) -> String {
    let mut text = decode_console(&output.stdout);
    let err = decode_console(&output.stderr);
    if !err.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&err);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_extensions_are_scripts() {
        assert!(is_shell_script(Path::new("mix.bat")));
        assert!(is_shell_script(Path::new("code.cmd")));
        assert!(!is_shell_script(Path::new("mix")));
        assert!(!is_shell_script(Path::new("erl.exe")));
    }
}
