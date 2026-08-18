//! Run Mix against a project with the pinned toolchain and a hard timeout.

use crate::error::{AppError, AppResult};
use crate::services::install::list_installed;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub fn mix_in_project(project_path: &Path, args: &[&str], timeout: Duration) -> AppResult<String> {
    mix_with_lines(project_path, args, timeout, |_| {})
}

/// Same as [`mix_in_project`], calling `on_line` as Mix prints (so the studio
/// console can open immediately instead of waiting for the whole run).
pub fn mix_with_lines(
    project_path: &Path,
    args: &[&str],
    timeout: Duration,
    on_line: impl FnMut(&str),
) -> AppResult<String> {
    let (otp_bin, elixir_bin) = crate::services::projects::bins_for_project(&project_path.to_string_lossy())
        .or_else(active_bins)
        .ok_or_else(|| AppError::msg("Install Elixir first — Mix needs a toolchain."))?;
    let mix = crate::services::host::mix_cmd(&elixir_bin);
    if !mix.exists() {
        return Err(AppError::msg("mix was not found next to Elixir."));
    }
    let path = crate::services::winproc::isolated_path(&otp_bin, &elixir_bin);
    let home = crate::services::winproc::erlang_home(&otp_bin);
    let child = crate::services::winproc::spawn_bat(
        &mix,
        args,
        &path,
        home.as_deref(),
        Some(project_path),
    )?;
    wait_lines(
        child,
        timeout,
        format!(
            "mix {} ran longer than {}s and was stopped.",
            args.join(" "),
            timeout.as_secs()
        ),
        format!("mix {} failed", args.join(" ")),
        on_line,
    )
}

pub(crate) fn wait_lines(
    mut child: std::process::Child,
    timeout: Duration,
    timeout_msg: String,
    fail_msg: String,
    mut on_line: impl FnMut(&str),
) -> AppResult<String> {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel::<String>();
    if let Some(out) = stdout {
        let tx = tx.clone();
        std::thread::spawn(move || pump_lines(out, tx));
    }
    if let Some(err) = stderr {
        std::thread::spawn(move || pump_lines(err, tx));
    } else {
        drop(tx);
    }

    let started = Instant::now();
    let mut collected = String::new();
    let mut recent: std::collections::VecDeque<String> = std::collections::VecDeque::with_capacity(12);
    let mut child_done = false;
    let mut status_ok = true;
    const MAX_OUT: usize = 1_048_576;
    loop {
        match rx.recv_timeout(Duration::from_millis(40)) {
            Ok(line) => {
                if recent.iter().any(|x| x == &line) {
                    continue;
                }
                recent.push_back(line.clone());
                if recent.len() > 12 {
                    recent.pop_front();
                }
                if !collected.is_empty() {
                    collected.push('\n');
                }
                collected.push_str(&line);
                on_line(&line);
                if collected.len() > MAX_OUT {
                    let _ = child.kill();
                    collected.push_str("\n… output truncated at 1 MB.");
                    return Ok(collected);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if child_done {
                    break;
                }
                // Pipes closed while the process is still alive — do not busy-spin.
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        if !child_done {
            match child.try_wait()? {
                Some(status) => {
                    child_done = true;
                    status_ok = status.success();
                }
                None if started.elapsed() > timeout => {
                    let _ = child.kill();
                    return Err(AppError::msg(timeout_msg));
                }
                None => {}
            }
        }
    }

    if status_ok {
        Ok(collected)
    } else {
        Err(AppError::msg(if collected.trim().is_empty() {
            fail_msg
        } else {
            collected
        }))
    }
}

fn pump_lines(mut reader: impl Read, tx: mpsc::Sender<String>) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        match reader.read(&mut tmp) {
            Ok(0) => {
                if !buf.is_empty() {
                    let _ = tx.send(line_from(&buf));
                }
                break;
            }
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                while let Some(i) = buf.iter().position(|&b| b == b'\n') {
                    let mut line: Vec<u8> = buf.drain(..=i).collect();
                    if line.last() == Some(&b'\n') {
                        line.pop();
                    }
                    if line.last() == Some(&b'\r') {
                        line.pop();
                    }
                    let _ = tx.send(line_from(&line));
                }
            }
            Err(_) => break,
        }
    }
}

fn line_from(bytes: &[u8]) -> String {
    crate::services::winproc::decode_bytes(bytes)
        .trim_end()
        .to_string()
}

fn looks_long_running(command: &str) -> bool {
    command.contains("phx.server")
        || command.contains("phoenix.server")
        || command.contains("--no-halt")
        || command.split_whitespace().any(|w| w == "iex")
}

fn active_bins() -> Option<(PathBuf, PathBuf)> {
    let installed = list_installed().ok()?;
    let active = installed.iter().find(|p| p.is_active).or_else(|| installed.first())?;
    if active.otp_path.is_empty() || active.elixir_path.is_empty() {
        return None;
    }
    Some((PathBuf::from(&active.otp_path), PathBuf::from(&active.elixir_path)))
}

/// Run a typed studio command in the project (mix/git/elixir) with the toolchain PATH.
pub fn shell_in_project(
    project_path: &Path,
    command: &str,
    timeout: Duration,
    on_line: impl FnMut(&str),
) -> AppResult<String> {
    let command = command.trim();
    if command.is_empty() {
        return Err(AppError::msg("Type a command first."));
    }
    if command.len() > 500 {
        return Err(AppError::msg("Command is too long."));
    }
    let lower = command.to_ascii_lowercase();
    let timeout = if looks_long_running(&lower) {
        Duration::from_secs(14_400)
    } else {
        timeout
    };
    if lower.starts_with("mix ") || lower == "mix" {
        let rest = command[3..].trim();
        let args: Vec<&str> = if rest.is_empty() {
            vec![]
        } else {
            rest.split_whitespace().collect()
        };
        return mix_with_lines(project_path, &args, timeout, on_line);
    }
    let (otp_bin, elixir_bin) = crate::services::projects::bins_for_project(&project_path.to_string_lossy())
        .or_else(active_bins)
        .ok_or_else(|| AppError::msg("Install Elixir first — the shell needs a toolchain."))?;
    let mut path = crate::services::winproc::isolated_path(&otp_bin, &elixir_bin);
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            path = format!(
                "{}{}{path}",
                dir.display(),
                crate::services::host::path_sep()
            );
        }
    }
    let home = crate::services::winproc::erlang_home(&otp_bin);
    let mut cmd = shell_command(command);
    cmd.current_dir(project_path)
        .env("PATH", &path)
        .env("TERM", "dumb")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(home) = home {
        cmd.env("ERLANG_HOME", home);
    }
    crate::services::winproc::hide_console(&mut cmd);
    let child = cmd.spawn()?;
    wait_lines(
        child,
        timeout,
        format!("`{command}` ran longer than {}s and was stopped.", timeout.as_secs()),
        format!("`{command}` failed"),
        on_line,
    )
}

fn shell_command(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("cmd.exe");
        cmd.arg("/D").arg("/S").arg("/C").arg(command);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn consecutive_duplicate_lines_are_dropped() {
        let mut last = String::new();
        let mut kept = Vec::new();
        for raw in ["Compiling 2 files (.ex)", "Compiling 2 files (.ex)", "Generated hello_elin app"] {
            if raw == last {
                continue;
            }
            last = raw.to_string();
            kept.push(raw);
        }
        assert_eq!(kept, ["Compiling 2 files (.ex)", "Generated hello_elin app"]);
    }
}
