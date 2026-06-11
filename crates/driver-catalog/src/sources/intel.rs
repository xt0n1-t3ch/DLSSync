use super::DriverSource;
use crate::consts::intel as c;
use crate::{
    DeviceClass, DeviceId, DriverError, DriverRelease, DriverVendor, DriverVersion, OsFamily,
    OsTarget, ReleaseChannel,
};
use async_trait::async_trait;
use std::io::Read;

pub struct IntelGpuSource;

fn version_token(version: &str) -> &str {
    version.split_whitespace().next().unwrap_or_default()
}

fn parse_iso_date(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(raw.trim())
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

/// Intel's `DisplayReleaseDate` is normally RFC3339 (`2026-05-15T00:00:00Z`), but
/// a catalog entry occasionally carries a bare `YYYY-MM-DD`. Accept both so a real
/// graphics driver is never dropped purely over a date-format quirk.
fn parse_release_date(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{NaiveDate, TimeZone, Utc};
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(dt) = parse_iso_date(trimmed) {
        return Some(dt);
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|naive| Utc.from_utc_datetime(&naive))
}

/// `VEN_8086&DEV_XXXX` exactly as Intel writes it in `Components[].DetectionValues`.
fn intel_hardware_id(device_id: u16) -> String {
    format!("VEN_{:04X}&DEV_{device_id:04X}", c::PCI_VENDOR_ID)
}

/// True when the configuration ships a graphics-class component. The DSA catalog
/// mixes Wi-Fi, Bluetooth, BIOS, LAN and NPU entries in with the GPU drivers, so
/// every other category must be ignored.
fn entry_is_graphics(entry: &serde_json::Value) -> bool {
    entry["Components"]
        .as_array()
        .is_some_and(|comps| comps.iter().any(|comp| comp["Category"] == "Graphics"))
}

/// True when any component's `DetectionValues` lists this exact PCI device id.
/// Intel keys each driver package to the hardware ids it actually supports — the
/// only correct way to tell an integrated UHD/Iris Xe driver apart from the Arc
/// package and never serve a mismatched installer (exit code 8).
fn entry_supports_device(entry: &serde_json::Value, device_id: u16) -> bool {
    let needle = intel_hardware_id(device_id);
    entry["Components"].as_array().is_some_and(|comps| {
        comps.iter().any(|comp| {
            comp["DetectionValues"].as_array().is_some_and(|vals| {
                vals.iter().any(|v| {
                    v.as_str()
                        .is_some_and(|s| s.to_ascii_uppercase().starts_with(&needle))
                })
            })
        })
    })
}

/// Whether a catalog entry targets a specific Windows version or is OS-neutral.
///
/// Intel ships separate packages for Windows 10 and Windows 11 for some GPU
/// families (notably Arc on Win10 vs Win11 with DirectStorage differences). The
/// entry `Name` is the only reliable signal; a `SupportedOS` field is absent in
/// the current DSA schema. An entry without either keyword is treated as neutral
/// and served to every OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OsSpecificity {
    Win10Only,
    Win11Only,
    Neutral,
}

fn entry_os_specificity(entry: &serde_json::Value) -> OsSpecificity {
    let name = entry["Name"].as_str().unwrap_or_default();
    let has_win10 = name.contains(c::OS_NAME_WIN10);
    let has_win11 = name.contains(c::OS_NAME_WIN11);
    match (has_win10, has_win11) {
        (true, false) => OsSpecificity::Win10Only,
        (false, true) => OsSpecificity::Win11Only,
        _ => OsSpecificity::Neutral,
    }
}

/// True when an entry is compatible with `os`.
///
/// A Win10 user must never receive a Win11-only package. An entry with no
/// OS qualifier in its `Name` (neutral) is served to both. When both
/// Win10-specific and Win11-specific packages exist for the same device, the
/// caller (see `resolve_history`) keeps only the right-OS set and drops the
/// neutral ones so a more-specific installer is always preferred.
fn entry_is_os_compatible(entry: &serde_json::Value, os: &OsTarget) -> bool {
    match entry_os_specificity(entry) {
        OsSpecificity::Neutral => true,
        OsSpecificity::Win10Only => os.family == OsFamily::Windows10X64,
        OsSpecificity::Win11Only => os.family == OsFamily::Windows11X64,
    }
}

fn entry_to_release(entry: &serde_json::Value) -> Option<DriverRelease> {
    let token = version_token(entry["Version"].as_str().unwrap_or_default());
    if !token.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let date = parse_release_date(entry["DisplayReleaseDate"].as_str().unwrap_or_default());
    let file0 = entry["Files"].as_array().and_then(|f| f.first());
    let download_url = file0
        .and_then(|f| f["Url"].as_str())
        .unwrap_or_default()
        .to_string();
    if download_url.is_empty() {
        return None;
    }
    let is_beta = entry["IsBeta"].as_bool().unwrap_or(false);
    Some(DriverRelease {
        vendor: DriverVendor::Intel,
        version: DriverVersion::four_part(token),
        channel: if is_beta {
            ReleaseChannel::Beta
        } else {
            ReleaseChannel::Stable
        },
        display_version: None,
        is_beta,
        download_url,
        size_bytes: file0.and_then(|f| f["Size"].as_u64()).unwrap_or(0),
        signature_subject: c::PUBLISHER_SUBJECT.to_string(),
        released_at: date,
        release_notes_url: entry["Url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        changelog: None,
    })
}

/// Every graphics driver in the catalog that lists this device id, newest-first,
/// deduped by version. Spans the device's current group plus any sibling
/// "Historical" group that still lists it. The first element is the recommended
/// `latest`. An unknown/zero device id resolves to nothing rather than guessing.
///
/// OS-aware: when the catalog contains entries specific to the user's Windows
/// version (e.g. "Windows 11" in the `Name`), only those are returned and
/// neutral entries are suppressed. When only neutral entries exist they are
/// returned as-is. Entries whose `Name` targets the *other* OS are always
/// excluded so a Win10 user never receives a Win11-only package and vice-versa.
pub fn resolve_history(
    json: &str,
    device_id: u16,
    os: &OsTarget,
    limit: usize,
) -> Result<Vec<DriverRelease>, DriverError> {
    if device_id == 0 {
        return Ok(Vec::new());
    }
    let root: serde_json::Value =
        serde_json::from_str(json).map_err(|e| DriverError::Parse(e.to_string()))?;
    let entries = root
        .as_array()
        .ok_or_else(|| DriverError::Parse("software-configurations: expected array".into()))?;

    // Collect entries that match device + graphics category + are not from the wrong OS.
    let candidates: Vec<&serde_json::Value> = entries
        .iter()
        .filter(|e| {
            entry_is_graphics(e)
                && entry_supports_device(e, device_id)
                && entry_is_os_compatible(e, os)
        })
        .collect();

    // If any OS-specific package for our OS is present, suppress neutral entries
    // so the more-specific installer wins (e.g. a Win10-labelled package beats a
    // generic "Windows" package for the same device).
    let target_specificity = match os.family {
        OsFamily::Windows10X64 => OsSpecificity::Win10Only,
        OsFamily::Windows11X64 => OsSpecificity::Win11Only,
    };
    let has_specific = candidates
        .iter()
        .any(|e| entry_os_specificity(e) == target_specificity);

    let filtered = candidates
        .iter()
        .filter(|e| !has_specific || entry_os_specificity(e) == target_specificity);

    let mut releases: Vec<DriverRelease> = filtered.filter_map(|e| entry_to_release(e)).collect();
    releases.sort_by_key(|r| std::cmp::Reverse(r.released_at));
    let mut seen = std::collections::BTreeSet::new();
    releases.retain(|r| seen.insert(r.version.display.clone()));
    releases.truncate(limit);
    Ok(releases)
}

/// The newest graphics driver that actually supports this exact device on `os` —
/// the correct integrated / Arc / legacy package, carrying its real download URL
/// and its own release-notes page (not the generic Arc landing page).
pub fn resolve_release(
    json: &str,
    device_id: u16,
    os: &OsTarget,
) -> Result<Option<DriverRelease>, DriverError> {
    Ok(resolve_history(json, device_id, os, usize::MAX)?
        .into_iter()
        .next())
}

fn extract_configurations(zip_bytes: &[u8]) -> Result<String, DriverError> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| DriverError::Parse(format!("dsa zip: {e}")))?;
    let mut file = archive
        .by_name(c::SOFTWARE_CONFIGURATIONS_FILE)
        .map_err(|e| DriverError::Parse(format!("dsa entry: {e}")))?;
    let mut json = String::new();
    file.read_to_string(&mut json)
        .map_err(|e| DriverError::Parse(format!("dsa read: {e}")))?;
    Ok(json)
}

#[async_trait]
impl DriverSource for IntelGpuSource {
    fn id(&self) -> &'static str {
        "intel-gpu"
    }

    fn supports(&self, device: &DeviceId) -> bool {
        device.class == DeviceClass::Gpu && device.vendor == DriverVendor::Intel
    }

    async fn latest(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        os: &OsTarget,
    ) -> Result<Option<DriverRelease>, DriverError> {
        let json = fetch_configurations_json(client).await?;
        resolve_release(&json, device.pci_device_id, os)
    }

    async fn history(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        os: &OsTarget,
        limit: usize,
    ) -> Result<Vec<DriverRelease>, DriverError> {
        let json = fetch_configurations_json(client).await?;
        resolve_history(&json, device.pci_device_id, os, limit)
    }
}

async fn fetch_configurations_json(client: &reqwest::Client) -> Result<String, DriverError> {
    let bytes = client
        .get(c::DSA_CATALOG_URL)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    extract_configurations(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OsFamily;

    fn device(device_id: u16) -> DeviceId {
        DeviceId {
            class: DeviceClass::Gpu,
            vendor: DriverVendor::Intel,
            pci_vendor_id: 0x8086,
            pci_device_id: device_id,
            model: "intel graphics".into(),
        }
    }

    fn win10() -> OsTarget {
        OsTarget {
            family: OsFamily::Windows10X64,
            dch: false,
        }
    }

    fn win11() -> OsTarget {
        OsTarget {
            family: OsFamily::Windows11X64,
            dch: false,
        }
    }

    #[test]
    fn supports_only_intel_gpus() {
        let source = IntelGpuSource;
        assert!(source.supports(&device(0xE20B)));
        let mut nvidia = device(0x2705);
        nvidia.vendor = DriverVendor::Nvidia;
        assert!(!source.supports(&nvidia));
    }

    #[test]
    fn intel_hardware_id_is_uppercase_padded() {
        assert_eq!(intel_hardware_id(0xE20B), "VEN_8086&DEV_E20B");
        assert_eq!(intel_hardware_id(0x9A49), "VEN_8086&DEV_9A49");
        assert_eq!(intel_hardware_id(0x0B), "VEN_8086&DEV_000B");
    }

    /// Catalog fixture with only OS-neutral entries (no "Windows 10" / "Windows
    /// 11" in any Name). All existing tests use this to ensure neutral-only
    /// catalogs are unaffected by the OS filter.
    const FIXTURE: &str = r#"[
        {"Id":919751,"GroupId":"785597","Version":"32.0.101.8801 WHQL Certified","DisplayReleaseDate":"2026-05-15T00:00:00Z","Name":"Intel Arc Graphics - Windows","IsBeta":false,"Url":"https://www.intel.com/.../785597/919751/intel-arc-graphics-windows.html","Files":[{"Url":"https://downloadmirror.intel.com/919751/gfx_win_101.8801.exe","Size":823031088}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B","VEN_8086&DEV_56A0","VEN_8086&DEV_7D55&SUBSYS_00000000"]}]},
        {"Id":919765,"GroupId":"857252","Version":"32.0.101.8626 WHQL Certified","DisplayReleaseDate":"2026-03-17T00:00:00Z","Name":"Historical Intel Arc Graphics Drivers","Url":null,"Files":[{"Url":"https://downloadmirror.intel.com/x/gfx_win_101.8626.exe","Size":1064962136}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B","VEN_8086&DEV_56A0"]}]},
        {"Id":915569,"GroupId":"864990","Version":"32.0.101.7085","DisplayReleaseDate":"2026-03-18T00:00:00Z","Name":"Intel 11th-14th Gen Processor Graphics - Windows","Url":"https://www.intel.com/.../864990/intel-11th-14th-gen-processor-graphics-windows.html","Files":[{"Url":"https://downloadmirror.intel.com/y/gfx_win_101.7085.exe","Size":500000000}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_9A49","VEN_8086&DEV_46A6"]}]},
        {"Id":916846,"GroupId":"776137","Version":"31.0.101.2141","DisplayReleaseDate":"2026-04-06T00:00:00Z","Name":"Intel 7th-10th Gen Processor Graphics - Windows","Url":"https://www.intel.com/.../776137/intel-7th-10th-gen-processor-graphics-windows.html","Files":[{"Url":"https://downloadmirror.intel.com/z/win64_2141.exe","Size":400000000}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_9BC4","VEN_8086&DEV_3E92"]}]},
        {"Id":764512,"GroupId":"762755","Version":"31.0.101.2115","DisplayReleaseDate":"2022-12-29T00:00:00Z","Name":"Intel 6th Gen Processor Graphics - Windows","Url":"https://www.intel.com/.../762755/intel-6th-gen-processor-graphics-windows.html","Files":[{"Url":"https://downloadmirror.intel.com/w/win64_2115.exe","Size":300000000}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_1912","VEN_8086&DEV_9BC4"]}]},
        {"Id":30195,"GroupId":"18799","Version":"15.45.34.5174","DisplayReleaseDate":"2021-02-05T00:00:00Z","Name":"Intel Graphics Driver 15.45","Url":"https://www.intel.com/.../18799/intel-graphics-driver-for-windows-15-45.html","Files":[{"Url":"https://downloadmirror.intel.com/v/win64_15.45.exe","Size":200000000}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_1912"]}]},
        {"Id":918237,"GroupId":"19351","Version":"24.40.0","DisplayReleaseDate":"2026-04-28T00:00:00Z","Name":"Intel Wireless Wi-Fi Drivers","Url":"https://www.intel.com/.../19351/wifi.html","Files":[{"Url":"https://downloadmirror.intel.com/u/wifi.exe","Size":100}],"Components":[{"Category":"Wireless","DetectionValues":["VEN_8086&DEV_ABCD","VEN_8086&DEV_E20B"]}]}
    ]"#;

    #[test]
    fn arc_b_series_resolves_to_current_arc_client_driver() {
        let r = resolve_release(FIXTURE, 0xE20B, &win11())
            .unwrap()
            .expect("release");
        assert_eq!(r.version.display, "32.0.101.8801");
        assert_eq!(
            r.download_url,
            "https://downloadmirror.intel.com/919751/gfx_win_101.8801.exe"
        );
        assert!(r.release_notes_url.as_deref().unwrap().contains("785597"));
        assert_eq!(r.signature_subject, "Intel Corporation");
    }

    #[test]
    fn arc_a_series_resolves_to_arc_client_driver() {
        let r = resolve_release(FIXTURE, 0x56A0, &win11())
            .unwrap()
            .expect("release");
        assert_eq!(r.version.display, "32.0.101.8801");
    }

    #[test]
    fn tiger_lake_integrated_resolves_to_its_own_32x_group_not_arc() {
        let r = resolve_release(FIXTURE, 0x9A49, &win11())
            .unwrap()
            .expect("release");
        assert_eq!(r.version.display, "32.0.101.7085");
        assert!(r.release_notes_url.as_deref().unwrap().contains("864990"));
        assert!(r.download_url.contains("7085"));
    }

    #[test]
    fn comet_lake_integrated_resolves_to_newest_legacy_31x_branch() {
        let r = resolve_release(FIXTURE, 0x9BC4, &win11())
            .unwrap()
            .expect("release");
        assert_eq!(r.version.display, "31.0.101.2141");
        assert!(r.release_notes_url.as_deref().unwrap().contains("776137"));
    }

    #[test]
    fn skylake_integrated_picks_newest_of_31x_over_legacy_15x() {
        let r = resolve_release(FIXTURE, 0x1912, &win11())
            .unwrap()
            .expect("release");
        assert_eq!(r.version.display, "31.0.101.2115");
    }

    #[test]
    fn unknown_and_zero_device_ids_resolve_to_nothing() {
        assert!(resolve_release(FIXTURE, 0xFFFF, &win11())
            .unwrap()
            .is_none());
        assert!(resolve_release(FIXTURE, 0, &win11()).unwrap().is_none());
    }

    #[test]
    fn wifi_only_device_id_is_ignored_by_the_graphics_filter() {
        assert!(resolve_release(FIXTURE, 0xABCD, &win11())
            .unwrap()
            .is_none());
    }

    #[test]
    fn history_spans_current_plus_sibling_historical_group_newest_first() {
        let releases = resolve_history(FIXTURE, 0xE20B, &win11(), 50).unwrap();
        let versions: Vec<_> = releases
            .iter()
            .map(|r| r.version.display.as_str())
            .collect();
        assert_eq!(versions, vec!["32.0.101.8801", "32.0.101.8626"]);
    }

    #[test]
    fn history_dedupes_repeated_versions_keeping_newest_first() {
        let dup = r#"[
            {"GroupId":"785597","Version":"32.0.101.8801","DisplayReleaseDate":"2026-05-15T00:00:00Z","Url":"https://intel/785597","Files":[{"Url":"https://dl/8801.exe","Size":2}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]},
            {"GroupId":"857252","Version":"32.0.101.8801","DisplayReleaseDate":"2026-03-01T00:00:00Z","Url":null,"Files":[{"Url":"https://dl/dup.exe","Size":1}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]}
        ]"#;
        let releases = resolve_history(dup, 0xE20B, &win11(), 50).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].size_bytes, 2);
    }

    #[test]
    fn history_respects_the_limit() {
        assert_eq!(
            resolve_history(FIXTURE, 0xE20B, &win11(), 1).unwrap().len(),
            1
        );
    }

    #[test]
    fn empty_array_and_malformed_json_handled() {
        assert!(resolve_release("[]", 0xE20B, &win11()).unwrap().is_none());
        assert!(resolve_release("not json", 0xE20B, &win11()).is_err());
        assert!(resolve_history("not json", 0xE20B, &win11(), 5).is_err());
    }

    #[test]
    fn graphics_entry_with_alt_or_missing_date_is_still_resolved() {
        let json = r#"[
            {"GroupId":"1","Version":"32.0.101.9999","DisplayReleaseDate":"2026-06-01","Url":"https://intel/1","Files":[{"Url":"https://dl/9999.exe","Size":5}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]},
            {"GroupId":"2","Version":"32.0.101.0001","DisplayReleaseDate":"","Url":null,"Files":[{"Url":"https://dl/0001.exe","Size":3}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]}
        ]"#;
        let releases = resolve_history(json, 0xE20B, &win11(), 50).unwrap();
        let versions: Vec<_> = releases
            .iter()
            .map(|r| r.version.display.as_str())
            .collect();
        assert!(
            versions.contains(&"32.0.101.9999"),
            "date-only entry must resolve"
        );
        assert!(
            versions.contains(&"32.0.101.0001"),
            "missing-date entry must not be silently dropped"
        );
        let dated = releases
            .iter()
            .find(|r| r.version.display == "32.0.101.9999")
            .unwrap();
        assert!(dated.released_at.is_some(), "YYYY-MM-DD must parse");
        let undated = releases
            .iter()
            .find(|r| r.version.display == "32.0.101.0001")
            .unwrap();
        assert!(undated.released_at.is_none());
    }

    // --- OS-specificity tests ---

    /// Catalog with both Win10-specific and Win11-specific packages for the same
    /// device, plus an older neutral package. A Win10 user must get the Win10
    /// entry; the Win11 and neutral entries must be suppressed.
    const OS_SPLIT_FIXTURE: &str = r#"[
        {"GroupId":"1","Version":"32.0.101.9000","DisplayReleaseDate":"2026-05-01T00:00:00Z","Name":"Intel Arc & Iris Xe Graphics - Windows 11","Url":"https://intel/w11","Files":[{"Url":"https://dl/9000-w11.exe","Size":900}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]},
        {"GroupId":"2","Version":"32.0.101.8900","DisplayReleaseDate":"2026-04-01T00:00:00Z","Name":"Intel Arc & Iris Xe Graphics - Windows 10","Url":"https://intel/w10","Files":[{"Url":"https://dl/8900-w10.exe","Size":890}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]},
        {"GroupId":"3","Version":"32.0.101.8000","DisplayReleaseDate":"2026-01-01T00:00:00Z","Name":"Intel Arc Graphics - Windows","Url":"https://intel/neutral","Files":[{"Url":"https://dl/8000-neutral.exe","Size":800}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]}
    ]"#;

    #[test]
    fn win10_target_prefers_win10_entry_over_win11_only_and_neutral() {
        let r = resolve_release(OS_SPLIT_FIXTURE, 0xE20B, &win10())
            .unwrap()
            .expect("release");
        assert_eq!(
            r.version.display, "32.0.101.8900",
            "Win10 user must receive the Windows 10 package, not the Win11 one"
        );
        assert!(
            r.download_url.contains("8900-w10"),
            "download URL must point at the Win10 installer"
        );
    }

    #[test]
    fn win11_target_prefers_win11_entry_over_win10_only_and_neutral() {
        let r = resolve_release(OS_SPLIT_FIXTURE, 0xE20B, &win11())
            .unwrap()
            .expect("release");
        assert_eq!(
            r.version.display, "32.0.101.9000",
            "Win11 user must receive the Windows 11 package, not the Win10 one"
        );
        assert!(r.download_url.contains("9000-w11"));
    }

    #[test]
    fn neutral_entry_is_served_when_no_os_specific_package_exists() {
        // Only a neutral entry in the catalog — both OS targets should receive it.
        let json = r#"[
            {"GroupId":"1","Version":"32.0.101.7777","DisplayReleaseDate":"2026-03-01T00:00:00Z","Name":"Intel Arc Graphics - Windows","Url":"https://intel/neutral","Files":[{"Url":"https://dl/7777.exe","Size":777}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]}
        ]"#;
        assert_eq!(
            resolve_release(json, 0xE20B, &win10())
                .unwrap()
                .expect("release")
                .version
                .display,
            "32.0.101.7777"
        );
        assert_eq!(
            resolve_release(json, 0xE20B, &win11())
                .unwrap()
                .expect("release")
                .version
                .display,
            "32.0.101.7777"
        );
    }

    #[test]
    fn win10_target_receives_nothing_when_only_win11_entry_exists() {
        let json = r#"[
            {"GroupId":"1","Version":"32.0.101.9000","DisplayReleaseDate":"2026-05-01T00:00:00Z","Name":"Intel Arc & Iris Xe Graphics - Windows 11","Url":"https://intel/w11","Files":[{"Url":"https://dl/9000-w11.exe","Size":900}],"Components":[{"Category":"Graphics","DetectionValues":["VEN_8086&DEV_E20B"]}]}
        ]"#;
        assert!(
            resolve_release(json, 0xE20B, &win10()).unwrap().is_none(),
            "Win10 user must not be served a Win11-only package"
        );
    }

    #[test]
    fn history_os_split_win10_returns_only_win10_entries() {
        let releases = resolve_history(OS_SPLIT_FIXTURE, 0xE20B, &win10(), 50).unwrap();
        assert_eq!(
            releases.len(),
            1,
            "only the Win10 package should be in history"
        );
        assert_eq!(releases[0].version.display, "32.0.101.8900");
    }

    #[test]
    fn os_specificity_detects_both_keywords_as_neutral() {
        // A pathological entry mentioning both "Windows 10" and "Windows 11" is
        // treated as neutral rather than misidentified — neither user is harmed.
        let entry = serde_json::json!({
            "Name": "Intel Driver for Windows 10 and Windows 11",
            "Components": []
        });
        assert_eq!(entry_os_specificity(&entry), OsSpecificity::Neutral);
    }
}
