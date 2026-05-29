use crate::DriverVendor;
use serde::{Deserialize, Serialize};

/// A comparable driver version. Only `packed` is compared (`is_newer_than` =
/// strict `>`); `display` is the human label and `raw` the source string.
///
/// Packing contract, fixed at construction per vendor:
/// - NVIDIA (`nvidia`): the last 5 marketing digits, e.g. `32.0.15.7216` → `57216`
///   shown as `572.16`. A shorter digit run is kept as-is (best effort).
/// - AMD / Intel (`four_part`): the first four dot-separated lanes, each clamped to
///   `u16::MAX`, packed big-endian (`l0<<48 | l1<<32 | l2<<16 | l3`).
///
/// Unparseable or empty input yields `packed == 0` — an `Unknown` that compares
/// less than every real version. Callers must treat `0` as "version unknown",
/// not "oldest". Comparison is packed-integer only; there are no semver rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverVersion {
    pub packed: u64,
    pub display: String,
    pub raw: String,
}

const NVIDIA_MARKETING_DIGITS: usize = 5;
const VERSION_LANE_MAX: u64 = u16::MAX as u64;

impl DriverVersion {
    pub fn unknown() -> Self {
        Self {
            packed: 0,
            display: "Unknown".to_string(),
            raw: String::new(),
        }
    }

    pub fn from_installed(vendor: DriverVendor, raw: &str) -> Self {
        match vendor {
            DriverVendor::Nvidia => Self::nvidia(raw),
            _ => Self::four_part(raw),
        }
    }

    pub fn nvidia(raw: &str) -> Self {
        let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return Self {
                packed: 0,
                display: "Unknown".to_string(),
                raw: raw.to_string(),
            };
        }
        let tail = if digits.len() >= NVIDIA_MARKETING_DIGITS {
            digits[digits.len() - NVIDIA_MARKETING_DIGITS..].to_string()
        } else {
            digits
        };
        let packed = tail.parse::<u64>().unwrap_or(0);
        let display = if tail.len() == NVIDIA_MARKETING_DIGITS {
            format!("{}.{}", &tail[..3], &tail[3..])
        } else {
            tail
        };
        Self {
            packed,
            display,
            raw: raw.to_string(),
        }
    }

    /// Pack the comparison version from `raw` (e.g. the AMD Windows driver-store
    /// value that matches WMI) while showing a friendlier `display` (e.g. the public
    /// Adrenalin release "26.5.2").
    pub fn four_part_labeled(raw: &str, display: &str) -> Self {
        let packed = Self::four_part(raw).packed;
        Self {
            packed,
            display: display.to_string(),
            raw: raw.to_string(),
        }
    }

    pub fn four_part(raw: &str) -> Self {
        let lanes: Vec<u64> = raw
            .split('.')
            .map(|segment| {
                segment
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                    .unwrap_or(0)
                    .min(VERSION_LANE_MAX)
            })
            .collect();
        let lane = |index: usize| lanes.get(index).copied().unwrap_or(0);
        let packed = (lane(0) << 48) | (lane(1) << 32) | (lane(2) << 16) | lane(3);
        let display = if raw.trim().is_empty() {
            "Unknown".to_string()
        } else {
            raw.to_string()
        };
        Self {
            packed,
            display,
            raw: raw.to_string(),
        }
    }

    pub fn is_newer_than(&self, other: &Self) -> bool {
        self.packed > other.packed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_wmi_store_form_decodes_to_marketing() {
        let v = DriverVersion::nvidia("32.0.15.7216");
        assert_eq!(v.packed, 57216);
        assert_eq!(v.display, "572.16");

        let older = DriverVersion::nvidia("31.0.15.5161");
        assert_eq!(older.packed, 55161);
        assert_eq!(older.display, "551.61");
    }

    #[test]
    fn nvidia_marketing_form_matches_store_form() {
        let from_store = DriverVersion::nvidia("32.0.15.7216");
        let from_marketing = DriverVersion::nvidia("572.16");
        assert_eq!(from_store.packed, from_marketing.packed);
        assert_eq!(from_marketing.display, "572.16");
    }

    #[test]
    fn nvidia_empty_is_unknown() {
        let v = DriverVersion::nvidia("");
        assert_eq!(v.packed, 0);
        assert_eq!(v.display, "Unknown");
    }

    #[test]
    fn nvidia_short_digit_run_is_best_effort_not_panic() {
        let v = DriverVersion::nvidia("3.2");
        assert_eq!(v.packed, 32);
        assert_eq!(v.display, "32");
    }

    #[test]
    fn four_part_packs_intel_and_amd_store_versions() {
        let intel = DriverVersion::four_part("32.0.101.8801");
        let amd = DriverVersion::four_part("32.0.31007.1017");
        assert!(intel.packed > 0);
        assert_eq!(intel.display, "32.0.101.8801");
        assert_eq!(amd.display, "32.0.31007.1017");

        let intel_newer = DriverVersion::four_part("32.0.101.8810");
        assert!(intel_newer.is_newer_than(&intel));
    }

    #[test]
    fn four_part_ignores_segments_beyond_four_and_empties() {
        let extra = DriverVersion::four_part("1.2.3.4.5");
        let four = DriverVersion::four_part("1.2.3.4");
        assert_eq!(extra.packed, four.packed);

        let empty = DriverVersion::four_part("");
        assert_eq!(empty.packed, 0);
        assert_eq!(empty.display, "Unknown");
    }

    #[test]
    fn four_part_saturates_oversized_lane() {
        let huge = DriverVersion::four_part("999999.0.0.0");
        let max_lane = DriverVersion::four_part("65535.0.0.0");
        assert_eq!(huge.packed, max_lane.packed);
    }

    #[test]
    fn from_installed_routes_by_vendor() {
        assert_eq!(
            DriverVersion::from_installed(DriverVendor::Nvidia, "32.0.15.7216").packed,
            57216
        );
        assert_eq!(
            DriverVersion::from_installed(DriverVendor::Intel, "32.0.101.8801").display,
            "32.0.101.8801"
        );
    }
}
