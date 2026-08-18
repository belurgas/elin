//! Live version catalog.
//!
//! Elixir builds come from Hex Bob (`builds.hex.pm`), which is the same index
//! asdf, mise, and the official install script use. OTP archives are
//! platform-specific: GitHub `otp_win64_*.zip` on Windows, erlef/otp_builds
//! on macOS, Hex Bob Ubuntu tarballs on Linux. Latest versions are always
//! resolved from the network; a short on-disk cache is only used when the
//! network fails.

use crate::domain::{
    compatible_otp_majors, recommended_otp_major, ElixirRelease, ElixirVersion, OtpRelease,
    OtpVersion, VersionCatalog,
};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

const ELIXIR_BUILDS: &str = "https://builds.hex.pm/builds/elixir/builds.txt";
const OTP_RELEASES: &str = "https://api.github.com/repos/erlang/otp/releases?per_page=100";
const OTP_LATEST: &str = "https://api.github.com/repos/erlang/otp/releases/latest";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CachedCatalog {
    catalog: VersionCatalog,
    unix: u64,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn cache_path() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|p| p.join("elin").join("catalog.json"))
}

fn read_cache() -> Option<VersionCatalog> {
    let path = cache_path()?;
    let raw = fs::read_to_string(path).ok()?;
    let cached: CachedCatalog = serde_json::from_str(&raw).ok()?;
    Some(cached.catalog)
}

fn write_cache(catalog: &VersionCatalog) {
    if let Some(path) = cache_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let payload = CachedCatalog {
            catalog: catalog.clone(),
            unix: now_unix(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&payload) {
            let _ = fs::write(path, json);
        }
    }
}

/// Fetch the catalog. Fresh cache (< 30 min) is returned immediately unless `force`.
pub async fn fetch_catalog(include_prerelease: bool, force: bool) -> AppResult<VersionCatalog> {
    if !force {
        if let Some(cached) = read_fresh_cache() {
            return Ok(cached);
        }
    }
    match fetch_live(include_prerelease).await {
        Ok(catalog) => {
            write_cache(&catalog);
            Ok(catalog)
        }
        Err(err) => {
            if let Some(cached) = read_cache() {
                Ok(cached)
            } else {
                Err(err)
            }
        }
    }
}

fn read_fresh_cache() -> Option<VersionCatalog> {
    let path = cache_path()?;
    let raw = fs::read_to_string(&path).ok()?;
    let cached: CachedCatalog = serde_json::from_str(&raw).ok()?;
    if now_unix().saturating_sub(cached.unix) < crate::services::cache::catalog_ttl() {
        Some(cached.catalog)
    } else {
        None
    }
}

async fn fetch_live(include_prerelease: bool) -> AppResult<VersionCatalog> {
    let elixir_text = crate::services::net::get_api(ELIXIR_BUILDS)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let elixir_map = parse_elixir_builds(&elixir_text);
    if elixir_map.is_empty() {
        return Err(AppError::msg(
            "Hex build index did not contain any Elixir releases",
        ));
    }

    let otp = match fetch_otp_releases().await {
        Ok(list) => list,
        Err(_) => synthesize_otp_from_elixir(&elixir_map),
    };

    Ok(assemble_catalog(elixir_map, otp, include_prerelease))
}

/// Parse Bob's `builds.txt`. Lines look like:
/// `v1.20.3-otp-28 2026-01-15T12:00:00Z <sha>`
pub fn parse_elixir_builds(text: &str) -> BTreeMap<String, BTreeSet<u32>> {
    let mut map: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for line in text.lines() {
        let token = line.split_whitespace().next().unwrap_or("");
        if token.is_empty() || token.starts_with("main") || token.starts_with("master") {
            continue;
        }
        let Some((version_raw, otp_raw)) = token.rsplit_once("-otp-") else {
            continue;
        };
        let Ok(otp_major) = otp_raw.parse::<u32>() else {
            continue;
        };
        let version = version_raw.trim_start_matches('v').to_string();
        if ElixirVersion::parse(&version).is_none() {
            continue;
        }
        map.entry(version).or_default().insert(otp_major);
    }
    map
}

async fn fetch_otp_releases() -> AppResult<Vec<OtpRelease>> {
    let mut collected: Vec<GithubRelease> = Vec::new();

    if let Ok(latest) = crate::services::net::get_api(OTP_LATEST)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        if latest.status().is_success() {
            if let Ok(rel) = latest.json::<GithubRelease>().await {
                collected.push(rel);
            }
        }
    }

    let page = crate::services::net::get_api(OTP_RELEASES)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<GithubRelease>>()
        .await?;
    collected.extend(page);

    let mut seen = BTreeSet::new();
    let mut releases = Vec::new();
    for rel in collected {
        if !seen.insert(rel.tag_name.clone()) {
            continue;
        }
        let Some(ver) = OtpVersion::parse(&rel.tag_name) else {
            continue;
        };
        let version = ver.to_string();
        let exe_url = rel.assets.iter().find_map(|a| {
            if a.name == format!("otp_win64_{version}.exe") {
                Some(crate::services::net::github_asset_url(&a.url, &a.browser_download_url))
            } else {
                None
            }
        });
        let zip_url = otp_archive_url(&rel, &version);
        if zip_url.is_none() && exe_url.is_none() {
            continue;
        }
        releases.push(OtpRelease {
            version,
            major: ver.major,
            zip_url,
            exe_url,
            is_latest: false,
            is_prerelease: rel.prerelease || rel.tag_name.contains("rc"),
            published_at: rel.published_at,
        });
    }

    releases.sort_by(|a, b| {
        OtpVersion::parse(&b.version)
            .unwrap_or(OtpVersion {
                major: 0,
                minor: 0,
                patch: 0,
                extra: 0,
            })
            .cmp(&OtpVersion::parse(&a.version).unwrap_or(OtpVersion {
                major: 0,
                minor: 0,
                patch: 0,
                extra: 0,
            }))
    });
    if let Some(first) = releases.iter_mut().find(|r| !r.is_prerelease) {
        first.is_latest = true;
    }
    Ok(releases)
}

/// When GitHub is rate-limited, still offer OTP majors that Hex already built against.
fn synthesize_otp_from_elixir(elixir_map: &BTreeMap<String, BTreeSet<u32>>) -> Vec<OtpRelease> {
    let majors: BTreeSet<u32> = elixir_map.values().flatten().copied().collect();
    majors
        .into_iter()
        .rev()
        .map(|major| OtpRelease {
            version: format!("{major}.0"),
            major,
            zip_url: Some(platform_otp_url(&format!("{major}.0"))),
            exe_url: None,
            is_latest: false,
            is_prerelease: false,
            published_at: None,
        })
        .collect()
}

fn assemble_catalog(
    elixir_map: BTreeMap<String, BTreeSet<u32>>,
    otp: Vec<OtpRelease>,
    include_prerelease: bool,
) -> VersionCatalog {
    let available_majors: Vec<u32> = otp.iter().map(|r| r.major).collect::<BTreeSet<_>>().into_iter().collect();

    let mut elixir_pairs: Vec<(ElixirVersion, ElixirRelease)> = elixir_map
        .into_iter()
        .filter_map(|(version, majors)| {
            let parsed = ElixirVersion::parse(&version)?;
            if !include_prerelease && parsed.is_prerelease() {
                return None;
            }
            let is_prerelease = parsed.is_prerelease();
            let mut otp_majors: Vec<u32> = majors.into_iter().collect();
            if otp_majors.is_empty() {
                otp_majors = compatible_otp_majors(&parsed);
            }
            otp_majors.sort();
            Some((
                parsed,
                ElixirRelease {
                    version,
                    otp_majors,
                    is_latest: false,
                    is_prerelease,
                },
            ))
        })
        .collect();

    elixir_pairs.sort_by(|a, b| b.0.cmp(&a.0));
    if let Some((_, rel)) = elixir_pairs.iter_mut().find(|(_, r)| !r.is_prerelease) {
        rel.is_latest = true;
    }
    let elixir: Vec<ElixirRelease> = elixir_pairs.into_iter().map(|(_, r)| r).collect();

    let latest_elixir = elixir.iter().find(|r| r.is_latest).map(|r| r.version.clone());
    let latest_otp = otp.iter().find(|r| r.is_latest).map(|r| r.version.clone());

    let recommended_elixir = latest_elixir.clone();
    let recommended_otp = recommended_elixir.as_ref().and_then(|ver| {
        let parsed = ElixirVersion::parse(ver)?;
        let major = recommended_otp_major(&parsed, &available_majors)?;
        otp.iter()
            .filter(|r| r.major == major && !r.is_prerelease)
            .max_by_key(|r| OtpVersion::parse(&r.version))
            .map(|r| r.version.clone())
    });

    VersionCatalog {
        elixir,
        otp,
        latest_elixir,
        latest_otp,
        recommended_elixir,
        recommended_otp,
        fetched_at: chrono_like_now(),
        source: crate::services::host::catalog_source().into(),
    }
}

fn chrono_like_now() -> String {
    // Keep the crate graph small: RFC3339-ish UTC from UNIX time is enough for the UI.
    let secs = now_unix();
    format!("{secs}")
}

/// Hex zip URL for a specific Elixir × OTP-major pair.
pub fn elixir_zip_url(elixir: &str, otp_major: u32) -> String {
    format!(
        "https://builds.hex.pm/builds/elixir/v{elixir}-otp-{otp_major}.zip"
    )
}

fn otp_archive_url(rel: &GithubRelease, version: &str) -> Option<String> {
    #[cfg(windows)]
    {
        rel.assets.iter().find_map(|a| {
            if a.name == format!("otp_win64_{version}.zip") {
                Some(crate::services::net::github_asset_url(&a.url, &a.browser_download_url))
            } else {
                None
            }
        })
    }
    #[cfg(not(windows))]
    {
        let _ = rel;
        Some(platform_otp_url(version))
    }
}

fn platform_otp_url(version: &str) -> String {
    if cfg!(windows) {
        format!("https://github.com/erlang/otp/releases/download/OTP-{version}/otp_win64_{version}.zip")
    } else if cfg!(target_os = "macos") {
        crate::services::host::darwin_otp_url(version)
    } else {
        crate::services::host::linux_otp_url(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_otp_url_matches_os() {
        let url = platform_otp_url("27.1.2");
        if cfg!(windows) {
            assert!(url.contains("otp_win64_27.1.2.zip"));
        } else if cfg!(target_os = "macos") {
            assert!(url.contains("erlef/otp_builds"));
            assert!(url.ends_with(".tar.gz"));
        } else {
            assert!(url.contains("builds.hex.pm/builds/otp/"));
            assert!(url.ends_with("/OTP-27.1.2.tar.gz"));
        }
    }

    #[test]
    fn parses_bob_builds_index() {
        let sample = "\
v1.20.3-otp-29 2026-01-01T00:00:00Z abc
v1.20.3-otp-28 2026-01-01T00:00:00Z def
v1.19.4-otp-27 2025-12-01T00:00:00Z ghi
main-otp-28 2026-01-02T00:00:00Z jkl
";
        let map = parse_elixir_builds(sample);
        assert_eq!(map.get("1.20.3").unwrap().len(), 2);
        assert!(map.get("1.20.3").unwrap().contains(&29));
        assert!(!map.contains_key("main"));
    }

    #[tokio::test]
    async fn live_hex_builds_index_is_reachable() {
        let Ok(resp) = crate::services::net::get_api(ELIXIR_BUILDS).send().await else {
            eprintln!("skip: hex.pm unreachable from this runner");
            return;
        };
        let Ok(resp) = resp.error_for_status() else {
            eprintln!("skip: hex.pm returned an error status");
            return;
        };
        let Ok(text) = resp.text().await else {
            eprintln!("skip: hex.pm body unreadable");
            return;
        };
        let map = parse_elixir_builds(&text);
        assert!(
            map.keys().any(|v| v.starts_with("1.")),
            "expected at least one Elixir 1.x build in the live index"
        );
    }
}
