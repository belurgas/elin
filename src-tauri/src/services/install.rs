//! Download, extract, and activate Elixir + OTP using the official
//! elixir-install layout (`~/.elixir-install/installs`), so Elin stays
//! compatible with https://elixir-lang.org/install.html.

use crate::domain::{recommended_otp_major, ElixirVersion, InstalledPair, OtpVersion, VersionCatalog};
use crate::error::{AppError, AppResult};
use crate::services::catalog::{elixir_zip_url, fetch_catalog};
use crate::services::env::{managed_root, set_user_path_entries};
use crate::services::host::{self, elixir_cmd, elixir_script, erl_binary, mix_cmd};
use crate::services::net;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Emitter};
use zip::ZipArchive;

/// Progress event payload consumed by the Install Theater UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub stage: String,
    pub message: String,
    pub percent: u8,
}

/// Result returned after a successful (or verified) install.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub pair: InstalledPair,
    pub elixir_version_output: String,
}

fn emit_progress(app: &AppHandle, stage: &str, message: &str, percent: u8) {
    let _ = app.emit(
        "install-progress",
        InstallProgress {
            stage: stage.into(),
            message: message.into(),
            percent,
        },
    );
}

/// Default toolchain root: `%USERPROFILE%\.elixir-install\installs`.
pub fn installs_dir() -> AppResult<PathBuf> {
    Ok(managed_root()?)
}

pub fn otp_dir(otp: &str) -> AppResult<PathBuf> {
    Ok(installs_dir()?.join("otp").join(otp))
}

pub fn elixir_dir(elixir: &str, otp_major: u32) -> AppResult<PathBuf> {
    Ok(installs_dir()?.join("elixir").join(format!("{elixir}-otp-{otp_major}")))
}

/// Install a compatible Elixir + OTP pair, optionally wiring PATH and Hex.
pub async fn install_pair(
    app: AppHandle,
    elixir: String,
    otp: String,
    add_to_path: bool,
    install_hex: bool,
) -> AppResult<InstallResult> {
    let elixir_ver = ElixirVersion::parse(&elixir)
        .ok_or_else(|| AppError::msg(format!("Invalid Elixir version: {elixir}")))?;
    let otp_ver = OtpVersion::parse(&otp)
        .ok_or_else(|| AppError::msg(format!("Invalid OTP version: {otp}")))?;

    if !crate::domain::versions_are_compatible(&elixir_ver, &otp_ver) {
        return Err(AppError::msg(format!(
            "Elixir {elixir} is not documented to run on OTP {otp}. Pick a compatible pair."
        )));
    }

    emit_progress(&app, "resolve", "Checking latest OTP and Elixir assets…", 4);
    let catalog = fetch_catalog(true, false).await?;
    let otp_release = catalog
        .otp
        .iter()
        .find(|r| r.version == otp)
        .cloned()
        .ok_or_else(|| AppError::msg(format!("OTP {otp} is not in the live catalog")))?;

    let otp_url = otp_release
        .zip_url
        .or(otp_release.exe_url.clone())
        .ok_or_else(|| AppError::msg("No download is available for this OTP version on this OS"))?;

    let cache = dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("elin")
        .join("downloads");
    fs::create_dir_all(&cache)?;

    let otp_archive = cache.join(otp_archive_name(&otp, &otp_url));
    emit_progress(&app, "otp-download", &format!("Downloading Erlang/OTP {otp}…"), 8);
    download_file(&app, &otp_url, &otp_archive, 8, 38).await?;

    let otp_dest = otp_dir(&otp)?;
    emit_progress(&app, "otp-extract", "Unpacking Erlang/OTP…", 42);
    extract_archive(&otp_archive, &otp_dest)?;
    flatten_if_nested(&otp_dest, erl_binary())?;
    run_otp_install_script(&otp_dest)?;
    chmod_binaries(&otp_dest);

    let elixir_url = elixir_zip_url(&elixir, otp_ver.major);
    let elixir_archive = cache.join(format!("elixir_{elixir}_otp_{}.zip", otp_ver.major));
    emit_progress(
        &app,
        "elixir-download",
        &format!("Downloading Elixir {elixir} (OTP {})…", otp_ver.major),
        50,
    );
    download_file(&app, &elixir_url, &elixir_archive, 50, 74).await?;

    let elixir_dest = elixir_dir(&elixir, otp_ver.major)?;
    emit_progress(&app, "elixir-extract", "Unpacking Elixir…", 78);
    extract_archive(&elixir_archive, &elixir_dest)?;
    flatten_if_nested(&elixir_dest, elixir_script())?;
    chmod_binaries(&elixir_dest);

    let otp_bin = find_bin_dir(&otp_dest, erl_binary())?;
    let elixir_bin = find_bin_dir(&elixir_dest, elixir_script())
        .or_else(|_| find_bin_dir(&elixir_dest, "elixir"))?;

    if add_to_path {
        emit_progress(&app, "path", "Adding toolchain folders to your user PATH…", 86);
        set_user_path_entries(&[otp_bin.clone(), elixir_bin.clone()])?;
    } else {
        emit_progress(
            &app,
            "path",
            "Leaving the default PATH unchanged — this pair is for a project pin.",
            86,
        );
    }

    if install_hex {
        emit_progress(&app, "hex", "Installing Hex and Rebar…", 90);
        let _ = run_with_toolchain(
            &otp_bin,
            &elixir_bin,
            "mix",
            &["local.hex", "--force"],
        );
        let _ = run_with_toolchain(
            &otp_bin,
            &elixir_bin,
            "mix",
            &["local.rebar", "--force"],
        );
    }

    emit_progress(&app, "verify", "Verifying elixir -v…", 96);
    let output = run_with_toolchain(&otp_bin, &elixir_bin, "elixir", &["-v"])?;

    let pair = InstalledPair {
        elixir,
        otp,
        elixir_path: elixir_bin.to_string_lossy().into(),
        otp_path: otp_bin.to_string_lossy().into(),
        is_active: add_to_path,
    };

    if add_to_path {
        save_active(&pair)?;
    }
    emit_progress(&app, "done", "Toolchain is ready.", 100);

    Ok(InstallResult {
        pair,
        elixir_version_output: output,
    })
}

/// List Elin-managed installs by scanning the official elixir-install layout.
pub fn list_installed() -> AppResult<Vec<InstalledPair>> {
    let root = installs_dir()?;
    let active = load_active();
    let mut pairs = Vec::new();
    let elixir_root = root.join("elixir");
    if !elixir_root.exists() {
        return Ok(pairs);
    }
    for entry in fs::read_dir(elixir_root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let Some((elixir, otp_major_raw)) = name.rsplit_once("-otp-") else {
            continue;
        };
        let Ok(major) = otp_major_raw.parse::<u32>() else {
            continue;
        };
        let elixir_bin = match find_bin_dir(&entry.path(), elixir_script())
            .or_else(|_| find_bin_dir(&entry.path(), "elixir.bat"))
            .or_else(|_| find_bin_dir(&entry.path(), "elixir"))
        {
            Ok(p) => p,
            Err(_) => continue,
        };
        // Prefer an OTP install whose major matches this Elixir build.
        let otp_root = root.join("otp");
        let otp = find_matching_otp(&otp_root, major).unwrap_or_else(|| format!("{major}.x"));
        let otp_path = otp_dir(&otp)
            .ok()
            .and_then(|p| {
                find_bin_dir(&p, erl_binary())
                    .or_else(|_| find_bin_dir(&p, "erl.exe"))
                    .or_else(|_| find_bin_dir(&p, "erl"))
                    .ok()
            })
            .map(|p| p.to_string_lossy().into())
            .unwrap_or_default();
        let on_path = crate::services::env::path_contains_dir(
            &crate::services::env::user_path(),
            &elixir_bin,
        );
        let matches_state = active
            .as_ref()
            .map(|a| a.elixir == elixir && a.otp.starts_with(&major.to_string()))
            .unwrap_or(false);
        let is_active = on_path || matches_state;
        pairs.push(InstalledPair {
            elixir: elixir.to_string(),
            otp,
            elixir_path: elixir_bin.to_string_lossy().into(),
            otp_path,
            is_active,
        });
    }
    pairs.sort_by(|a, b| b.elixir.cmp(&a.elixir));
    Ok(pairs)
}

fn find_matching_otp(otp_root: &Path, major: u32) -> Option<String> {
    let mut best: Option<OtpVersion> = None;
    let Ok(entries) = fs::read_dir(otp_root) else {
        return None;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(ver) = OtpVersion::parse(&name) {
            if ver.major == major {
                if best.as_ref().map(|b| &ver > b).unwrap_or(true) {
                    best = Some(ver);
                }
            }
        }
    }
    best.map(|v| v.to_string())
}

/// Newest installed Elixir that satisfies a Mix `elixir:` requirement.
pub fn pair_satisfying(req: &str) -> Option<InstalledPair> {
    let mut pairs = list_installed().ok()?;
    pairs.retain(|p| {
        ElixirVersion::parse(&p.elixir)
            .map(|v| crate::domain::elixir_satisfies(req, &v))
            .unwrap_or(false)
            && (Path::new(&p.elixir_path).join(elixir_script()).exists()
                || Path::new(&p.elixir_path).join("elixir.bat").exists()
                || Path::new(&p.elixir_path).join("elixir").exists())
            && (Path::new(&p.otp_path).join(erl_binary()).exists()
                || Path::new(&p.otp_path).join("erl.exe").exists()
                || Path::new(&p.otp_path).join("erl").exists())
    });
    pairs.sort_by(|a, b| {
        ElixirVersion::parse(&b.elixir)
            .unwrap_or(ElixirVersion {
                major: 0,
                minor: 0,
                patch: 0,
                pre: None,
            })
            .cmp(&ElixirVersion::parse(&a.elixir).unwrap_or(ElixirVersion {
                major: 0,
                minor: 0,
                patch: 0,
                pre: None,
            }))
    });
    pairs.into_iter().next()
}

pub fn find_pair(elixir: &str, otp: Option<&str>) -> Option<InstalledPair> {
    let pairs = list_installed().ok()?;
    pairs.into_iter().find(|p| {
        p.elixir == elixir && otp.map(|o| p.otp == o || p.otp.starts_with(o)).unwrap_or(true)
    })
}

/// Pick the newest catalog Elixir matching `req` plus a compatible Windows OTP.
pub fn pick_from_catalog(req: &str, catalog: &VersionCatalog) -> AppResult<(String, String)> {
    let mut elixir_rels: Vec<_> = catalog
        .elixir
        .iter()
        .filter(|r| !r.is_prerelease)
        .filter(|r| {
            ElixirVersion::parse(&r.version)
                .map(|v| crate::domain::elixir_satisfies(req, &v))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    elixir_rels.sort_by(|a, b| {
        ElixirVersion::parse(&b.version)
            .cmp(&ElixirVersion::parse(&a.version))
    });
    let elixir = elixir_rels.first().ok_or_else(|| {
        AppError::msg(format!("No catalog Elixir release matches `{req}`."))
    })?;
    let ev = ElixirVersion::parse(&elixir.version)
        .ok_or_else(|| AppError::msg("Could not parse the chosen Elixir version."))?;
    let available: Vec<u32> = catalog
        .otp
        .iter()
        .filter(|o| !o.is_prerelease)
        .map(|o| o.major)
        .collect();
    let major = recommended_otp_major(&ev, &available)
        .or_else(|| elixir.otp_majors.iter().copied().max())
        .ok_or_else(|| AppError::msg("No compatible OTP major is in the catalog."))?;
    let otp = catalog
        .otp
        .iter()
        .filter(|o| o.major == major && !o.is_prerelease && o.zip_url.is_some())
        .max_by_key(|o| OtpVersion::parse(&o.version))
        .ok_or_else(|| AppError::msg(format!("No OTP {major} build is in the catalog for this OS.")))?;
    Ok((elixir.version.clone(), otp.version.clone()))
}

pub fn activate_pair(elixir: &str, otp: &str) -> AppResult<InstalledPair> {
    let otp_ver = OtpVersion::parse(otp).ok_or_else(|| AppError::msg("Invalid OTP version"))?;
    let otp_bin = find_bin_dir(&otp_dir(otp)?, erl_binary())
        .or_else(|_| find_bin_dir(&otp_dir(otp)?, "erl.exe"))
        .or_else(|_| find_bin_dir(&otp_dir(otp)?, "erl"))?;
    let elixir_bin = find_bin_dir(&elixir_dir(elixir, otp_ver.major)?, elixir_script())
        .or_else(|_| find_bin_dir(&elixir_dir(elixir, otp_ver.major)?, "elixir.bat"))
        .or_else(|_| find_bin_dir(&elixir_dir(elixir, otp_ver.major)?, "elixir"))?;
    set_user_path_entries(&[otp_bin.clone(), elixir_bin.clone()])?;
    let pair = InstalledPair {
        elixir: elixir.into(),
        otp: otp.into(),
        elixir_path: elixir_bin.to_string_lossy().into(),
        otp_path: otp_bin.to_string_lossy().into(),
        is_active: true,
    };
    save_active(&pair)?;
    Ok(pair)
}

pub fn remove_pair(elixir: &str, otp: &str) -> AppResult<()> {
    let otp_ver = OtpVersion::parse(otp).ok_or_else(|| AppError::msg("Invalid OTP version"))?;
    let elixir_path = elixir_dir(elixir, otp_ver.major)?;
    if elixir_path.exists() {
        fs::remove_dir_all(elixir_path)?;
    }
    Ok(())
}

fn state_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("elin").join("state.json"))
}

fn save_active(pair: &InstalledPair) -> AppResult<()> {
    if let Some(path) = state_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(pair).unwrap_or_default())?;
    }
    Ok(())
}

fn load_active() -> Option<InstalledPair> {
    let path = state_path()?;
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

async fn download_file(
    app: &AppHandle,
    url: &str,
    dest: &Path,
    start_percent: u8,
    end_percent: u8,
) -> AppResult<()> {
    if dest.exists() && dest.metadata()?.len() > 1024 {
        emit_progress(app, "cache", "Using a previously downloaded archive…", start_percent);
        return Ok(());
    }
    emit_progress(
        app,
        "download",
        "Starting download…",
        start_percent.saturating_add(1).min(end_percent),
    );
    net::download_to_file(
        url,
        dest,
        None,
        &net::github_download_headers(url),
        |got, total| {
            let span = end_percent.saturating_sub(start_percent) as f32;
            let percent = if total > 0 {
                start_percent + ((span * (got as f32 / total as f32)).round() as u8).min(span as u8)
            } else if got > 0 {
                start_percent.saturating_add(2).min(end_percent)
            } else {
                start_percent
            };
            let message = if total > 0 {
                format!(
                    "Downloading… {} / {}",
                    net::format_bytes(got),
                    net::format_bytes(total)
                )
            } else if got > 0 {
                format!("Downloading… {}", net::format_bytes(got))
            } else {
                "Starting download…".into()
            };
            emit_progress(app, "download", &message, percent.min(end_percent));
        },
    )
    .await?;
    Ok(())
}

fn extract_archive(archive_path: &Path, dest: &Path) -> AppResult<()> {
    match host::otp_archive_kind_from_url(&archive_path.to_string_lossy()) {
        host::ArchiveKind::TarGz => extract_tar_gz(archive_path, dest),
        host::ArchiveKind::Zip => extract_zip(archive_path, dest),
    }
}

fn otp_archive_name(otp: &str, url: &str) -> String {
    match host::otp_archive_kind_from_url(url) {
        host::ArchiveKind::TarGz => format!("otp_{otp}.tar.gz"),
        host::ArchiveKind::Zip => format!("otp_{otp}.zip"),
    }
}

fn extract_tar_gz(archive_path: &Path, dest: &Path) -> AppResult<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;
    let file = File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    archive
        .unpack(dest)
        .map_err(|e| AppError::Install(format!("Could not unpack OTP archive: {e}")))?;
    Ok(())
}

fn extract_zip(archive_path: &Path, dest: &Path) -> AppResult<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let out = dest.join(rel);
        if entry.name().ends_with('/') {
            fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&out)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }
    Ok(())
}

/// Some zips wrap a single top-level folder. Hoist contents if the binary is nested.
fn flatten_if_nested(dest: &Path, binary: &str) -> AppResult<()> {
    if dest.join("bin").join(binary).exists() || dest.join(binary).exists() {
        return Ok(());
    }
    let entries: Vec<_> = fs::read_dir(dest)?.flatten().collect();
    if entries.len() == 1 && entries[0].path().is_dir() {
        let nested = entries[0].path();
        for child in fs::read_dir(&nested)? {
            let child = child?;
            let target = dest.join(child.file_name());
            fs::rename(child.path(), target)?;
        }
        let _ = fs::remove_dir(nested);
    }
    Ok(())
}

fn chmod_binaries(root: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut targets = vec![root.join("Install")];
        if let Ok(bin) = fs::read_dir(root.join("bin")) {
            targets.extend(bin.flatten().map(|e| e.path()));
        }
        for path in targets {
            if !path.is_file() {
                continue;
            }
            if let Ok(meta) = path.metadata() {
                let mut perms = meta.permissions();
                perms.set_mode(perms.mode() | 0o755);
                let _ = fs::set_permissions(&path, perms);
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = root;
    }
}

/// Hex Bob Linux OTP tarballs need `./Install -sasl $PWD` before `erl` works.
fn run_otp_install_script(otp_dest: &Path) -> AppResult<()> {
    let script = otp_dest.join("Install");
    if !script.exists() {
        return Ok(());
    }
    chmod_binaries(otp_dest);
    let mut cmd = Command::new(&script);
    cmd.arg("-sasl")
        .arg(otp_dest)
        .current_dir(otp_dest)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    crate::services::winproc::hide_console(&mut cmd);
    let output = cmd
        .output()
        .map_err(|e| AppError::Install(format!("Could not run OTP Install: {e}")))?;
    if !output.status.success() {
        return Err(AppError::Install(format!(
            "OTP Install script failed: {}",
            crate::services::winproc::output_text(&output).trim()
        )));
    }
    chmod_binaries(otp_dest);
    Ok(())
}

fn find_bin_dir(root: &Path, binary: &str) -> AppResult<PathBuf> {
    let direct = root.join("bin");
    if direct.join(binary).exists() {
        return Ok(direct);
    }
    for entry in walkdir::WalkDir::new(root).max_depth(4) {
        let entry = entry.map_err(|e| AppError::msg(e.to_string()))?;
        if entry.file_name() == binary {
            if let Some(parent) = entry.path().parent() {
                return Ok(parent.to_path_buf());
            }
        }
    }
    Err(AppError::Install(format!(
        "Could not find {binary} inside {}",
        root.display()
    )))
}

pub fn run_with_toolchain(
    otp_bin: &Path,
    elixir_bin: &Path,
    program: &str,
    args: &[&str],
) -> AppResult<String> {
    let exe = if program == "elixir" {
        elixir_cmd(elixir_bin)
    } else if program == "mix" {
        mix_cmd(elixir_bin)
    } else if program == "iex" {
        host::iex_cmd(elixir_bin)
    } else {
        PathBuf::from(program)
    };
    let exe = if exe.exists() {
        exe
    } else {
        host::find_script(elixir_bin, program).unwrap_or(exe)
    };
    if !exe.exists() {
        return Err(AppError::Install(format!(
            "Could not find {} at {}",
            program,
            exe.display()
        )));
    }
    let path = crate::services::winproc::isolated_path(otp_bin, elixir_bin);
    let home = crate::services::winproc::erlang_home(otp_bin);
    let output = crate::services::winproc::run_bat(&exe, args, &path, home.as_deref())?;
    let text = crate::services::winproc::output_text(&output);
    if !output.status.success() {
        return Err(AppError::Install(format!(
            "{program} failed: {}",
            text.trim().if_empty("no output from the process")
        )));
    }
    Ok(text)
}

trait IfEmpty {
    fn if_empty<'a>(&'a self, other: &'a str) -> &'a str;
}

impl IfEmpty for str {
    fn if_empty<'a>(&'a self, other: &'a str) -> &'a str {
        if self.is_empty() {
            other
        } else {
            self
        }
    }
}

/// SHA-256 of a file, used later if we add checksum verification.
#[allow(dead_code)]
pub fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}
