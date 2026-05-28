use super::DriverSource;
use crate::consts::intel as c;
use crate::{
    DeviceClass, DeviceId, DriverError, DriverRelease, DriverVendor, DriverVersion, OsTarget,
    ReleaseChannel,
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

fn entry_to_release(entry: &serde_json::Value) -> Option<DriverRelease> {
    let token = version_token(entry["Version"].as_str().unwrap_or_default());
    if !token.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let date = parse_iso_date(entry["DisplayReleaseDate"].as_str().unwrap_or_default())?;
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
        released_at: Some(date),
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
pub fn resolve_history(
    json: &str,
    device_id: u16,
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
    let mut releases: Vec<DriverRelease> = entries
        .iter()
        .filter(|e| entry_is_graphics(e) && entry_supports_device(e, device_id))
        .filter_map(entry_to_release)
        .collect();
    releases.sort_by_key(|r| std::cmp::Reverse(r.released_at));
    let mut seen = std::collections::BTreeSet::new();
    releases.retain(|r| seen.insert(r.version.display.clone()));
    releases.truncate(limit);
    Ok(releases)
}

/// The newest graphics driver that actually supports this exact device — the
/// correct integrated / Arc / legacy package, carrying its real download URL and
/// its own release-notes page (not the generic Arc landing page).
pub fn resolve_release(json: &str, device_id: u16) -> Result<Option<DriverRelease>, DriverError> {
    Ok(resolve_history(json, device_id, usize::MAX)?
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
        _os: &OsTarget,
    ) -> Result<Option<DriverRelease>, DriverError> {
        let json = fetch_configurations_json(client).await?;
        resolve_release(&json, device.pci_device_id)
    }

    async fn history(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        _os: &OsTarget,
        limit: usize,
    ) -> Result<Vec<DriverRelease>, DriverError> {
        let json = fetch_configurations_json(client).await?;
        resolve_history(&json, device.pci_device_id, limit)
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

    fn device(device_id: u16) -> DeviceId {
        DeviceId {
            class: DeviceClass::Gpu,
            vendor: DriverVendor::Intel,
            pci_vendor_id: 0x8086,
            pci_device_id: device_id,
            model: "intel graphics".into(),
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
        let r = resolve_release(FIXTURE, 0xE20B).unwrap().expect("release");
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
        let r = resolve_release(FIXTURE, 0x56A0).unwrap().expect("release");
        assert_eq!(r.version.display, "32.0.101.8801");
    }

    #[test]
    fn tiger_lake_integrated_resolves_to_its_own_32x_group_not_arc() {
        let r = resolve_release(FIXTURE, 0x9A49).unwrap().expect("release");
        assert_eq!(r.version.display, "32.0.101.7085");
        assert!(r.release_notes_url.as_deref().unwrap().contains("864990"));
        assert!(r.download_url.contains("7085"));
    }

    #[test]
    fn comet_lake_integrated_resolves_to_newest_legacy_31x_branch() {
        let r = resolve_release(FIXTURE, 0x9BC4).unwrap().expect("release");
        assert_eq!(r.version.display, "31.0.101.2141");
        assert!(r.release_notes_url.as_deref().unwrap().contains("776137"));
    }

    #[test]
    fn skylake_integrated_picks_newest_of_31x_over_legacy_15x() {
        let r = resolve_release(FIXTURE, 0x1912).unwrap().expect("release");
        assert_eq!(r.version.display, "31.0.101.2115");
    }

    #[test]
    fn unknown_and_zero_device_ids_resolve_to_nothing() {
        assert!(resolve_release(FIXTURE, 0xFFFF).unwrap().is_none());
        assert!(resolve_release(FIXTURE, 0).unwrap().is_none());
    }

    #[test]
    fn wifi_only_device_id_is_ignored_by_the_graphics_filter() {
        assert!(resolve_release(FIXTURE, 0xABCD).unwrap().is_none());
    }

    #[test]
    fn history_spans_current_plus_sibling_historical_group_newest_first() {
        let releases = resolve_history(FIXTURE, 0xE20B, 50).unwrap();
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
        let releases = resolve_history(dup, 0xE20B, 50).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].size_bytes, 2);
    }

    #[test]
    fn history_respects_the_limit() {
        assert_eq!(resolve_history(FIXTURE, 0xE20B, 1).unwrap().len(), 1);
    }

    #[test]
    fn empty_array_and_malformed_json_handled() {
        assert!(resolve_release("[]", 0xE20B).unwrap().is_none());
        assert!(resolve_release("not json", 0xE20B).is_err());
        assert!(resolve_history("not json", 0xE20B, 5).is_err());
    }
}
