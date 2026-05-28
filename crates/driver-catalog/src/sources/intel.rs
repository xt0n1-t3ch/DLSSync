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

fn group_id_str(entry: &serde_json::Value) -> Option<String> {
    let gid = &entry["GroupId"];
    gid.as_str()
        .map(str::to_string)
        .or_else(|| gid.as_u64().map(|n| n.to_string()))
}

fn group_matches(entry: &serde_json::Value, target: &str) -> bool {
    group_id_str(entry).as_deref() == Some(target)
}

fn group_is_client(entry: &serde_json::Value) -> bool {
    group_matches(entry, c::CLIENT_GRAPHICS_GROUP_ID)
}

/// Intel publishes a sibling "Historical" group next to each current driver
/// group; including those entries gives the user a known-compatible older
/// driver without scraping intel.com (which is Akamai-protected from server
/// requests).
const HISTORICAL_GROUP_IDS: &[&str] = &["857252", "857390"];

fn group_is_history_capable(entry: &serde_json::Value) -> bool {
    if group_is_client(entry) {
        return true;
    }
    let Some(gid) = group_id_str(entry) else {
        return false;
    };
    HISTORICAL_GROUP_IDS.iter().any(|h| gid == *h)
}

fn parse_iso_date(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(raw.trim())
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

fn entry_to_release(entry: &serde_json::Value) -> Option<DriverRelease> {
    let token = version_token(entry["Version"].as_str().unwrap_or_default());
    if !token.starts_with(c::GRAPHICS_VERSION_PREFIX) {
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

/// Pure parser over the `software-configurations.json` array extracted from the
/// Intel DSA catalog. Picks the newest WHQL/optional client graphics driver.
pub fn parse_software_configurations(json: &str) -> Result<Option<DriverRelease>, DriverError> {
    let root: serde_json::Value =
        serde_json::from_str(json).map_err(|e| DriverError::Parse(e.to_string()))?;
    let entries = root
        .as_array()
        .ok_or_else(|| DriverError::Parse("software-configurations: expected array".into()))?;
    let mut best: Option<DriverRelease> = None;
    for entry in entries {
        if !group_is_client(entry) {
            continue;
        }
        let Some(release) = entry_to_release(entry) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|b| release.released_at > b.released_at)
        {
            best = Some(release);
        }
    }
    Ok(best)
}

/// Pure parser returning the current client driver plus every entry from the
/// sibling "Historical Intel Arc/Pro Graphics" groups (Intel only ships the
/// previous recommended driver in DSA — that yields 1-2 entries per device,
/// limited but honest data with no fragile HTML scraping).
pub fn parse_software_configurations_history(
    json: &str,
) -> Result<Vec<DriverRelease>, DriverError> {
    let root: serde_json::Value =
        serde_json::from_str(json).map_err(|e| DriverError::Parse(e.to_string()))?;
    let entries = root
        .as_array()
        .ok_or_else(|| DriverError::Parse("software-configurations: expected array".into()))?;
    let mut releases: Vec<DriverRelease> = entries
        .iter()
        .filter(|e| group_is_history_capable(e))
        .filter_map(entry_to_release)
        .collect();
    releases.sort_by_key(|r| std::cmp::Reverse(r.released_at));
    let mut seen = std::collections::BTreeSet::new();
    releases.retain(|r| seen.insert(r.version.display.clone()));
    Ok(releases)
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
        _device: &DeviceId,
        _os: &OsTarget,
    ) -> Result<Option<DriverRelease>, DriverError> {
        let json = fetch_configurations_json(client).await?;
        parse_software_configurations(&json)
    }

    async fn history(
        &self,
        client: &reqwest::Client,
        _device: &DeviceId,
        _os: &OsTarget,
        limit: usize,
    ) -> Result<Vec<DriverRelease>, DriverError> {
        let json = fetch_configurations_json(client).await?;
        let mut releases = parse_software_configurations_history(&json)?;
        releases.truncate(limit);
        Ok(releases)
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

    fn device(vendor: DriverVendor, class: DeviceClass) -> DeviceId {
        DeviceId {
            class,
            vendor,
            pci_vendor_id: 0,
            pci_device_id: 0,
            model: "intel arc b580".into(),
        }
    }

    #[test]
    fn supports_only_intel_gpus() {
        let source = IntelGpuSource;
        assert!(source.supports(&device(DriverVendor::Intel, DeviceClass::Gpu)));
        assert!(!source.supports(&device(DriverVendor::Nvidia, DeviceClass::Gpu)));
    }

    const FIXTURE: &str = r#"[
        {"Version":"15.22.58.2993","DisplayReleaseDate":"2013-02-19T00:00:00Z","Name":"Old GMA","GroupId":"111","Url":"https://intel/old","Files":[{"Url":"https://dl/old.exe","Size":1}]},
        {"Version":"32.0.101.8517 - Q1.26.R2","DisplayReleaseDate":"2026-04-24T00:00:00Z","Name":"Intel Arc Pro Graphics","GroupId":"741626","Url":"https://intel/pro","Files":[{"Url":"https://dl/pro.exe","Size":2}]},
        {"Version":"32.0.101.7085","DisplayReleaseDate":"2026-03-18T00:00:00Z","Name":"Intel Arc Graphics","GroupId":785597,"IsBeta":false,"Url":"https://intel/old-client","Files":[{"Url":"https://dl/7085.exe","Size":700}]},
        {"Version":"32.0.101.8801 WHQL Certified","DisplayReleaseDate":"2026-05-15T00:00:00Z","Name":"Intel Arc Graphics - Windows","GroupId":"785597","IsBeta":false,"Url":"https://www.intel.com/.../785597/.../graphics.html","Files":[{"Url":"https://downloadmirror.intel.com/919751/gfx_win_101.8801.exe","Hash":"81093879","Size":823031088,"OperatingSystems":["windows-11-24h2-64"]}]}
    ]"#;

    #[test]
    fn parses_newest_client_graphics_driver_only() {
        let release = parse_software_configurations(FIXTURE)
            .unwrap()
            .expect("release");
        assert_eq!(release.vendor, DriverVendor::Intel);
        assert_eq!(release.version.display, "32.0.101.8801");
        assert_eq!(
            release.download_url,
            "https://downloadmirror.intel.com/919751/gfx_win_101.8801.exe"
        );
        assert_eq!(release.size_bytes, 823031088);
        assert_eq!(release.signature_subject, "Intel Corporation");
        assert!(release
            .release_notes_url
            .as_deref()
            .unwrap()
            .contains("intel.com"));
        assert!(release.released_at.is_some());
    }

    #[test]
    fn ignores_pro_and_legacy_and_returns_none_when_no_client() {
        let only_pro = r#"[{"Version":"32.0.101.8517","DisplayReleaseDate":"2026-04-24T00:00:00Z","GroupId":"741626","Files":[{"Url":"pro.exe","Size":2}]}]"#;
        assert!(parse_software_configurations(only_pro).unwrap().is_none());
    }

    #[test]
    fn empty_array_and_malformed_json_handled() {
        assert!(parse_software_configurations("[]").unwrap().is_none());
        assert!(parse_software_configurations("not json").is_err());
    }

    const HISTORY_FIXTURE: &str = r#"[
        {"Version":"15.22.58.2993","DisplayReleaseDate":"2013-02-19T00:00:00Z","GroupId":"111","Files":[{"Url":"https://dl/old.exe","Size":1}]},
        {"Version":"32.0.101.8801 WHQL Certified","DisplayReleaseDate":"2026-05-15T00:00:00Z","Name":"Intel Arc Graphics","GroupId":"785597","Url":"https://intel/785597","Files":[{"Url":"https://dl/8801.exe","Size":823031088}]},
        {"Version":"32.0.101.8626 WHQL Certified","DisplayReleaseDate":"2026-03-17T00:00:00Z","Name":"Historical Intel Arc Graphics","GroupId":"857252","Url":null,"Files":[{"Url":"https://dl/8626.exe","Size":1064962136}]},
        {"Version":"32.0.101.6637 - Q1.25","DisplayReleaseDate":"2025-03-27T00:00:00Z","Name":"Historical Intel Arc Pro Graphics","GroupId":"857390","Url":null,"Files":[{"Url":"https://dl/6637.exe","Size":658757216}]},
        {"Version":"32.0.101.7085","DisplayReleaseDate":"2026-03-18T00:00:00Z","Name":"Intel 11-14th Gen integrated","GroupId":"864990","Files":[{"Url":"https://dl/7085.exe","Size":700}]},
        {"Version":"32.0.101.8801 WHQL Certified","DisplayReleaseDate":"2026-05-15T00:00:00Z","GroupId":"785597","Files":[{"Url":"https://dl/dup.exe","Size":1}]}
    ]"#;

    #[test]
    fn history_collects_current_plus_sibling_historical_groups_newest_first() {
        let releases = parse_software_configurations_history(HISTORY_FIXTURE).unwrap();
        let versions: Vec<_> = releases
            .iter()
            .map(|r| r.version.display.as_str())
            .collect();
        assert_eq!(
            versions,
            vec!["32.0.101.8801", "32.0.101.8626", "32.0.101.6637"]
        );
    }

    #[test]
    fn history_dedupes_repeated_versions_keeping_first_occurrence() {
        let releases = parse_software_configurations_history(HISTORY_FIXTURE).unwrap();
        assert_eq!(
            releases
                .iter()
                .filter(|r| r.version.display == "32.0.101.8801")
                .count(),
            1
        );
        assert_eq!(releases[0].size_bytes, 823031088);
    }

    #[test]
    fn history_excludes_unrelated_groups_and_pre_graphics_versions() {
        let releases = parse_software_configurations_history(HISTORY_FIXTURE).unwrap();
        for r in &releases {
            assert!(
                r.version.display.starts_with("32.0.101."),
                "non-graphics version leaked into history: {}",
                r.version.display
            );
        }
        assert!(releases
            .iter()
            .all(|r| r.version.display != "32.0.101.7085"));
    }

    #[test]
    fn history_returns_empty_on_no_matches_or_malformed_json() {
        assert!(parse_software_configurations_history("[]")
            .unwrap()
            .is_empty());
        assert!(parse_software_configurations_history("not json").is_err());
    }
}
