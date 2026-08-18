//! Elin self-update from GitHub Releases (`belurgas/elin`).
//!
//! Checks `releases/latest`, compares tags, downloads the NSIS installer,
//! then launches it and quits so files are not locked.

use crate::domain::ElixirVersion;
use crate::error::{AppError, AppResult};
use crate::services::net::{self, get_api};
#[cfg(windows)]
use crate::services::winproc::{hide_console_ex, CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub const REPO: &str = "belurgas/elin";
const LATEST_URL: &str = "https://api.github.com/repos/belurgas/elin/releases/latest";
const FRESH: Duration = Duration::from_secs(50 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdate {
    pub current: String,
    pub latest: String,
    pub newer: bool,
    pub name: String,
    pub notes: String,
    pub html_url: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub asset_browser_url: Option<String>,
    pub asset_size: Option<u64>,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub percent: u8,
    pub message: String,
    pub downloaded: u64,
    pub total: u64,
    #[serde(default)]
    pub stage: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    #[serde(default)]
    url: String,
    browser_download_url: String,
    size: u64,
}

struct Cache {
    at: Instant,
    value: AppUpdate,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);

pub fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn up_to_date() -> AppUpdate {
    let current = current_version();
    AppUpdate {
        latest: current.clone(),
        current: current.clone(),
        newer: false,
        name: format!("Elin {current}"),
        notes: String::new(),
        html_url: format!("https://github.com/{REPO}/releases"),
        asset_name: None,
        asset_url: None,
        asset_browser_url: None,
        asset_size: None,
        published_at: None,
    }
}

pub async fn check(force: bool) -> AppResult<AppUpdate> {
    if !force {
        if let Ok(guard) = CACHE.lock() {
            if let Some(cached) = guard.as_ref() {
                if cached.at.elapsed() < FRESH {
                    return Ok(cached.value.clone());
                }
            }
        }
    }
    let found = match fetch_latest().await {
        Ok(rel) => from_release(rel),
        Err(err) if is_not_found(&err) => up_to_date(),
        Err(err) => {
            if let Ok(guard) = CACHE.lock() {
                if let Some(cached) = guard.as_ref() {
                    return Ok(cached.value.clone());
                }
            }
            return Err(err);
        }
    };
    if let Ok(mut guard) = CACHE.lock() {
        *guard = Some(Cache {
            at: Instant::now(),
            value: found.clone(),
        });
    }
    Ok(found)
}

fn is_not_found(err: &AppError) -> bool {
    let text = err.to_string().to_ascii_lowercase();
    text.contains("404") || text.contains("not found")
}

async fn fetch_latest() -> AppResult<GithubRelease> {
    let response = get_api(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;
    if response.status().as_u16() == 404 {
        return Err(AppError::msg("404"));
    }
    let rel = response.error_for_status()?.json::<GithubRelease>().await?;
    if rel.draft {
        return Err(AppError::msg("404"));
    }
    let _ = rel.prerelease;
    Ok(rel)
}

fn from_release(rel: GithubRelease) -> AppUpdate {
    let current = current_version();
    let latest = rel.tag_name.trim().trim_start_matches('v').to_string();
    let asset = pick_installer(&rel.assets);
    AppUpdate {
        newer: is_newer(&latest, &current),
        current,
        latest: latest.clone(),
        name: rel.name.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| format!("Elin {latest}")),
        notes: clip_notes(rel.body.as_deref().unwrap_or("")),
        html_url: rel.html_url,
        asset_name: asset.map(|a| a.name.clone()),
        asset_url: asset.map(|a| net::github_asset_url(&a.url, &a.browser_download_url)),
        asset_browser_url: asset.map(|a| a.browser_download_url.clone()),
        asset_size: asset.map(|a| a.size),
        published_at: rel.published_at,
    }
}

fn is_newer(latest: &str, current: &str) -> bool {
    match (ElixirVersion::parse(latest), ElixirVersion::parse(current)) {
        (Some(a), Some(b)) => a > b,
        _ => latest != current && !latest.is_empty(),
    }
}

fn default_installer_name(version: &str) -> String {
    if cfg!(windows) {
        format!("Elin_{version}_x64-setup.exe")
    } else if cfg!(target_os = "macos") {
        format!("Elin_{version}_aarch64.dmg")
    } else {
        format!("elin_{version}_amd64.AppImage")
    }
}

fn pick_installer(assets: &[GithubAsset]) -> Option<&GithubAsset> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let mut scored: Vec<(i32, &GithubAsset)> = assets
        .iter()
        .filter_map(|a| score_asset(a, os, arch).map(|s| (s, a)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, a)| a).next()
}

fn score_asset(asset: &GithubAsset, os: &str, arch: &str) -> Option<i32> {
    let n = asset.name.to_ascii_lowercase();
    if n.contains("uninstall") {
        return None;
    }
    let mut score = 0;
    match os {
        "windows" => {
            if n.ends_with(".exe") {
                score += 50;
            } else if n.ends_with(".msi") {
                score += 30;
            } else {
                return None;
            }
            if n.contains("setup") {
                score += 20;
            }
        }
        "macos" => {
            if n.ends_with(".dmg") {
                score += 50;
            } else if n.ends_with(".app.tar.gz") {
                score += 35;
            } else {
                return None;
            }
            if arch == "aarch64" && (n.contains("aarch64") || n.contains("arm64") || n.contains("universal")) {
                score += 15;
            }
            if arch == "x86_64" && (n.contains("x64") || n.contains("x86_64") || n.contains("universal")) {
                score += 15;
            }
        }
        "linux" => {
            if n.ends_with(".appimage") {
                score += 50;
            } else if n.ends_with(".deb") {
                score += 40;
            } else if n.ends_with(".rpm") {
                score += 30;
            } else {
                return None;
            }
            if arch == "x86_64" && (n.contains("amd64") || n.contains("x86_64") || n.contains("x64")) {
                score += 15;
            }
            if arch == "aarch64" && (n.contains("arm64") || n.contains("aarch64")) {
                score += 15;
            }
        }
        _ => return None,
    }
    Some(score)
}

fn clip_notes(body: &str) -> String {
    let trimmed = body.replace('\r', "").trim().to_string();
    const MAX: usize = 80_000;
    if trimmed.chars().count() <= MAX {
        return trimmed;
    }
    let mut out = trimmed.chars().take(MAX).collect::<String>();
    out.push('…');
    out
}

fn update_dir() -> AppResult<PathBuf> {
    let dir = std::env::temp_dir().join("elin").join("updates");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn start(app: &AppHandle, force: bool) -> AppResult<()> {
    if BUSY.swap(true, Ordering::SeqCst) {
        return Err(AppError::msg("An update is already running."));
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        emit_progress(&app, 1, "Starting download…", 0, 0, "download");
        match download(&app, force).await {
            Ok(path) => {
                emit_progress(&app, 100, "Launching installer…", 0, 0, "install");
                if let Err(err) = launch_installer(&app, &path) {
                    emit_progress(&app, 0, &err.to_string(), 0, 0, "error");
                    BUSY.store(false, Ordering::SeqCst);
                }
            }
            Err(err) => {
                emit_progress(&app, 0, &err.to_string(), 0, 0, "error");
                BUSY.store(false, Ordering::SeqCst);
            }
        }
    });
    Ok(())
}

pub async fn download(app: &AppHandle, force: bool) -> AppResult<PathBuf> {
    let info = check(force).await?;
    if !info.newer {
        return Err(AppError::msg("This copy is already the latest Elin."));
    }
    // Public GitHub release files: the browser URL hits the CDN directly.
    // The API asset URL needs Accept + redirects and is the one that hung the UI.
    let primary = info
        .asset_browser_url
        .clone()
        .or(info.asset_url.clone())
        .ok_or_else(|| AppError::msg("This release has no installer for this OS yet."))?;
    let fallback = info.asset_url.clone().filter(|u| u != &primary);
    let name = info
        .asset_name
        .unwrap_or_else(|| default_installer_name(&info.latest));
    let dest = update_dir()?.join(&name);
    if dest.exists() {
        let len = dest.metadata()?.len();
        if info.asset_size.map(|s| s == len).unwrap_or(len > 1024) {
            emit_progress(app, 100, "Already downloaded.", len, info.asset_size.unwrap_or(len), "download");
            return Ok(dest);
        }
        let _ = fs::remove_file(&dest);
    }

    let size = info.asset_size;
    let first = pull_installer(app, &primary, &dest, size).await;
    match first {
        Ok(()) => Ok(dest),
        Err(err) => {
            if let Some(url) = fallback {
                let _ = fs::remove_file(dest.with_extension("partial"));
                pull_installer(app, &url, &dest, size).await?;
                Ok(dest)
            } else {
                Err(err)
            }
        }
    }
}

async fn pull_installer(app: &AppHandle, url: &str, dest: &Path, size: Option<u64>) -> AppResult<()> {
    emit_progress(app, 1, "Starting download…", 0, size.unwrap_or(0), "download");
    net::download_to_file(url, dest, size, &net::github_download_headers(url), |got, total| {
        let percent = if total > 0 {
            net::percent(got, total).min(99)
        } else if got > 0 {
            15
        } else {
            1
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
        emit_progress(app, percent, &message, got, total, "download");
    })
    .await?;
    let len = dest.metadata()?.len();
    emit_progress(app, 100, "Download complete.", len, size.unwrap_or(len), "download");
    Ok(())
}

pub fn launch_installer(app: &AppHandle, path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(AppError::msg("The installer file is missing. Download it again."));
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let allowed_ext = name.ends_with(".exe")
        || name.ends_with(".msi")
        || name.ends_with(".dmg")
        || name.ends_with(".appimage")
        || name.ends_with(".deb")
        || name.ends_with(".rpm");
    if !allowed_ext {
        return Err(AppError::msg("That file is not an Elin installer."));
    }
    let updates = update_dir()?;
    match (fs::canonicalize(path), fs::canonicalize(&updates)) {
        (Ok(file), Ok(dir)) if file.starts_with(&dir) => {}
        _ if path.starts_with(&updates) => {}
        _ => return Err(AppError::msg("Installer must be an Elin download.")),
    }
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new(path);
        hide_console_ex(&mut cmd, DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        cmd.spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.ends_with(".AppImage") || name.ends_with(".appimage") {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(path) {
                let mut perms = meta.permissions();
                perms.set_mode(perms.mode() | 0o755);
                let _ = std::fs::set_permissions(path, perms);
            }
            std::process::Command::new(path).spawn()?;
        } else if name.ends_with(".deb") {
            std::process::Command::new("xdg-open").arg(path).spawn()?;
        } else {
            std::process::Command::new("xdg-open").arg(path).spawn()?;
        }
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        handle.exit(0);
    });
    Ok(())
}

fn emit_progress(app: &AppHandle, percent: u8, message: &str, downloaded: u64, total: u64, stage: &str) {
    let _ = app.emit(
        "app-update-progress",
        UpdateProgress {
            percent,
            message: message.into(),
            downloaded,
            total,
            stage: stage.into(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_compares_semver() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.2"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("0.1.1", "0.1.0"));
    }

    #[test]
    fn strips_v_prefix() {
        let rel = GithubRelease {
            tag_name: "v1.2.3".into(),
            name: Some("Elin 1.2.3".into()),
            body: Some("hello".into()),
            html_url: "https://github.com/belurgas/elin/releases/tag/v1.2.3".into(),
            draft: false,
            prerelease: false,
            published_at: None,
            assets: vec![GithubAsset {
                name: "Elin_1.2.3_x64-setup.exe".into(),
                url: "https://api.github.com/repos/belurgas/elin/releases/assets/1".into(),
                browser_download_url: "https://example.com/setup.exe".into(),
                size: 10,
            }],
        };
        let info = from_release(rel);
        assert_eq!(info.latest, "1.2.3");
        assert_eq!(info.asset_name.as_deref(), Some("Elin_1.2.3_x64-setup.exe"));
        assert_eq!(
            info.asset_url.as_deref(),
            Some("https://api.github.com/repos/belurgas/elin/releases/assets/1")
        );
    }

    #[cfg(windows)]
    #[test]
    fn prefers_nsis_setup_over_other_exe() {
        let assets = vec![
            GithubAsset {
                name: "notes.exe".into(),
                url: String::new(),
                browser_download_url: "a".into(),
                size: 1,
            },
            GithubAsset {
                name: "Elin_1.0.0_x64-setup.exe".into(),
                url: String::new(),
                browser_download_url: "b".into(),
                size: 2,
            },
        ];
        let hit = pick_installer(&assets).unwrap();
        assert!(hit.name.contains("setup"));
    }

    #[test]
    fn prefers_platform_package() {
        let assets = vec![
            GithubAsset {
                name: "Elin_1.0.0_x64-setup.exe".into(),
                url: String::new(),
                browser_download_url: "a".into(),
                size: 1,
            },
            GithubAsset {
                name: "Elin_1.0.0_aarch64.dmg".into(),
                url: String::new(),
                browser_download_url: "b".into(),
                size: 2,
            },
            GithubAsset {
                name: "elin_1.0.0_amd64.AppImage".into(),
                url: String::new(),
                browser_download_url: "c".into(),
                size: 3,
            },
        ];
        let hit = pick_installer(&assets).unwrap();
        let n = hit.name.to_ascii_lowercase();
        if cfg!(windows) {
            assert!(n.ends_with(".exe"));
        } else if cfg!(target_os = "macos") {
            assert!(n.ends_with(".dmg"));
        } else {
            assert!(n.ends_with(".appimage"));
        }
    }

    #[test]
    fn clips_huge_notes_only() {
        let long = "n".repeat(90_000);
        let clipped = clip_notes(&long);
        assert!(clipped.ends_with('…'));
        assert!(clipped.chars().count() <= 80_001);
        assert_eq!(clip_notes("# Hello\n\n- one"), "# Hello\n\n- one");
    }
}
