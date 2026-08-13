//! Elin self-update from GitHub Releases (`belurgas/elin`).
//!
//! Checks `releases/latest`, compares tags, downloads the NSIS installer,
//! then launches it and quits so files are not locked.

use crate::domain::ElixirVersion;
use crate::error::{AppError, AppResult};
use crate::services::net::HTTP;
use crate::services::winproc::{hide_console_ex, CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

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
    browser_download_url: String,
    size: u64,
}

struct Cache {
    at: Instant,
    value: AppUpdate,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

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
    let response = HTTP
        .get(LATEST_URL)
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
    let asset = pick_windows_installer(&rel.assets);
    AppUpdate {
        newer: is_newer(&latest, &current),
        current,
        latest: latest.clone(),
        name: rel.name.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| format!("Elin {latest}")),
        notes: clip_notes(rel.body.as_deref().unwrap_or("")),
        html_url: rel.html_url,
        asset_name: asset.map(|a| a.name.clone()),
        asset_url: asset.map(|a| a.browser_download_url.clone()),
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

fn pick_windows_installer(assets: &[GithubAsset]) -> Option<&GithubAsset> {
    let mut exes: Vec<&GithubAsset> = assets
        .iter()
        .filter(|a| a.name.to_ascii_lowercase().ends_with(".exe"))
        .collect();
    exes.sort_by_key(|a| {
        let n = a.name.to_ascii_lowercase();
        (
            !n.contains("setup"),
            n.contains("uninstall"),
            n.contains("msi"),
        )
    });
    exes.into_iter().next()
}

fn clip_notes(body: &str) -> String {
    let trimmed = body.replace('\r', "").trim().to_string();
    if trimmed.chars().count() <= 800 {
        return trimmed;
    }
    let mut out = trimmed.chars().take(800).collect::<String>();
    out.push('…');
    out
}

fn update_dir() -> AppResult<PathBuf> {
    let dir = std::env::temp_dir().join("elin").join("updates");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub async fn download(app: &AppHandle, force: bool) -> AppResult<PathBuf> {
    let info = check(force).await?;
    if !info.newer {
        return Err(AppError::msg("This copy is already the latest Elin."));
    }
    let url = info
        .asset_url
        .ok_or_else(|| AppError::msg("This release has no Windows installer yet."))?;
    let name = info
        .asset_name
        .unwrap_or_else(|| format!("Elin_{}_x64-setup.exe", info.latest));
    let dest = update_dir()?.join(&name);
    if dest.exists() {
        let len = dest.metadata()?.len();
        if info.asset_size.map(|s| s == len).unwrap_or(len > 1024) {
            emit_progress(app, 100, "Already downloaded.", len, info.asset_size.unwrap_or(len));
            return Ok(dest);
        }
        let _ = fs::remove_file(&dest);
    }
    emit_progress(app, 0, "Starting download…", 0, info.asset_size.unwrap_or(0));
    let response = HTTP
        .get(&url)
        .timeout(Duration::from_secs(600))
        .send()
        .await?
        .error_for_status()?;
    let total = response.content_length().unwrap_or(info.asset_size.unwrap_or(0));
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&dest).await?;
    let mut downloaded: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).await?;
        let percent = if total > 0 {
            ((downloaded as f64 / total as f64) * 100.0).round() as u8
        } else {
            0
        };
        emit_progress(
            app,
            percent.min(99),
            &format!("Downloading… {} / {}", bytes(downloaded), bytes(total)),
            downloaded,
            total,
        );
    }
    file.flush().await?;
    emit_progress(app, 100, "Download complete.", downloaded, total);
    Ok(dest)
}

pub fn launch_installer(app: &AppHandle, path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(AppError::msg("The installer file is missing. Download it again."));
    }
    let mut cmd = std::process::Command::new(path);
    hide_console_ex(&mut cmd, DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    cmd.spawn()?;
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        handle.exit(0);
    });
    Ok(())
}

fn emit_progress(app: &AppHandle, percent: u8, message: &str, downloaded: u64, total: u64) {
    let _ = app.emit(
        "app-update-progress",
        UpdateProgress {
            percent,
            message: message.into(),
            downloaded,
            total,
        },
    );
}

fn bytes(n: u64) -> String {
    if n >= 1_048_576 {
        format!("{:.1} MB", n as f64 / 1_048_576.0)
    } else if n >= 1024 {
        format!("{:.0} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
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
                browser_download_url: "https://example.com/setup.exe".into(),
                size: 10,
            }],
        };
        let info = from_release(rel);
        assert_eq!(info.latest, "1.2.3");
        assert_eq!(info.asset_name.as_deref(), Some("Elin_1.2.3_x64-setup.exe"));
    }

    #[test]
    fn prefers_nsis_setup_over_other_exe() {
        let assets = vec![
            GithubAsset {
                name: "notes.exe".into(),
                browser_download_url: "a".into(),
                size: 1,
            },
            GithubAsset {
                name: "Elin_1.0.0_x64-setup.exe".into(),
                browser_download_url: "b".into(),
                size: 2,
            },
        ];
        let hit = pick_windows_installer(&assets).unwrap();
        assert!(hit.name.contains("setup"));
    }

    #[test]
    fn clips_long_notes() {
        let long = "n".repeat(900);
        let clipped = clip_notes(&long);
        assert!(clipped.ends_with('…'));
        assert!(clipped.chars().count() <= 801);
    }
}
