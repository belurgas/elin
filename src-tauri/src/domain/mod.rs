//! Domain types: version numbers, compatibility, and catalog models.

pub mod compatibility;
pub mod version;

use serde::{Deserialize, Serialize};

pub use compatibility::{compatible_otp_majors, recommended_otp_major, versions_are_compatible};
pub use version::{elixir_satisfies, ElixirVersion, OtpVersion};

/// A published Elixir release together with the OTP majors it can run on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElixirRelease {
    pub version: String,
    pub otp_majors: Vec<u32>,
    pub is_latest: bool,
    pub is_prerelease: bool,
}

/// A published Erlang/OTP release that has a Windows download.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtpRelease {
    pub version: String,
    pub major: u32,
    pub zip_url: Option<String>,
    pub exe_url: Option<String>,
    pub is_latest: bool,
    pub is_prerelease: bool,
    pub published_at: Option<String>,
}

/// Combined live catalog shown in the installer UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCatalog {
    pub elixir: Vec<ElixirRelease>,
    pub otp: Vec<OtpRelease>,
    pub latest_elixir: Option<String>,
    pub latest_otp: Option<String>,
    pub recommended_elixir: Option<String>,
    pub recommended_otp: Option<String>,
    pub fetched_at: String,
    pub source: String,
}

/// A locally installed Elixir + OTP pair managed by Elin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPair {
    pub elixir: String,
    pub otp: String,
    pub elixir_path: String,
    pub otp_path: String,
    pub is_active: bool,
}
