//! Official Elixir ↔ Erlang/OTP compatibility table.
//!
//! Source: https://hexdocs.pm/elixir/compatibility-and-deprecations.html
//! Keep the ranges conservative (the last three OTP majors per Elixir minor).
//! Patch-level exceptions (for example OTP 26 on Elixir 1.14.5) are encoded
//! as extra majors on the family — beginners should still get a working pair.

use super::version::{ElixirVersion, OtpVersion};

/// Inclusive OTP major range supported by an Elixir minor family.
struct FamilySupport {
    elixir_minor: u32,
    otp_from: u32,
    otp_to: u32,
}

/// Compatibility rows for Elixir 1.x minor families.
///
/// Newer families not listed yet inherit the latest row so Elin still offers
/// a pairing instead of blocking the user after a brand-new Elixir release.
const TABLE: &[FamilySupport] = &[
    FamilySupport {
        elixir_minor: 20,
        otp_from: 27,
        otp_to: 29,
    },
    FamilySupport {
        elixir_minor: 19,
        otp_from: 26,
        otp_to: 28,
    },
    FamilySupport {
        elixir_minor: 18,
        otp_from: 25,
        otp_to: 27,
    },
    FamilySupport {
        elixir_minor: 17,
        otp_from: 25,
        otp_to: 27,
    },
    FamilySupport {
        elixir_minor: 16,
        otp_from: 24,
        otp_to: 26,
    },
    FamilySupport {
        elixir_minor: 15,
        otp_from: 24,
        otp_to: 26,
    },
    FamilySupport {
        elixir_minor: 14,
        otp_from: 23,
        otp_to: 26,
    },
    FamilySupport {
        elixir_minor: 13,
        otp_from: 22,
        otp_to: 25,
    },
    FamilySupport {
        elixir_minor: 12,
        otp_from: 22,
        otp_to: 24,
    },
    FamilySupport {
        elixir_minor: 11,
        otp_from: 21,
        otp_to: 24,
    },
    FamilySupport {
        elixir_minor: 10,
        otp_from: 21,
        otp_to: 23,
    },
    FamilySupport {
        elixir_minor: 9,
        otp_from: 20,
        otp_to: 22,
    },
];

fn family_row(elixir_minor: u32) -> &'static FamilySupport {
    TABLE
        .iter()
        .find(|row| row.elixir_minor == elixir_minor)
        .or_else(|| TABLE.first())
        .unwrap()
}

/// OTP majors that a given Elixir version is documented to support.
pub fn compatible_otp_majors(elixir: &ElixirVersion) -> Vec<u32> {
    let row = family_row(elixir.minor);
    (row.otp_from..=row.otp_to).collect()
}

/// Highest OTP major that is both compatible and present in `available`.
pub fn recommended_otp_major(elixir: &ElixirVersion, available: &[u32]) -> Option<u32> {
    let compatible = compatible_otp_majors(elixir);
    available
        .iter()
        .copied()
        .filter(|major| compatible.contains(major))
        .max()
}

/// True when the pair is inside the documented support window.
pub fn versions_are_compatible(elixir: &ElixirVersion, otp: &OtpVersion) -> bool {
    compatible_otp_majors(elixir).contains(&otp.major)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elixir_1_20_supports_otp_27_to_29() {
        let elixir = ElixirVersion::parse("1.20.3").unwrap();
        assert_eq!(compatible_otp_majors(&elixir), vec![27, 28, 29]);
        assert!(versions_are_compatible(
            &elixir,
            &OtpVersion::parse("28.4").unwrap()
        ));
        assert!(!versions_are_compatible(
            &elixir,
            &OtpVersion::parse("26.2").unwrap()
        ));
    }

    #[test]
    fn recommends_highest_available_compatible_major() {
        let elixir = ElixirVersion::parse("1.18.4").unwrap();
        assert_eq!(recommended_otp_major(&elixir, &[24, 25, 26, 27, 28]), Some(27));
    }
}
