//! Run `.bat` toolchains on Windows without quote-hell, and decode OEM console text.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Do not flash a terminal when a GUI Elin starts `cmd`, Mix, git, or PowerShell.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Keep a spawned editor alive if Elin later dies.
pub const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

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

/// PATH with this pair first, and other Elixir/OTP installs stripped so they cannot shadow it.
pub fn isolated_path(otp_bin: &Path, elixir_bin: &Path) -> String {
    let rest = std::env::var("PATH").unwrap_or_default();
    let filtered: Vec<&str> = rest
        .split(';')
        .filter(|p| {
            let l = p.to_lowercase().replace('/', "\\");
            !l.contains("\\elixir")
                && !l.contains("\\erlang")
                && !l.contains("\\erl-")
                && !l.contains("\\.elixir-install")
        })
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
    parts.join(";")
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

/// `cmd /C <bat> <args>` as separate argv so paths with spaces are not double-quoted.
/// `.bat` cannot be CreateProcess'd directly on Windows.
pub fn run_bat(bat: &Path, args: &[&str], path: &str, erlang_home: Option<&Path>) -> std::io::Result<Output> {
    let mut cmd = Command::new("cmd.exe");
    cmd.arg("/D")
        .arg("/C")
        .arg(bat)
        .args(args)
        .env("PATH", path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(home) = erlang_home {
        cmd.env("ERLANG_HOME", home);
    }
    hide_console(&mut cmd);
    cmd.output()
}

/// Spawn a `.bat` with stdout/stderr piped. Same quoting rules as [`run_bat`].
pub fn spawn_bat(
    bat: &Path,
    args: &[&str],
    path: &str,
    erlang_home: Option<&Path>,
    cwd: Option<&Path>,
) -> std::io::Result<std::process::Child> {
    let mut cmd = Command::new("cmd.exe");
    cmd.arg("/D")
        .arg("/C")
        .arg(bat)
        .args(args)
        .env("PATH", path)
        .env("TERM", "dumb")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(home) = erlang_home {
        cmd.env("ERLANG_HOME", home);
    }
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    hide_console(&mut cmd);
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
