use super::DriverSource;
use crate::consts::nvidia as c;
use crate::{
    DeviceClass, DeviceId, DriverChangelog, DriverError, DriverRelease, DriverVendor,
    DriverVersion, OsFamily, OsTarget, ReleaseChannel,
};
use async_trait::async_trait;

pub struct NvidiaGpuSource;

pub fn os_id(os: &OsTarget) -> u32 {
    match os.family {
        OsFamily::Windows10X64 => c::OS_ID_WIN10_X64,
        OsFamily::Windows11X64 => c::OS_ID_WIN11_X64,
    }
}

pub fn clean_gpu_name(model: &str) -> String {
    let stripped = model.trim().strip_prefix("NVIDIA ").unwrap_or(model.trim());
    let mut name = stripped.to_string();
    for marker in [" with Max-Q Design", " (OEM)", " COLLECTORS EDITION"] {
        if let Some(index) = name.find(marker) {
            name.truncate(index);
        }
    }
    name.replace("Super", "SUPER").trim().to_string()
}

pub fn match_pfid(gpu_data: &serde_json::Value, model: &str) -> Option<String> {
    find_pfid(gpu_data, &clean_gpu_name(model))
}

fn find_pfid(value: &serde_json::Value, key: &str) -> Option<String> {
    let serde_json::Value::Object(map) = value else {
        return None;
    };
    if let Some(found) = map.get(key).and_then(|v| v.as_str()) {
        return Some(found.to_string());
    }
    for (candidate, nested) in map {
        if candidate.eq_ignore_ascii_case(key) {
            if let Some(found) = nested.as_str() {
                return Some(found.to_string());
            }
        }
        if let Some(found) = find_pfid(nested, key) {
            return Some(found);
        }
    }
    None
}

pub fn build_lookup_url(pfid: &str, os_id: u32, dch: bool, game_ready: bool) -> String {
    build_lookup_url_with_count(pfid, os_id, dch, game_ready, 1, true)
}

/// Variant that asks the Ajax service for up to `count` historical drivers per
/// query. NVIDIA caps `numberOfResults` near 50 in practice; values above that
/// silently return what the service has on file. `whql_only=false` drops the
/// WHQL filter so Beta/Studio rows surface alongside Game Ready WHQL entries.
pub fn build_lookup_url_with_count(
    pfid: &str,
    os_id: u32,
    dch: bool,
    game_ready: bool,
    count: usize,
    whql_only: bool,
) -> String {
    format!(
        "{base}?func=DriverManualLookup&pfid={pfid}&osID={os}&languageCode={lang}&isWHQL={whql}&dch={dch}&upCRD={crd}&sort1=0&numberOfResults={count}",
        base = c::AJAX_DRIVER_LOOKUP,
        pfid = pfid,
        os = os_id,
        lang = c::LANGUAGE_CODE_EN_US,
        whql = u8::from(whql_only),
        dch = u8::from(dch),
        crd = u8::from(!game_ready),
        count = count,
    )
}

pub fn parse_lookup_response(body: &str) -> Result<Option<DriverRelease>, DriverError> {
    let releases = parse_history_response(body)?;
    Ok(releases.into_iter().next())
}

/// Parse every `IDS[i].downloadInfo` block into a `DriverRelease`, preserving
/// Ajax order (which is newest-first). Drops entries with an empty `Version`
/// rather than failing the whole batch. The Ajax server uses the `Success` field
/// as the COUNT of returned drivers when `numberOfResults > 1` (so a 50-row
/// response carries `Success: 50`), which is why this function does not gate on
/// `Success` and instead trusts the `IDS` array.
pub fn parse_history_response(body: &str) -> Result<Vec<DriverRelease>, DriverError> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| DriverError::Parse(e.to_string()))?;
    let ids = root["IDS"].as_array().cloned().unwrap_or_default();
    let mut out = Vec::with_capacity(ids.len());
    for entry in ids {
        if let Some(release) = parse_download_info(&entry["downloadInfo"]) {
            out.push(release);
        }
    }
    Ok(out)
}

fn parse_download_info(info: &serde_json::Value) -> Option<DriverRelease> {
    let version_str = info["Version"].as_str().unwrap_or_default();
    if version_str.is_empty() {
        return None;
    }
    let is_beta = flag_is_true(&info["IsBeta"]);
    let display_version = info["DisplayVersion"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let changelog = extract_changelog(
        info["ReleaseNotes"].as_str().unwrap_or_default(),
        info["OtherNotes"].as_str().unwrap_or_default(),
    );
    Some(DriverRelease {
        vendor: DriverVendor::Nvidia,
        version: DriverVersion::nvidia(version_str),
        channel: if is_beta {
            ReleaseChannel::Beta
        } else {
            ReleaseChannel::Stable
        },
        display_version,
        is_beta,
        download_url: info["DownloadURL"].as_str().unwrap_or_default().to_string(),
        size_bytes: parse_size(info["DownloadURLFileSize"].as_str().unwrap_or_default()),
        signature_subject: c::PUBLISHER_SUBJECT.to_string(),
        released_at: parse_release_date(info["ReleaseDateTime"].as_str().unwrap_or_default()),
        release_notes_url: info["DetailsURL"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        changelog: (!changelog.is_empty()).then_some(changelog),
    })
}

fn flag_is_true(value: &serde_json::Value) -> bool {
    value.as_u64() == Some(1) || value.as_str() == Some("1") || value.as_bool() == Some(true)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn strip_html(input: &str) -> String {
    let mut text = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }
    text.replace("&amp;", "&")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn inner_matches(html: &str, open: &str, close: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find(open) {
        let after = &rest[start + open.len()..];
        let Some(end) = after.find(close) else { break };
        let chunk = strip_html(&after[..end]);
        if !chunk.is_empty() {
            out.push(chunk);
        }
        rest = &after[end + close.len()..];
    }
    out
}

fn extract_changelog(release_notes_enc: &str, other_notes_enc: &str) -> DriverChangelog {
    let notes = percent_decode(release_notes_enc);
    let highlights: Vec<String> = inner_matches(&notes, "<strong>", "</strong>")
        .into_iter()
        .filter(|h| {
            let l = h.to_lowercase();
            l.starts_with("game ready") || l.contains("studio") || l.contains("driver")
        })
        .take(3)
        .collect();
    let fixed = inner_matches(&notes, "<li>", "</li>")
        .into_iter()
        .filter(|f| !f.eq_ignore_ascii_case("n/a"))
        .take(12)
        .collect();
    let other = percent_decode(other_notes_enc);
    let notes_page_url = other
        .split(['"', '\''])
        .find(|s| s.contains("release-notes") && s.ends_with(".pdf"))
        .map(str::to_string);
    DriverChangelog {
        highlights,
        fixed,
        notes_page_url,
    }
}

fn parse_size(raw: &str) -> u64 {
    let mut parts = raw.split_whitespace();
    let Some(number) = parts.next().and_then(|n| n.parse::<f64>().ok()) else {
        return 0;
    };
    let multiplier = match parts.next().unwrap_or("MB").to_ascii_uppercase().as_str() {
        "GB" => 1024.0 * 1024.0 * 1024.0,
        "KB" => 1024.0,
        _ => 1024.0 * 1024.0,
    };
    (number * multiplier) as u64
}

fn parse_release_date(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{NaiveDate, TimeZone, Utc};
    let raw = raw.trim();
    let at_midnight =
        |date: NaiveDate| -> Option<_> { Some(Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0)?)) };
    if let Ok(date) = NaiveDate::parse_from_str(raw, "%a %b %d, %Y") {
        return at_midnight(date);
    }
    let parts: Vec<i64> = raw
        .split('-')
        .filter_map(|p| p.trim().parse::<i64>().ok())
        .collect();
    if parts.len() == 3 {
        let date = NaiveDate::from_ymd_opt(parts[0] as i32, parts[1] as u32, parts[2] as u32)?;
        return at_midnight(date);
    }
    None
}

#[async_trait]
impl DriverSource for NvidiaGpuSource {
    fn id(&self) -> &'static str {
        "nvidia-gpu"
    }

    fn supports(&self, device: &DeviceId) -> bool {
        device.class == DeviceClass::Gpu && device.vendor == DriverVendor::Nvidia
    }

    async fn latest(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        os: &OsTarget,
    ) -> Result<Option<DriverRelease>, DriverError> {
        let Some(pfid) = resolve_pfid(client, device).await? else {
            return Ok(None);
        };
        let url = build_lookup_url(&pfid, os_id(os), os.dch, true);
        let body = client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_lookup_response(&body)
    }

    async fn history(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        os: &OsTarget,
        limit: usize,
    ) -> Result<Vec<DriverRelease>, DriverError> {
        let Some(pfid) = resolve_pfid(client, device).await? else {
            return Ok(Vec::new());
        };
        let count = limit.clamp(1, 50);
        let url = build_lookup_url_with_count(&pfid, os_id(os), os.dch, true, count, false);
        let body = client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_history_response(&body)
    }
}

async fn resolve_pfid(
    client: &reqwest::Client,
    device: &DeviceId,
) -> Result<Option<String>, DriverError> {
    let gpu_data: serde_json::Value = client
        .get(c::GPU_DATA_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(match_pfid(&gpu_data, &device.model))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_gpu_name_strips_prefix_and_suffixes() {
        assert_eq!(
            clean_gpu_name("NVIDIA GeForce RTX 3070"),
            "GeForce RTX 3070"
        );
        assert_eq!(
            clean_gpu_name("NVIDIA GeForce RTX 4080 with Max-Q Design"),
            "GeForce RTX 4080"
        );
        assert_eq!(
            clean_gpu_name("GeForce RTX 4070 Super"),
            "GeForce RTX 4070 SUPER"
        );
    }

    #[test]
    fn match_pfid_finds_flat_and_nested_entries() {
        let flat = serde_json::json!({ "GeForce RTX 3070": "933" });
        assert_eq!(
            match_pfid(&flat, "NVIDIA GeForce RTX 3070"),
            Some("933".to_string())
        );

        let nested = serde_json::json!({
            "desktop": { "GeForce RTX 4090": "1010" },
            "notebook": { "GeForce RTX 4090 Laptop GPU": "1011" }
        });
        assert_eq!(
            match_pfid(&nested, "NVIDIA GeForce RTX 4090"),
            Some("1010".to_string())
        );
        assert_eq!(match_pfid(&nested, "GeForce RTX 2060"), None);
    }

    #[test]
    fn build_lookup_url_encodes_lookup_parameters() {
        let url = build_lookup_url("933", 135, true, true);
        assert!(url.contains("pfid=933"));
        assert!(url.contains("osID=135"));
        assert!(url.contains("dch=1"));
        assert!(url.contains("upCRD=0"));
        let studio = build_lookup_url("933", 57, false, false);
        assert!(studio.contains("dch=0"));
        assert!(studio.contains("upCRD=1"));
    }

    #[test]
    fn parse_lookup_response_extracts_release_on_success() {
        let body = r#"{
            "Success": "1",
            "IDS": [{
                "downloadInfo": {
                    "Version": "572.16",
                    "DownloadURL": "https://us.download.nvidia.com/Windows/572.16/572.16-desktop-win10-win11-64bit-international-dch-whql.exe",
                    "DownloadURLFileSize": "823.45 MB",
                    "ReleaseDateTime": "2025-2-12",
                    "DetailsURL": "https://www.nvidia.com/details"
                }
            }]
        }"#;
        let release = parse_lookup_response(body).unwrap().expect("release");
        assert_eq!(release.version.display, "572.16");
        assert_eq!(release.version.packed, 57216);
        assert!(release.download_url.ends_with("dch-whql.exe"));
        assert!(release.size_bytes > 800 * 1024 * 1024);
        assert_eq!(release.signature_subject, "NVIDIA Corporation");
        assert!(release.released_at.is_some());
        assert_eq!(
            release.release_notes_url.as_deref(),
            Some("https://www.nvidia.com/details")
        );
    }

    #[test]
    fn parse_lookup_response_returns_none_on_failure_or_empty_version() {
        let failure = r#"{"Success":"0","IDS":[]}"#;
        assert!(parse_lookup_response(failure).unwrap().is_none());

        let empty = r#"{"Success":"1","IDS":[{"downloadInfo":{"Version":""}}]}"#;
        assert!(parse_lookup_response(empty).unwrap().is_none());
    }

    #[test]
    fn parse_lookup_response_rejects_malformed_json() {
        assert!(parse_lookup_response("not json").is_err());
    }

    #[test]
    fn parse_size_handles_units_and_garbage() {
        assert_eq!(parse_size("1.00 GB"), 1024 * 1024 * 1024);
        assert_eq!(parse_size("512 KB"), 512 * 1024);
        assert_eq!(parse_size("700 MB"), 700 * 1024 * 1024);
        assert_eq!(parse_size("unknown"), 0);
        assert_eq!(parse_size(""), 0);
    }

    #[test]
    fn parse_release_date_handles_live_named_and_dash_formats() {
        assert!(parse_release_date("Tue May 26, 2026").is_some());
        assert!(parse_release_date("2025-2-12").is_some());
        assert!(parse_release_date("garbage").is_none());
        assert!(parse_release_date("").is_none());
    }

    #[test]
    fn extract_changelog_pulls_headline_and_fixed_bullets() {
        let notes = "%3Cstrong%3EGame%20Ready%20for%20007%20First%20Light%3C%2Fstrong%3E%3Cbr%3E%3Cstrong%3EFixed%20Gaming%20Bugs%3C%2Fstrong%3E%3Cul%3E%3Cli%3EEnshrouded%3A%20Missing%20terrain%20%5B5955501%5D%3C%2Fli%3E%3Cli%3EN%2FA%3C%2Fli%3E%3C%2Ful%3E";
        let other = "%3Ca%20href%3D%22https%3A%2F%2Fus.download.nvidia.com%2FWindows%2F610.47%2F610.47-win11-win10-release-notes.pdf%22%3Enotes%3C%2Fa%3E";
        let log = extract_changelog(notes, other);
        assert!(log
            .highlights
            .iter()
            .any(|h| h.contains("Game Ready for 007 First Light")));
        assert_eq!(log.fixed, vec!["Enshrouded: Missing terrain [5955501]"]);
        assert_eq!(
            log.notes_page_url.as_deref(),
            Some("https://us.download.nvidia.com/Windows/610.47/610.47-win11-win10-release-notes.pdf")
        );
    }

    #[test]
    fn build_lookup_url_with_count_overrides_number_of_results() {
        let url = build_lookup_url_with_count("1040", 135, true, true, 50, true);
        assert!(url.contains("numberOfResults=50"));
        assert!(url.contains("pfid=1040"));
        assert!(url.contains("osID=135"));
        assert!(url.contains("isWHQL=1"));
    }

    #[test]
    fn build_lookup_url_with_count_emits_unfiltered_query_when_whql_only_false() {
        let url = build_lookup_url_with_count("1040", 135, true, true, 50, false);
        assert!(url.contains("isWHQL=0"));
        assert!(url.contains("numberOfResults=50"));
    }

    #[test]
    fn parse_history_response_keeps_every_valid_id_newest_first() {
        let body = r#"{
            "Success": "1",
            "IDS": [
              {"downloadInfo":{"Version":"610.47","DownloadURL":"https://x/a.exe","DownloadURLFileSize":"1.00 GB","ReleaseDateTime":"Tue May 26, 2026"}},
              {"downloadInfo":{"Version":"596.49","DownloadURL":"https://x/b.exe","DownloadURLFileSize":"950 MB","ReleaseDateTime":"Tue May 12, 2026"}},
              {"downloadInfo":{"Version":"","DownloadURL":"https://x/c.exe"}}
            ]
        }"#;
        let releases = parse_history_response(body).expect("parse");
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].version.display, "610.47");
        assert_eq!(releases[1].version.display, "596.49");
    }

    #[test]
    fn parse_history_response_on_failure_returns_empty_vec() {
        let body = r#"{"Success":"0","IDS":[]}"#;
        assert!(parse_history_response(body).unwrap().is_empty());
    }

    #[test]
    fn parse_history_response_treats_success_as_a_count_not_a_boolean() {
        let body = r#"{
            "Success": 3,
            "IDS": [
              {"downloadInfo":{"Version":"610.47","DownloadURL":"https://x/a.exe","ReleaseDateTime":"Tue May 26, 2026"}},
              {"downloadInfo":{"Version":"596.49","DownloadURL":"https://x/b.exe","ReleaseDateTime":"Tue May 12, 2026"}},
              {"downloadInfo":{"Version":"596.36","DownloadURL":"https://x/c.exe","ReleaseDateTime":"Tue Apr 28, 2026"}}
            ]
        }"#;
        let releases = parse_history_response(body).expect("parse");
        assert_eq!(releases.len(), 3);
    }

    #[test]
    fn parse_lookup_response_carries_beta_and_changelog() {
        let body = r#"{
            "Success": "1",
            "IDS": [{
                "downloadInfo": {
                    "Version": "610.50",
                    "DisplayVersion": "R610 U1 (610.50)",
                    "IsBeta": "1",
                    "DownloadURL": "https://us.download.nvidia.com/Windows/610.50/x.exe",
                    "DownloadURLFileSize": "900 MB",
                    "ReleaseDateTime": "Tue May 26, 2026",
                    "DetailsURL": "https://www.nvidia.com/en-us/drivers/details/271418/",
                    "ReleaseNotes": "%3Cstrong%3EGame%20Ready%20for%20Test%3C%2Fstrong%3E%3Cul%3E%3Cli%3EFixed%20a%20thing%3C%2Fli%3E%3C%2Ful%3E"
                }
            }]
        }"#;
        let r = parse_lookup_response(body).unwrap().expect("release");
        assert!(r.is_beta);
        assert_eq!(r.channel, ReleaseChannel::Beta);
        assert_eq!(r.display_version.as_deref(), Some("R610 U1 (610.50)"));
        let log = r.changelog.expect("changelog");
        assert!(log
            .highlights
            .iter()
            .any(|h| h.contains("Game Ready for Test")));
        assert_eq!(log.fixed, vec!["Fixed a thing"]);
    }
}
