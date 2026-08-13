//! On-disk cache with TTL. Catalog, Hex Radar, and probe snapshots share one folder.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CATALOG_TTL_SECS: u64 = 30 * 60;
const HEX_TTL_SECS: u64 = 15 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    pub catalog_age_secs: Option<u64>,
    pub catalog_fresh: bool,
    pub hex_age_secs: Option<u64>,
    pub hex_fresh: bool,
    pub dir: String,
}

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("elin")
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn status() -> CacheStatus {
    let dir = cache_dir();
    let catalog_age = file_age(dir.join("catalog.json"));
    let hex_age = file_age(dir.join("hex-trending.json"));
    CacheStatus {
        catalog_fresh: catalog_age.map(|a| a < CATALOG_TTL_SECS).unwrap_or(false),
        hex_fresh: hex_age.map(|a| a < HEX_TTL_SECS).unwrap_or(false),
        catalog_age_secs: catalog_age,
        hex_age_secs: hex_age,
        dir: dir.to_string_lossy().into(),
    }
}

fn file_age(path: PathBuf) -> Option<u64> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let secs = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(now_unix().saturating_sub(secs))
}

pub fn catalog_ttl() -> u64 {
    CATALOG_TTL_SECS
}

pub fn hex_ttl() -> u64 {
    HEX_TTL_SECS
}

pub fn read_json<T: for<'de> Deserialize<'de>>(name: &str) -> Option<(T, u64)> {
    let path = cache_dir().join(name);
    let raw = fs::read_to_string(&path).ok()?;
    let value = serde_json::from_str(&raw).ok()?;
    let age = file_age(path)?;
    Some((value, age))
}

pub fn write_json<T: Serialize>(name: &str, value: &T) {
    let dir = cache_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(value) {
        let _ = fs::write(dir.join(name), json);
    }
}

pub fn clear() {
    if let Ok(entries) = fs::read_dir(cache_dir()) {
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path());
        }
    }
}
