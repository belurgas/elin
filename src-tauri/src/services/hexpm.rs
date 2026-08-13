//! Hex Radar: search packages on hex.pm without leaving Elin.

use crate::error::AppResult;
use crate::services::net::HTTP;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HexPackage {
    pub name: String,
    pub description: Option<String>,
    pub downloads: Option<u64>,
    pub downloads_recent: Option<u64>,
    pub latest: Option<String>,
    pub html_url: String,
    pub docs_url: String,
    #[serde(default)]
    pub licenses: Vec<String>,
    #[serde(default)]
    pub links: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct HexSearchItem {
    name: String,
    meta: Option<HexMeta>,
    downloads: Option<HexDownloads>,
    html_url: Option<String>,
    docs_html_url: Option<String>,
    latest_version: Option<String>,
    latest_stable_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HexMeta {
    description: Option<String>,
    #[serde(default)]
    licenses: Vec<String>,
    #[serde(default)]
    links: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct HexDownloads {
    all: Option<u64>,
    recent: Option<u64>,
}

/// Search hex.pm. Empty query returns currently trending packages (cached 15 min).
pub async fn search_packages(query: String, force: bool) -> AppResult<Vec<HexPackage>> {
    let trending = query.trim().is_empty();
    if trending && !force {
        if let Some((cached, age)) =
            crate::services::cache::read_json::<Vec<HexPackage>>("hex-trending.json")
        {
            if age < crate::services::cache::hex_ttl() {
                return Ok(cached);
            }
        }
    }
    let url = if trending {
        "https://hex.pm/api/packages?sort=recent_downloads".to_string()
    } else {
        format!(
            "https://hex.pm/api/packages?search={}&sort=recent_downloads",
            urlencoding::encode(query.trim())
        )
    };
    let items = HTTP
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<HexSearchItem>>()
        .await?;

    let packages: Vec<HexPackage> = items
        .into_iter()
        .take(28)
        .map(|item| {
            let html = item
                .html_url
                .unwrap_or_else(|| format!("https://hex.pm/packages/{}", item.name));
            let docs = item
                .docs_html_url
                .unwrap_or_else(|| format!("https://hexdocs.pm/{}", item.name));
            HexPackage {
                name: item.name,
                description: item.meta.as_ref().and_then(|m| m.description.clone()),
                downloads: item.downloads.as_ref().and_then(|d| d.all),
                downloads_recent: item.downloads.and_then(|d| d.recent),
                latest: item.latest_stable_version.or(item.latest_version),
                html_url: html,
                docs_url: docs,
                licenses: item.meta.as_ref().map(|m| m.licenses.clone()).unwrap_or_default(),
                links: item.meta.map(|m| m.links).unwrap_or_default(),
            }
        })
        .collect();
    if trending {
        crate::services::cache::write_json("hex-trending.json", &packages);
    }
    Ok(packages)
}

/// Full hex.pm package record (licenses, links, longer meta).
pub async fn get_package(name: String) -> AppResult<HexPackage> {
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(crate::error::AppError::msg("Package name looks invalid."));
    }
    let url = format!("https://hex.pm/api/packages/{name}");
    let item = HTTP
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<HexSearchItem>()
        .await?;
    let html = item
        .html_url
        .unwrap_or_else(|| format!("https://hex.pm/packages/{}", item.name));
    let docs = item
        .docs_html_url
        .unwrap_or_else(|| format!("https://hexdocs.pm/{}", item.name));
    Ok(HexPackage {
        name: item.name,
        description: item.meta.as_ref().and_then(|m| m.description.clone()),
        downloads: item.downloads.as_ref().and_then(|d| d.all),
        downloads_recent: item.downloads.and_then(|d| d.recent),
        latest: item.latest_stable_version.or(item.latest_version),
        html_url: html,
        docs_url: docs,
        licenses: item.meta.as_ref().map(|m| m.licenses.clone()).unwrap_or_default(),
        links: item.meta.map(|m| m.links).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_search_payload() {
        let raw = r#"[{
            "name":"phoenix",
            "html_url":"https://hex.pm/packages/phoenix",
            "docs_html_url":"https://hexdocs.pm/phoenix",
            "latest_version":"1.8.11",
            "latest_stable_version":"1.8.11",
            "meta":{"description":"Peace of mind"},
            "downloads":{"all":153397254}
        }]"#;
        let items: Vec<HexSearchItem> = serde_json::from_str(raw).unwrap();
        assert_eq!(items[0].name, "phoenix");
        assert_eq!(items[0].latest_stable_version.as_deref(), Some("1.8.11"));
        assert_eq!(items[0].downloads.as_ref().unwrap().all, Some(153397254));
    }
}
