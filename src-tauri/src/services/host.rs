//! OS / arch facts used by PATH, install, catalog, and process spawning.
//!
//! Windows keeps the existing elixir-install zip layout. macOS uses
//! erlef/otp_builds tarballs. Linux uses Hex Bob Ubuntu OTP builds (the same
//! source as `elixir-lang.org/install.sh`) plus the official `Install` script.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    TarGz,
}

pub fn path_sep() -> char {
    if cfg!(windows) { ';' } else { ':' }
}

pub fn split_path(path_var: &str) -> Vec<&str> {
    path_var.split(path_sep()).collect()
}

pub fn join_path(parts: &[String]) -> String {
    parts.join(&path_sep().to_string())
}

/// Canonical comparison key for a filesystem path (slash, no trailing slash).
pub fn path_key(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

pub fn elixir_script() -> &'static str {
    if cfg!(windows) { "elixir.bat" } else { "elixir" }
}

pub fn mix_script() -> &'static str {
    if cfg!(windows) { "mix.bat" } else { "mix" }
}

pub fn iex_script() -> &'static str {
    if cfg!(windows) { "iex.bat" } else { "iex" }
}

pub fn erl_binary() -> &'static str {
    if cfg!(windows) { "erl.exe" } else { "erl" }
}

pub fn elixir_cmd(elixir_bin: &Path) -> PathBuf {
    elixir_bin.join(elixir_script())
}

pub fn mix_cmd(elixir_bin: &Path) -> PathBuf {
    elixir_bin.join(mix_script())
}

pub fn iex_cmd(elixir_bin: &Path) -> PathBuf {
    elixir_bin.join(iex_script())
}

pub fn find_script(dir: &Path, stem: &str) -> Option<PathBuf> {
    let names = if cfg!(windows) {
        [
            format!("{stem}.bat"),
            format!("{stem}.cmd"),
            format!("{stem}.exe"),
            stem.to_string(),
        ]
    } else {
        [stem.to_string(), format!("{stem}.sh"), format!("{stem}.bat"), format!("{stem}.exe")]
    };
    names.into_iter().map(|n| dir.join(n)).find(|p| p.exists())
}

pub fn otp_archive_kind() -> ArchiveKind {
    if cfg!(windows) {
        ArchiveKind::Zip
    } else {
        ArchiveKind::TarGz
    }
}

pub fn otp_archive_kind_from_url(url: &str) -> ArchiveKind {
    let lower = url.to_ascii_lowercase();
    if lower.contains(".tar.gz") || lower.ends_with(".tgz") {
        ArchiveKind::TarGz
    } else if cfg!(not(windows)) && !lower.contains(".zip") {
        otp_archive_kind()
    } else {
        ArchiveKind::Zip
    }
}

/// CPU arch as used by Hex Bob (`amd64` / `arm64`).
pub fn bob_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" | "x86" => "amd64",
        "aarch64" => "arm64",
        other if other.contains("arm") => "arm64",
        _ => "amd64",
    }
}

/// Target triple fragment used by erlef/otp_builds on macOS.
pub fn darwin_otp_target() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "aarch64-apple-darwin",
        _ => "x86_64-apple-darwin",
    }
}

/// Ubuntu LTS folder Hex Bob actually publishes (`20.04` / `22.04` / `24.04`).
///
/// Non-Ubuntu distros get 22.04: older glibc, so the tarball still runs on
/// newer Debian/Fedora/Arch. Ubuntu itself maps to the matching LTS.
pub fn linux_bob_ubuntu() -> &'static str {
    let Some((id, version)) = os_release() else {
        return "22.04";
    };
    if id != "ubuntu" && id != "linuxmint" && id != "pop" && id != "elementary" {
        return "22.04";
    }
    match version.split('.').next().unwrap_or("") {
        "20" | "21" => "20.04",
        "22" | "23" => "22.04",
        _ => "24.04",
    }
}

fn os_release() -> Option<(String, String)> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    parse_os_release(&text)
}

pub fn parse_os_release(text: &str) -> Option<(String, String)> {
    let mut id = None;
    let mut version = None;
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().trim_matches('"').to_ascii_lowercase();
        match k {
            "ID" => id = Some(v),
            "VERSION_ID" => version = Some(v),
            _ => {}
        }
    }
    Some((id?, version.unwrap_or_default()))
}

/// Hex Bob OTP tarball for this Linux machine.
pub fn linux_otp_url(version: &str) -> String {
    let file = if version.starts_with("master") || version.starts_with("maint") {
        format!("{version}.tar.gz")
    } else {
        format!("OTP-{version}.tar.gz")
    };
    format!(
        "https://builds.hex.pm/builds/otp/{}/{}/{}",
        bob_arch(),
        format!("ubuntu-{}", linux_bob_ubuntu()),
        file
    )
}

/// erlef/otp_builds tarball for this macOS machine.
pub fn darwin_otp_url(version: &str) -> String {
    format!(
        "https://github.com/erlef/otp_builds/releases/download/OTP-{version}/otp-{}.tar.gz",
        darwin_otp_target()
    )
}

pub fn catalog_source() -> &'static str {
    if cfg!(windows) {
        "builds.hex.pm + github.com/erlang/otp"
    } else if cfg!(target_os = "macos") {
        "builds.hex.pm + github.com/erlef/otp_builds"
    } else {
        "builds.hex.pm (Elixir + Ubuntu OTP)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_kind_from_url() {
        assert_eq!(otp_archive_kind_from_url("https://x/otp_win64_27.zip"), ArchiveKind::Zip);
        assert_eq!(
            otp_archive_kind_from_url("https://x/OTP-27.1.2.tar.gz"),
            ArchiveKind::TarGz
        );
        assert_eq!(otp_archive_kind(), if cfg!(windows) { ArchiveKind::Zip } else { ArchiveKind::TarGz });
    }

    #[test]
    fn path_key_unifies_slashes() {
        assert_eq!(path_key(r"D:\a\b\"), path_key("D:/a/b"));
        assert_eq!(path_key("/Users/me/.elixir-install/installs/"), path_key("/Users/me/.elixir-install/installs"));
    }

    #[test]
    fn parses_ubuntu_os_release() {
        let sample = "ID=ubuntu\nVERSION_ID=\"22.04\"\n";
        assert_eq!(parse_os_release(sample), Some(("ubuntu".into(), "22.04".into())));
    }

    #[test]
    fn linux_url_uses_bob_layout() {
        let url = linux_otp_url("27.1.2");
        assert!(url.contains("builds.hex.pm/builds/otp/"));
        assert!(url.ends_with("/OTP-27.1.2.tar.gz"));
        assert!(url.contains("/ubuntu-"));
    }

    #[test]
    fn darwin_url_uses_erlef() {
        let url = darwin_otp_url("27.1.2");
        assert!(url.contains("erlef/otp_builds"));
        assert!(url.contains("OTP-27.1.2"));
        assert!(url.ends_with(".tar.gz"));
    }
}
