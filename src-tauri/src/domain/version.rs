//! Lightweight version parsers for Elixir (`1.20.3`) and OTP (`28.5.0.4`).
//!
//! OTP versions are not SemVer: they often have a fourth numeric component.
//! Comparison is numeric, left to right, missing parts treated as zero.

use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

/// Parsed Elixir version (`major.minor.patch` plus optional pre-release tag).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElixirVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    /// Pre-release label such as `rc.0`. Empty for stable releases.
    pub pre: Option<String>,
}

impl ElixirVersion {
    /// Parse strings like `1.20.3`, `v1.20.3`, or `1.19.0-rc.0`.
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim().trim_start_matches('v');
        let (core, pre) = match trimmed.split_once('-') {
            Some((core, pre)) => (core, Some(pre.to_string())),
            None => (trimmed, None),
        };
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().unwrap_or(0);
        Some(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    pub fn is_prerelease(&self) -> bool {
        self.pre.is_some()
    }
}

/// Mix `elixir:` requirement (`~> 1.15`, `~> 1.15.0`, `>= 1.14.0`).
pub fn elixir_satisfies(req: &str, ver: &ElixirVersion) -> bool {
    let req = req.trim();
    if req.is_empty() {
        return true;
    }
    req.split(" or ").any(|clause| clause_satisfies(clause.trim(), ver))
}

fn clause_satisfies(clause: &str, ver: &ElixirVersion) -> bool {
    let c = clause.trim().trim_end_matches(',');
    if let Some(rest) = c.strip_prefix("~>") {
        let Some(min) = ElixirVersion::parse(rest.trim()) else {
            return true;
        };
        let dots = rest.trim().split('.').filter(|p| !p.is_empty()).count();
        let max = if dots >= 3 {
            ElixirVersion {
                major: min.major,
                minor: min.minor + 1,
                patch: 0,
                pre: None,
            }
        } else {
            ElixirVersion {
                major: min.major + 1,
                minor: 0,
                patch: 0,
                pre: None,
            }
        };
        ver >= &min && ver < &max
    } else if let Some(rest) = c.strip_prefix(">=") {
        ElixirVersion::parse(rest.trim()).map(|min| ver >= &min).unwrap_or(true)
    } else if let Some(rest) = c.strip_prefix("==") {
        ElixirVersion::parse(rest.trim()).map(|min| ver == &min).unwrap_or(true)
    } else if let Some(parsed) = ElixirVersion::parse(c.trim_start_matches('v')) {
        ver.major == parsed.major && ver.minor == parsed.minor
    } else {
        true
    }
}

impl Display for ElixirVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.pre {
            Some(pre) => write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, pre),
            None => write!(f, "{}.{}.{}", self.major, self.minor, self.patch),
        }
    }
}

impl PartialOrd for ElixirVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ElixirVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}

/// Parsed OTP version with an optional fourth component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtpVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub extra: u32,
}

impl OtpVersion {
    /// Parse strings like `28.4`, `OTP-28.5.0.4`, or `27.3.4.16`.
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input
            .trim()
            .trim_start_matches("OTP-")
            .trim_start_matches("otp-")
            .trim_start_matches('v');
        let mut nums = trimmed.split('.').filter_map(|p| p.parse::<u32>().ok());
        let major = nums.next()?;
        Some(Self {
            major,
            minor: nums.next().unwrap_or(0),
            patch: nums.next().unwrap_or(0),
            extra: nums.next().unwrap_or(0),
        })
    }
}

impl Display for OtpVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.extra > 0 {
            write!(
                f,
                "{}.{}.{}.{}",
                self.major, self.minor, self.patch, self.extra
            )
        } else if self.patch > 0 {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        } else {
            write!(f, "{}.{}", self.major, self.minor)
        }
    }
}

impl PartialOrd for OtpVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OtpVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch, self.extra).cmp(&(
            other.major,
            other.minor,
            other.patch,
            other.extra,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_elixir_stable_and_rc() {
        let v = ElixirVersion::parse("v1.20.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 20);
        assert_eq!(v.patch, 3);
        assert!(!v.is_prerelease());

        let rc = ElixirVersion::parse("1.19.0-rc.0").unwrap();
        assert!(rc.is_prerelease());
        assert!(v > rc);
    }

    #[test]
    fn tilde_req_allows_next_minor_but_not_next_major() {
        let v15 = ElixirVersion::parse("1.15.7").unwrap();
        let v16 = ElixirVersion::parse("1.16.0").unwrap();
        let v20 = ElixirVersion::parse("1.20.3").unwrap();
        assert!(elixir_satisfies("~> 1.15", &v15));
        assert!(elixir_satisfies("~> 1.15", &v16));
        assert!(elixir_satisfies("~> 1.15", &v20));
        assert!(!elixir_satisfies("~> 1.15.0", &v16));
        assert!(elixir_satisfies("~> 1.15.0", &v15));
        assert!(!elixir_satisfies("~> 1.18", &v15));
    }

    #[test]
    fn parses_otp_four_component() {
        let v = OtpVersion::parse("OTP-28.5.0.4").unwrap();
        assert_eq!(v.major, 28);
        assert_eq!(v.extra, 4);
        assert!(OtpVersion::parse("28.5.0.5").unwrap() > v);
    }
}
