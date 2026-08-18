//! Environment Doctor: grouped checks, exact paths, and one-click fixes.

use crate::error::AppResult;
use crate::services::env::{add_elin_to_path, elin_install_dir, elin_on_user_path, is_managed_path, set_user_path_entries, user_path};
use crate::services::install::list_installed;
use crate::services::probe::{discovered_bins, probe_machine, StartupProbe};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    pub id: String,
    pub group: String,
    pub title: String,
    pub ok: bool,
    pub detail: String,
    pub hint: String,
    pub severity: String,
    pub path: Option<String>,
    pub fix_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub score: u8,
    pub checks: Vec<DoctorCheck>,
    pub elixir_version: Option<String>,
    pub erlang_version: Option<String>,
    pub probe: StartupProbe,
}

/// Run every check. Safe to call often; each probe is a short process or file test.
pub fn run_doctor() -> AppResult<DoctorReport> {
    let probe = probe_machine()?;
    let mut checks = Vec::new();

    checks.push(check(
        "elixir-present",
        "runtime",
        "Elixir is installed",
        probe.elixir.is_some(),
        match &probe.elixir {
            Some(h) => format!("{} · {}", h.path, h.source),
            None => "No Elixir binary on PATH, Homebrew, or ~/.elixir-install.".into(),
        },
        "Use Install to fetch a compatible Elixir + OTP pair.",
        "error",
        probe.elixir.as_ref().map(|h| h.path.clone()),
        None,
    ));

    checks.push(check(
        "elixir-on-path",
        "path",
        "Elixir is callable from a new console",
        probe.elixir.as_ref().map(|h| h.callable).unwrap_or(false),
        if probe.elixir.as_ref().map(|h| h.callable).unwrap_or(false) {
            probe.elixir.as_ref().map(|h| h.why.clone()).unwrap_or_default()
        } else if probe.elixir.is_some() {
            "Installed, but a new console cannot run `elixir`.".into()
        } else {
            "Nothing to add yet.".into()
        },
        "Click Fix to prepend the discovered bin folders. Then open a new terminal.",
        "error",
        probe.elixir.as_ref().map(|h| h.path.clone()),
        if probe.elixir.as_ref().map(|h| h.needs_path_fix).unwrap_or(false) {
            Some("add-path".into())
        } else {
            None
        },
    ));

    checks.push(check(
        "otp-present",
        "runtime",
        "Erlang/OTP is installed",
        probe.erlang.is_some(),
        match &probe.erlang {
            Some(h) => format!("{} · {}", h.version.clone().unwrap_or_default(), h.path),
            None => "Elixir needs `erl` next to it.".into(),
        },
        "Install a matching OTP from the Install page.",
        "error",
        probe.erlang.as_ref().map(|h| h.path.clone()),
        None,
    ));

    checks.push(check(
        "mix",
        "runtime",
        "Mix is available",
        probe.mix.is_some(),
        match &probe.mix {
            Some(h) => h.path.clone(),
            None => "Mix ships with Elixir in the same bin folder.".into(),
        },
        "Reinstall Elixir if Mix is missing.",
        "error",
        probe.mix.as_ref().map(|h| h.path.clone()),
        None,
    ));

    let hex_ok = hex_archive_present();
    checks.push(check(
        "hex",
        "tooling",
        "Hex package manager",
        hex_ok,
        if hex_ok {
            "Hex archive found under ~/.mix/archives.".into()
        } else if probe.mix.is_none() {
            "Mix is missing, so Hex cannot be installed yet.".into()
        } else {
            "No Hex archive under ~/.mix/archives yet.".into()
        },
        "Fix runs `mix local.hex --force` and `mix local.rebar --force` using the Mix next to Elixir.",
        "warn",
        probe.mix.as_ref().map(|h| h.path.clone()),
        if probe.mix.is_some() && !hex_ok {
            Some("install-hex".into())
        } else {
            None
        },
    ));

    let git_ok = probe.git.as_ref().map(|h| h.callable).unwrap_or(false);
    checks.push(check(
        "git",
        "tooling",
        "Git is callable from a new console",
        git_ok,
        match &probe.git {
            Some(h) => format!("{} · {}", h.path, h.why),
            None => "Mix dependencies are Git/Hex checkouts. Git is required.".into(),
        },
        if probe.git.as_ref().map(|h| h.needs_path_fix).unwrap_or(false) {
            "Fix adds Git's folder to the user PATH."
        } else if cfg!(windows) {
            "Install Git for Windows from https://git-scm.com/download/win"
        } else if cfg!(target_os = "macos") {
            "Install Git with Xcode CLT (`xcode-select --install`) or Homebrew."
        } else {
            "Install Git with your package manager (`sudo apt install git`)."
        },
        "warn",
        probe.git.as_ref().map(|h| h.path.clone()),
        if probe.git.as_ref().map(|h| h.needs_path_fix).unwrap_or(false) {
            Some("add-path-git".into())
        } else if probe.git.is_none() {
            Some("open-git".into())
        } else {
            None
        },
    ));

    #[cfg(windows)]
    {
        let vcruntime = PathBuf::from(r"C:\Windows\System32\vcruntime140.dll").exists();
        checks.push(check(
            "vcredist",
            "system",
            "Visual C++ Redistributable",
            vcruntime,
            if vcruntime {
                "vcruntime140.dll is present.".into()
            } else {
                "The Erlang VM on Windows needs the VC++ runtime.".into()
            },
            "Install the latest x64 VC++ Redistributable from Microsoft.",
            "warn",
            None,
            None,
        ));
    }

    let managed = list_installed().unwrap_or_default();
    checks.push(check(
        "managed-install",
        "runtime",
        "Elin-managed toolchain",
        !managed.is_empty(),
        if managed.is_empty() {
            "No versions under ~/.elixir-install/installs yet. A system install is fine too.".into()
        } else {
            format!("{} Elixir build(s) managed by Elin.", managed.len())
        },
        "The Install page keeps versions side by side.",
        "info",
        None,
        None,
    ));

    let path = user_path();
    let has_managed = crate::services::host::split_path(&path).iter().any(|e| is_managed_path(e));
    checks.push(check(
        "path-wired",
        "path",
        "User PATH includes a toolchain",
        has_managed || probe.elixir.as_ref().map(|h| h.callable).unwrap_or(false),
        if has_managed {
            "User PATH points at ~/.elixir-install/installs.".into()
        } else if probe.user_path_has_elixir {
            "Elixir is on PATH from another installer (Program Files, Scoop, Chocolatey).".into()
        } else {
            "PATH does not include Elixir yet.".into()
        },
        "Fix prepends the discovered bin folders without needing admin rights.",
        "warn",
        None,
        if !has_managed && probe.elixir.as_ref().map(|h| h.needs_path_fix).unwrap_or(false) {
            Some("add-path".into())
        } else {
            None
        },
    ));

    let elixir_version = probe.elixir.as_ref().and_then(|h| h.version.clone());
    let erlang_version = probe.erlang.as_ref().and_then(|h| h.version.clone());
    let can_start = probe.elixir.is_some() && probe.erlang.is_some();
    checks.push(check(
        "pair",
        "runtime",
        "Elixir and OTP are both on disk",
        can_start,
        match (&probe.elixir, &probe.erlang) {
            (Some(ex), Some(erl)) => format!(
                "{} · {}",
                elixir_version.clone().unwrap_or_else(|| ex.path.clone()),
                erlang_version.clone().unwrap_or_else(|| erl.path.clone())
            ),
            (Some(_), None) => "Elixir is present but `erl` was not found. The VM will not start.".into(),
            _ => "Install a matching Elixir + OTP pair.".into(),
        },
        "If OTP is missing, Elixir prints a cryptic error. Install the recommended pair.",
        "error",
        None,
        None,
    ));

    let elin_ok = elin_on_user_path();
    let elin_dir = elin_install_dir().map(|p| p.to_string_lossy().into_owned());
    checks.push(check(
        "elin-cli",
        "path",
        "`elin` is on the user PATH",
        elin_ok,
        match &elin_dir {
            Some(dir) if elin_ok => format!("New consoles can run `elin` from {dir}."),
            Some(dir) => format!("Elin lives in {dir}, but that folder is not on the user PATH."),
            None => "Could not resolve Elin's install folder.".into(),
        },
        "Fix prepends Elin's folder (not ~/.elixir-install) so `elin add` works in a new terminal.",
        "warn",
        elin_dir,
        if elin_ok { None } else { Some("add-path-elin".into()) },
    ));

    let passed = checks.iter().filter(|c| c.ok).count();
    let score = ((passed as f32 / checks.len() as f32) * 100.0).round() as u8;

    Ok(DoctorReport {
        score,
        checks,
        elixir_version,
        erlang_version,
        probe,
    })
}

fn check(
    id: &str,
    group: &str,
    title: &str,
    ok: bool,
    detail: String,
    hint: &str,
    severity: &str,
    path: Option<String>,
    fix_id: Option<String>,
) -> DoctorCheck {
    DoctorCheck {
        id: id.into(),
        group: group.into(),
        title: title.into(),
        ok,
        detail,
        hint: hint.into(),
        severity: severity.into(),
        path,
        fix_id,
    }
}

fn hex_archive_present() -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let archives = home.join(".mix").join("archives");
    let Ok(entries) = std::fs::read_dir(&archives) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.file_name()
            .to_string_lossy()
            .to_lowercase()
            .starts_with("hex")
    })
}

/// Apply a Doctor fix by id.
pub fn apply_fix(fix_id: &str) -> AppResult<String> {
    match fix_id {
        "add-path" => {
            let probe = probe_machine()?;
            let bins = discovered_bins(&probe);
            if bins.is_empty() {
                return Err(crate::error::AppError::msg(
                    "Could not find bin folders to add.",
                ));
            }
            set_user_path_entries(&bins)?;
            Ok(format!(
                "Added {} folder(s) to the user PATH. Open a new terminal.",
                bins.len()
            ))
        }
        "add-path-git" => crate::services::probe::add_hit_to_path("git"),
        "install-hex" => {
            let probe = probe_machine()?;
            let mix = probe.mix.as_ref().ok_or_else(|| {
                crate::error::AppError::msg("Mix was not found. Install Elixir first.")
            })?;
            let mix_path = PathBuf::from(&mix.path);
            let elixir_bin = mix_path.parent().ok_or_else(|| {
                crate::error::AppError::msg("Could not resolve Mix's bin folder.")
            })?;
            let otp_bin = probe
                .erlang
                .as_ref()
                .and_then(|h| PathBuf::from(&h.path).parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| elixir_bin.to_path_buf());
            let path = crate::services::winproc::isolated_path(&otp_bin, elixir_bin);
            let home = crate::services::winproc::erlang_home(&otp_bin);
            let mix_script = if mix_path.exists() {
                mix_path.clone()
            } else {
                crate::services::host::mix_cmd(elixir_bin)
            };
            let hex = crate::services::winproc::run_bat(
                &mix_script,
                &["local.hex", "--force"],
                &path,
                home.as_deref(),
            )?;
            let rebar = crate::services::winproc::run_bat(
                &mix_script,
                &["local.rebar", "--force"],
                &path,
                home.as_deref(),
            )?;
            if hex.status.success() && rebar.status.success() {
                Ok("Hex and Rebar are installed.".into())
            } else {
                let text = crate::services::winproc::output_text(&hex);
                Err(crate::error::AppError::msg(if text.trim().is_empty() {
                    crate::services::winproc::output_text(&rebar)
                } else {
                    text
                }))
            }
        }
        "add-path-elin" => add_elin_to_path(),
        "open-git" => {
            let url = if cfg!(windows) {
                "https://git-scm.com/download/win"
            } else if cfg!(target_os = "macos") {
                "https://git-scm.com/download/mac"
            } else {
                "https://git-scm.com/download/linux"
            };
            #[cfg(windows)]
            {
                let mut cmd = Command::new("cmd");
                cmd.args(["/C", "start", "", url]);
                crate::services::winproc::hide_console(&mut cmd);
                let _ = cmd.spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = Command::new("open").arg(url).spawn();
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let _ = Command::new("xdg-open").arg(url).spawn();
            }
            Ok("Opened the Git download page.".into())
        }
        other => Err(crate::error::AppError::msg(format!("Unknown fix: {other}"))),
    }
}
