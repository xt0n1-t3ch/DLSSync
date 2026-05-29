//! Parsing the local Windows DriverStore via `pnputil /enum-drivers`.
//!
//! Windows Update returns only the single newest applicable driver per device,
//! and WMI reports only the currently-installed version. The DriverStore is the
//! one no-scrape source of *previously-installed* (superseded) versions still
//! cached on disk — so it backs the "older versions" column for the System &
//! Components view. Packages that share an `original_name` (e.g. `nahimicv3.inf`)
//! are versions of the same driver; the newest is whichever ranks highest by
//! [`crate::is_newer`].

use crate::is_newer;
use std::collections::BTreeMap;

/// One third-party driver package in the local DriverStore.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DriverStorePackage {
    /// Store-assigned published name, e.g. `oem28.inf`.
    pub published_name: String,
    /// The INF's original (vendor) name, e.g. `nahimicv3.inf` — the stable key
    /// that ties multiple store versions of one driver together.
    pub original_name: String,
    pub provider: String,
    pub class: String,
    /// Four-part driver version, e.g. `2.2.0.134`.
    pub version: String,
    /// ISO `YYYY-MM-DD` parsed from the `MM/DD/YYYY` `pnputil` date, when present.
    pub date: Option<String>,
}

fn field<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    let (key, value) = line.split_once(':')?;
    if key.trim().eq_ignore_ascii_case(label) {
        Some(value.trim())
    } else {
        None
    }
}

/// `pnputil` prints the version line as `MM/DD/YYYY V.V.V.V`. Split into an ISO
/// date + the version token; either part may be absent.
fn split_driver_version(raw: &str) -> (Option<String>, String) {
    let mut parts = raw.split_whitespace();
    let first = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() {
        return (None, first.to_string());
    }
    (mdy_to_iso(first), rest.join(" "))
}

fn mdy_to_iso(mdy: &str) -> Option<String> {
    let p: Vec<&str> = mdy.split('/').collect();
    if p.len() != 3 {
        return None;
    }
    let (m, d, y) = (p[0], p[1], p[2]);
    if m.len() > 2
        || d.len() > 2
        || y.len() != 4
        || !p.iter().all(|s| s.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some(format!("{y}-{m:0>2}-{d:0>2}"))
}

/// Parse the full text of `pnputil /enum-drivers` into one entry per package.
/// Unknown/localized field labels are simply skipped, so a non-English Windows
/// yields fewer fields rather than a crash.
pub fn parse_enum_drivers(output: &str) -> Vec<DriverStorePackage> {
    let mut out: Vec<DriverStorePackage> = Vec::new();
    let mut cur: Option<DriverStorePackage> = None;
    let push = |cur: &mut Option<DriverStorePackage>, out: &mut Vec<DriverStorePackage>| {
        if let Some(pkg) = cur.take() {
            if !pkg.published_name.is_empty() {
                out.push(pkg);
            }
        }
    };
    for line in output.lines() {
        let line = line.trim();
        if let Some(v) = field(line, "Published Name") {
            push(&mut cur, &mut out);
            cur = Some(DriverStorePackage {
                published_name: v.to_string(),
                ..Default::default()
            });
        } else if let Some(pkg) = cur.as_mut() {
            if let Some(v) = field(line, "Original Name") {
                pkg.original_name = v.to_string();
            } else if let Some(v) = field(line, "Provider Name") {
                pkg.provider = v.to_string();
            } else if let Some(v) = field(line, "Class Name") {
                pkg.class = v.to_string();
            } else if let Some(v) = field(line, "Driver Version") {
                let (date, version) = split_driver_version(v);
                pkg.date = date;
                pkg.version = version;
            }
        }
    }
    push(&mut cur, &mut out);
    out
}

/// Group packages by `original_name`, newest-first within each group, so the UI
/// can show "v2.2.0.134 (current) · v2.2.0.130" for one driver. Empty/unknown
/// original names are dropped.
pub fn versions_by_original_name(
    packages: &[DriverStorePackage],
) -> BTreeMap<String, Vec<DriverStorePackage>> {
    let mut map: BTreeMap<String, Vec<DriverStorePackage>> = BTreeMap::new();
    for pkg in packages {
        if pkg.original_name.is_empty() {
            continue;
        }
        map.entry(pkg.original_name.to_ascii_lowercase())
            .or_default()
            .push(pkg.clone());
    }
    for group in map.values_mut() {
        group.sort_by(|a, b| {
            if is_newer(
                a.date.as_deref(),
                Some(&a.version),
                b.date.as_deref(),
                Some(&b.version),
            ) {
                std::cmp::Ordering::Less
            } else if is_newer(
                b.date.as_deref(),
                Some(&b.version),
                a.date.as_deref(),
                Some(&a.version),
            ) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Microsoft PnP Utility

Published Name:     oem28.inf
Original Name:      amdgpio2.inf
Provider Name:      Advanced Micro Devices, Inc
Class Name:         System
Class GUID:         {4d36e97d-e325-11ce-bfc1-08002be10318}
Driver Version:     08/20/2024 2.2.0.134
Signer Name:        Microsoft Windows Hardware Compatibility Publisher

Published Name:     oem11.inf
Original Name:      amdgpio2.inf
Provider Name:      Advanced Micro Devices, Inc
Class Name:         System
Class GUID:         {4d36e97d-e325-11ce-bfc1-08002be10318}
Driver Version:     09/15/2022 2.2.0.130
Signer Name:        Microsoft Windows Hardware Compatibility Publisher

Published Name:     oem5.inf
Original Name:      nahimicv3.inf
Provider Name:      A-Volute
Class Name:         MEDIA
Driver Version:     11/20/2025 1.1.4.0
";

    #[test]
    fn parses_each_package_block() {
        let pkgs = parse_enum_drivers(SAMPLE);
        assert_eq!(pkgs.len(), 3);
        assert_eq!(pkgs[0].published_name, "oem28.inf");
        assert_eq!(pkgs[0].original_name, "amdgpio2.inf");
        assert_eq!(pkgs[0].provider, "Advanced Micro Devices, Inc");
        assert_eq!(pkgs[0].version, "2.2.0.134");
        assert_eq!(pkgs[0].date.as_deref(), Some("2024-08-20"));
        assert_eq!(pkgs[2].original_name, "nahimicv3.inf");
        assert_eq!(pkgs[2].version, "1.1.4.0");
    }

    #[test]
    fn groups_superseded_versions_newest_first() {
        let pkgs = parse_enum_drivers(SAMPLE);
        let by_name = versions_by_original_name(&pkgs);
        let amd = by_name.get("amdgpio2.inf").expect("amd group");
        assert_eq!(amd.len(), 2);
        assert_eq!(amd[0].version, "2.2.0.134", "newest first");
        assert_eq!(amd[1].version, "2.2.0.130");
    }

    #[test]
    fn ignores_preamble_and_blank_lines() {
        assert!(parse_enum_drivers("Microsoft PnP Utility\n\n\n").is_empty());
        assert!(parse_enum_drivers("").is_empty());
    }

    #[test]
    fn version_line_without_date_keeps_version() {
        let (date, version) = split_driver_version("10.0.26100.1");
        assert_eq!(date, None);
        assert_eq!(version, "10.0.26100.1");
    }
}
