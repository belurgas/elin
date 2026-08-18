//! Startup probe: find Elixir/OTP/Mix even when this process inherited a stale PATH.

use crate::error::AppResult;
use crate::services::env::{
    console_path, machine_path, path_contains_dir, prepend_user_path_dirs, user_path,
};
use crate::services::install::list_installed;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A binary Elin discovered on disk, with PATH membership and console reachability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryHit {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub on_process_path: bool,
    pub on_user_path: bool,
    pub on_machine_path: bool,
    pub callable: bool,
    pub needs_path_fix: bool,
    pub source: String,
    pub why: String,
}

/// Snapshot taken at launch (and whenever Doctor runs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupProbe {
    pub elixir: Option<BinaryHit>,
    pub erlang: Option<BinaryHit>,
    pub mix: Option<BinaryHit>,
    pub git: Option<BinaryHit>,
    pub managed_count: usize,
    pub user_path_has_elixir: bool,
    pub process_path_has_elixir: bool,
    pub notes: Vec<String>,
}

/// Locate toolchain binaries in well-known Windows locations + PATH.
pub fn probe_machine() -> AppResult<StartupProbe> {
    let elixir = find_elixir();
    let erlang = find_erlang();
    let mix = find_mix(elixir.as_ref());
    let git = find_git();

    let mut notes = Vec::new();
    match &elixir {
        Some(hit) if hit.needs_path_fix => notes.push(format!(
            "Elixir is installed at {} but a new console cannot run `elixir`. {}",
            hit.path, hit.why
        )),
        Some(hit) if !hit.on_process_path && hit.callable => notes.push(
            "Elixir is on PATH for new consoles, but this Elin process started with an older environment.".into(),
        ),
        None => notes.push("No Elixir install was found. Use the Install page.".into()),
        Some(hit) => notes.push(format!("Elixir ready at {}.", hit.path)),
    }
    if elixir.is_some() && erlang.is_none() {
        notes.push("Elixir without Erlang/OTP usually fails with a cryptic error. Install a matching OTP.".into());
    }

    let managed_count = list_installed().map(|v| v.len()).unwrap_or(0);

    Ok(StartupProbe {
        user_path_has_elixir: elixir.as_ref().map(|h| h.on_user_path).unwrap_or(false),
        process_path_has_elixir: elixir.as_ref().map(|h| h.on_process_path).unwrap_or(false),
        elixir,
        erlang,
        mix,
        git,
        managed_count,
        notes,
    })
}

fn find_elixir() -> Option<BinaryHit> {
    let mut candidates: Vec<(PathBuf, &'static str)> = Vec::new();

    if let Some(p) = which_any(&["elixir", "elixir.bat", "elixir.cmd", "elixir.exe"]) {
        candidates.push((p, "PATH"));
    }
    #[cfg(windows)]
    for extra in [
        PathBuf::from(r"C:\Program Files\Elixir\bin\elixir.bat"),
        PathBuf::from(r"C:\Program Files (x86)\Elixir\bin\elixir.bat"),
    ] {
        if extra.exists() {
            candidates.push((extra, "Program Files"));
        }
    }
    if let Some(home) = dirs::home_dir() {
        let root = home.join(".elixir-install").join("installs").join("elixir");
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let bin = entry.path().join("bin");
                if let Some(script) = crate::services::host::find_script(&bin, "elixir") {
                    candidates.push((script, "elixir-install"));
                }
            }
        }
        #[cfg(windows)]
        {
            let scoop = home.join("scoop").join("apps").join("elixir").join("current").join("bin").join("elixir.bat");
            if scoop.exists() {
                candidates.push((scoop, "Scoop"));
            }
        }
        for extra in [
            home.join(".asdf").join("shims").join("elixir"),
            home.join(".local").join("share").join("mise").join("shims").join("elixir"),
            PathBuf::from("/opt/homebrew/bin/elixir"),
            PathBuf::from("/usr/local/bin/elixir"),
        ] {
            if extra.exists() {
                candidates.push((extra, "system"));
            }
        }
    }

    let (path, source) = candidates.into_iter().next()?;
    Some(describe("elixir", path, source, None))
}

fn find_erlang() -> Option<BinaryHit> {
    let mut candidates: Vec<(PathBuf, &'static str)> = Vec::new();
    if let Some(p) = which_any(&["erl", "erl.exe"]) {
        candidates.push((p, "PATH"));
    }
    #[cfg(windows)]
    {
        for extra in [
            PathBuf::from(r"C:\Program Files\Erlang OTP\bin\erl.exe"),
            PathBuf::from(r"C:\Program Files\erl-27.0\bin\erl.exe"),
            PathBuf::from(r"C:\Program Files\erl-26.2\bin\erl.exe"),
        ] {
            if extra.exists() {
                candidates.push((extra, "Program Files"));
            }
        }
        if let Ok(entries) = std::fs::read_dir(r"C:\Program Files") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.contains("erlang") || name.starts_with("erl") {
                    let erl = entry.path().join("bin").join("erl.exe");
                    if erl.exists() {
                        candidates.push((erl, "Program Files"));
                    }
                }
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let root = home.join(".elixir-install").join("installs").join("otp");
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let bin = entry.path().join("bin");
                if let Some(erl) = crate::services::host::find_script(&bin, "erl") {
                    candidates.push((erl, "elixir-install"));
                } else if bin.join("erl.exe").exists() {
                    candidates.push((bin.join("erl.exe"), "elixir-install"));
                } else if bin.join("erl").exists() {
                    candidates.push((bin.join("erl"), "elixir-install"));
                }
            }
        }
        for extra in [
            home.join(".asdf").join("shims").join("erl"),
            PathBuf::from("/opt/homebrew/bin/erl"),
            PathBuf::from("/usr/local/bin/erl"),
        ] {
            if extra.exists() {
                candidates.push((extra, "system"));
            }
        }
    }
    let (path, source) = candidates.into_iter().next()?;
    Some(describe("erlang", path, source, None))
}

fn find_mix(elixir: Option<&BinaryHit>) -> Option<BinaryHit> {
    if let Some(p) = which_any(&["mix", "mix.bat", "mix.cmd", "mix.ps1"]) {
        return Some(describe("mix", p, "PATH", None));
    }
    if let Some(elixir) = elixir {
        let dir = PathBuf::from(&elixir.path);
        let parent = dir.parent().unwrap_or(&dir);
        if let Some(mix) = crate::services::host::find_script(parent, "mix") {
            return Some(describe("mix", mix, &elixir.source, None));
        }
        let mix = dir.with_file_name(crate::services::host::mix_script());
        if mix.exists() {
            return Some(describe("mix", mix, &elixir.source, None));
        }
    }
    None
}

fn find_git() -> Option<BinaryHit> {
    let mut candidates: Vec<(PathBuf, &'static str)> = Vec::new();
    if let Some(p) = which_any(&["git", "git.exe"]) {
        candidates.push((p, "PATH"));
    }
    #[cfg(windows)]
    for extra in [
        PathBuf::from(r"C:\Program Files\Git\cmd\git.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Git\cmd\git.exe"),
        PathBuf::from(r"C:\Program Files\Git\bin\git.exe"),
    ] {
        if extra.exists() {
            candidates.push((extra, "Program Files"));
        }
    }
    #[cfg(not(windows))]
    for extra in [
        PathBuf::from("/opt/homebrew/bin/git"),
        PathBuf::from("/usr/local/bin/git"),
        PathBuf::from("/usr/bin/git"),
    ] {
        if extra.exists() {
            candidates.push((extra, "system"));
        }
    }
    let (path, source) = candidates.into_iter().next()?;
    Some(describe("git", path, source, Some(&["--version"])))
}

fn describe(name: &str, path: PathBuf, source: &str, version_args: Option<&[&str]>) -> BinaryHit {
    let user = user_path();
    let machine = machine_path();
    let process = std::env::var("PATH").unwrap_or_default();
    let console = console_path();
    let on_user_path = path_contains_dir(&user, path.parent().unwrap_or(&path));
    let on_machine_path = path_contains_dir(&machine, path.parent().unwrap_or(&path));
    let on_process_path = path_contains_dir(&process, path.parent().unwrap_or(&path));
    let callable = command_on_console_path(where_name(name), &console);
    let needs_path_fix = path.exists() && !callable;
    let why = explain(name, on_user_path, on_machine_path, on_process_path, callable, &path);
    let version = match version_args {
        Some(args) => probe_out(&path, args),
        None => version_from_path(&path).or_else(|| probe_out(&path, &["--version"])),
    };
    BinaryHit {
        name: name.into(),
        version,
        on_process_path,
        on_user_path,
        on_machine_path,
        callable,
        needs_path_fix,
        source: source.into(),
        path: path.to_string_lossy().into(),
        why,
    }
}

fn version_from_path(path: &Path) -> Option<String> {
    for ancestor in path.ancestors().take(6) {
        let name = ancestor.file_name()?.to_string_lossy();
        if let Some((elixir, _)) = name.rsplit_once("-otp-") {
            if crate::domain::ElixirVersion::parse(elixir).is_some() {
                return Some(elixir.to_string());
            }
        }
        if name.chars().next()?.is_ascii_digit() && crate::domain::OtpVersion::parse(&name).is_some() {
            return Some(format!("OTP {name}"));
        }
    }
    version_from_install_files(path)
}

fn version_from_install_files(bin: &Path) -> Option<String> {
    let root = bin.parent()?.parent()?;
    let app = root.join("lib").join("elixir").join("ebin").join("elixir.app");
    if let Ok(text) = std::fs::read_to_string(app) {
        if let Some(vsn) = vsn_from_appfile(&text) {
            return Some(vsn);
        }
    }
    let releases = root.join("releases");
    if let Ok(entries) = std::fs::read_dir(releases) {
        for entry in entries.flatten() {
            let otp_ver = entry.path().join("OTP_VERSION");
            if let Ok(text) = std::fs::read_to_string(otp_ver) {
                let v = text.trim();
                if !v.is_empty() {
                    return Some(format!("OTP {v}"));
                }
            }
        }
    }
    None
}

fn vsn_from_appfile(text: &str) -> Option<String> {
    let key = "{vsn, \"";
    let start = text.find(key)? + key.len();
    let end = text[start..].find('"')?;
    let vsn = text[start..start + end].trim();
    if vsn.is_empty() {
        None
    } else {
        Some(vsn.to_string())
    }
}

fn explain(
    name: &str,
    on_user: bool,
    on_machine: bool,
    on_process: bool,
    callable: bool,
    path: &Path,
) -> String {
    if callable && on_machine && !on_user {
        format!("`{name}` is on the system PATH. New consoles can run it; it does not need the user PATH.")
    } else if callable && on_user {
        format!("`{name}` is on the user PATH. New consoles can run it.")
    } else if callable {
        format!("`{name}` resolves in a fresh console (PATHEXT / another PATH entry).")
    } else if on_process && !on_user && !on_machine {
        format!(
            "This Elin process can see `{name}`, but neither user nor system PATH includes {}. New terminals will not."
            , path.parent().map(|p| p.display().to_string()).unwrap_or_default()
        )
    } else {
        format!(
            "Found at {}, but `where {name}` in a fresh console fails. The bin folder is missing from both user and system PATH.",
            path.display()
        )
    }
}

fn where_name(name: &str) -> &str {
    match name {
        "erlang" | "otp" => "erl",
        other => other,
    }
}

/// Resolve `command` against the PATH a new console would get — not Elin's inherited PATH.
fn command_on_console_path(command: &str, console_path: &str) -> bool {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("where.exe");
        cmd.arg(command).env("PATH", console_path);
        return run_capped(&mut cmd, Duration::from_millis(800))
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false);
    }
    #[cfg(not(windows))]
    {
        for dir in crate::services::host::split_path(console_path) {
            let candidate = Path::new(dir).join(command);
            if candidate.is_file() {
                return true;
            }
        }
        false
    }
}

/// `which`, then PATHEXT-style names the crate sometimes misses.
pub fn which_any(names: &[&str]) -> Option<PathBuf> {
    for name in names {
        if let Ok(p) = which::which(name) {
            return Some(p);
        }
    }
    None
}

fn probe_out(bin: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args);
    let output = run_capped(&mut cmd, Duration::from_millis(1500))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.lines().last().unwrap_or(trimmed).to_string())
    }
}

fn run_capped(cmd: &mut Command, limit: Duration) -> Option<std::process::Output> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    crate::services::winproc::hide_console(cmd);
    let mut child = cmd.spawn().ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) if started.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
}

/// Bins to prepend so an existing Program Files install becomes visible.
pub fn discovered_bins(probe: &StartupProbe) -> Vec<PathBuf> {
    let mut bins = Vec::new();
    if let Some(erl) = &probe.erlang {
        if let Some(parent) = PathBuf::from(&erl.path).parent() {
            bins.push(parent.to_path_buf());
        }
    }
    if let Some(ex) = &probe.elixir {
        if let Some(parent) = PathBuf::from(&ex.path).parent() {
            bins.push(parent.to_path_buf());
        }
    }
    bins
}

/// Add one discovered binary's folder to the user PATH, only if a new console cannot run it.
pub fn add_hit_to_path(name: &str) -> AppResult<String> {
    let probe = probe_machine()?;
    let hit = match name {
        "elixir" => probe.elixir.clone(),
        "erlang" | "otp" => probe.erlang.clone(),
        "mix" => probe.mix.clone(),
        "git" => probe.git.clone(),
        _ => None,
    }
    .ok_or_else(|| crate::error::AppError::msg(format!("No {name} install was found on disk.")))?;

    if hit.callable {
        return Ok(format!(
            "`{name}` already runs in a new console. {}",
            hit.why
        ));
    }
    let dir = PathBuf::from(&hit.path)
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| crate::error::AppError::msg("Could not resolve the bin folder."))?;
    let added = prepend_user_path_dirs(&[dir.clone()])?;
    if added.is_empty() {
        return Ok(format!(
            "The folder {} is already listed on PATH. Open a new terminal.",
            dir.display()
        ));
    }
    Ok(format!(
        "Added {} to the user PATH. Open a new terminal to use `{name}`.",
        dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::env::path_contains_dir;

    #[cfg(windows)]
    #[test]
    fn path_membership_is_parent_based() {
        let path = r"C:\Program Files\Elixir\bin;C:\Windows";
        let bin = PathBuf::from(r"C:\Program Files\Elixir\bin\elixir.bat");
        assert!(path_contains_dir(path, bin.parent().unwrap()));
        assert!(!path_contains_dir(r"C:\Windows", bin.parent().unwrap()));
    }

    #[cfg(windows)]
    #[test]
    fn trailing_slash_still_matches() {
        let path = r"C:\Program Files\Git\cmd\;C:\Windows";
        let dir = PathBuf::from(r"C:\Program Files\Git\cmd");
        assert!(path_contains_dir(path, &dir));
    }

    #[cfg(not(windows))]
    #[test]
    fn path_membership_is_parent_based() {
        let path = "/opt/elixir/bin:/usr/bin";
        let bin = PathBuf::from("/opt/elixir/bin/elixir");
        assert!(path_contains_dir(path, bin.parent().unwrap()));
        assert!(!path_contains_dir("/usr/bin", bin.parent().unwrap()));
    }
}
