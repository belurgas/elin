//! Shared HTTP client used by catalog, download, and Hex.pm calls.

use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

/// GitHub requires a descriptive UA or it may reject the call.
pub fn user_agent() -> String {
    format!(
        "Elin/{} (+https://github.com/belurgas/elin)",
        env!("CARGO_PKG_VERSION")
    )
}

/// Process-wide client with rustls. JSON calls set a 45s timeout on the request.
/// Downloads set their own long timeout and a per-chunk stall watchdog.
pub static HTTP: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent(user_agent())
        .connect_timeout(Duration::from_secs(20))
        .pool_idle_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(16))
        .build()
        .expect("failed to build HTTP client")
});

const API_TIMEOUT: Duration = Duration::from_secs(45);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const STALL: Duration = Duration::from_secs(45);

/// GET for catalogs and JSON APIs — must not hang forever.
pub fn get_api(url: impl AsRef<str>) -> reqwest::RequestBuilder {
    HTTP.get(url.as_ref()).timeout(API_TIMEOUT)
}

/// Prefer the GitHub Releases *API* asset URL. `browser_download_url` can stall
/// on github.com → objects.githubusercontent.com without the octet-stream Accept.
pub fn github_asset_url(api_url: &str, browser_url: &str) -> String {
    if api_url.contains("/releases/assets/") {
        api_url.to_string()
    } else {
        browser_url.to_string()
    }
}

pub fn github_download_headers(url: &str) -> Vec<(&'static str, &'static str)> {
    if url.contains("api.github.com") {
        vec![("Accept", "application/octet-stream")]
    } else {
        Vec::new()
    }
}

pub fn percent(downloaded: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    ((downloaded as f64 / total as f64) * 100.0).round() as u8
}

pub fn format_bytes(n: u64) -> String {
    if n >= 1_048_576 {
        format!("{:.1} MB", n as f64 / 1_048_576.0)
    } else if n >= 1024 {
        format!("{:.0} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

/// Stream `url` onto `dest`. Calls `on_progress(downloaded, total)` as bytes arrive.
/// `total` is 0 when the server omits Content-Length.
pub async fn download_to_file(
    url: &str,
    dest: &Path,
    expected_size: Option<u64>,
    headers: &[(&str, &str)],
    mut on_progress: impl FnMut(u64, u64),
) -> AppResult<u64> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("partial");
    let mut req = HTTP.get(url).timeout(DOWNLOAD_TIMEOUT);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    on_progress(0, expected_size.unwrap_or(0));
    let response = req.send().await?.error_for_status()?;
    let total = response.content_length().or(expected_size).unwrap_or(0);
    on_progress(0, total);

    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&tmp).await?;
    let mut downloaded: u64 = 0;
    let mut last_ui = Instant::now() - Duration::from_millis(200);

    loop {
        let next = tokio::time::timeout(STALL, stream.next())
            .await
            .map_err(|_| AppError::msg("Download stalled. Check the network and try again."))?;
        let Some(chunk) = next else { break };
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).await?;
        if last_ui.elapsed() >= Duration::from_millis(120) {
            on_progress(downloaded, total);
            last_ui = Instant::now();
        }
    }
    file.flush().await?;
    drop(file);
    if dest.exists() {
        let _ = std::fs::remove_file(dest);
    }
    std::fs::rename(&tmp, dest)?;
    on_progress(downloaded, if total > 0 { total } else { downloaded });
    Ok(downloaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_api_asset_url() {
        let api = "https://api.github.com/repos/erlang/otp/releases/assets/1";
        let browser = "https://github.com/erlang/otp/releases/download/OTP-28.0/otp_win64_28.0.zip";
        assert_eq!(github_asset_url(api, browser), api);
        assert_eq!(github_asset_url("", browser), browser);
    }

    #[test]
    fn percent_is_zero_without_total() {
        assert_eq!(percent(100, 0), 0);
        assert_eq!(percent(50, 100), 50);
    }
}
