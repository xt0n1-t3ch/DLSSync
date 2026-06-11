use super::DriverSource;
use crate::consts::amd as c;
use crate::{
    DeviceClass, DeviceId, DriverError, DriverRelease, DriverVendor, DriverVersion, OsTarget,
    ReleaseChannel,
};
use async_trait::async_trait;

pub struct AmdGpuSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmdArch {
    Mainstream,
    Rdna12,
    PolarisVega,
}

/// Known PCI device ids on the RDNA1/RDNA2 legacy driver branch (Navi 10/12/14
/// = RX 5000, Navi 21/22/23/24 = RX 6000). Used to pick the right branch even
/// when an OEM renames the card so the model string is unreliable.
const RDNA12_DEVICE_IDS: &[u16] = &[
    0x7310, 0x7312, 0x7318, 0x7319, 0x731A, 0x731B, 0x731E, 0x731F, 0x7340, 0x7341, 0x7347, 0x734F,
    0x7360, 0x7362, 0x73A0, 0x73A1, 0x73A2, 0x73A3, 0x73A5, 0x73AB, 0x73AE, 0x73AF, 0x73BF, 0x73D0,
    0x73DF, 0x73E0, 0x73E1, 0x73E3, 0x73E8, 0x73E9, 0x73EF, 0x73FF, 0x7420, 0x7421, 0x7422, 0x7423,
    0x743F,
];

/// Known PCI device ids on the Polaris/Vega legacy driver branch (Polaris 10/11/
/// 12/20/21/22 = RX 400/500, Vega 10/12/20 = Vega 56/64, Radeon VII).
const POLARIS_VEGA_DEVICE_IDS: &[u16] = &[
    0x67C0, 0x67C2, 0x67C4, 0x67C7, 0x67CA, 0x67CC, 0x67CF, 0x67D0, 0x67D4, 0x67D7, 0x67DF, 0x6FDF,
    0x67E0, 0x67E1, 0x67E3, 0x67E8, 0x67EB, 0x67EF, 0x67FF, 0x6980, 0x6981, 0x6985, 0x6986, 0x6987,
    0x698F, 0x699F, 0x6860, 0x6861, 0x6862, 0x6863, 0x6864, 0x6867, 0x6868, 0x686C, 0x687F, 0x69A0,
    0x69A1, 0x69A2, 0x69A3, 0x69AF, 0x66A0, 0x66A1, 0x66A2, 0x66A3, 0x66A7, 0x66AF,
];

/// Classify by PCI device id — the authoritative key. Returns `None` for ids not
/// on a legacy branch (RDNA3+ / APUs / unknown) so the caller falls back to the
/// model-name classifier (which defaults such cards to the mainstream branch).
pub fn amd_arch_from_device_id(device_id: u16) -> Option<AmdArch> {
    if device_id == 0 {
        return None;
    }
    if RDNA12_DEVICE_IDS.contains(&device_id) {
        return Some(AmdArch::Rdna12);
    }
    if POLARIS_VEGA_DEVICE_IDS.contains(&device_id) {
        return Some(AmdArch::PolarisVega);
    }
    None
}

/// Classify an AMD GPU model into the driver branch AMD publishes for it.
/// RDNA3+ (RX 7000/9000) take the mainstream branch; RX 5000/6000 take the
/// RDNA1/2 legacy branch; Polaris/Vega take their own. Used as a fallback when
/// the PCI device id is unknown.
pub fn amd_arch(model: &str) -> AmdArch {
    let m = model.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| m.contains(n));
    if has(&[
        "rx 5500", "rx 5600", "rx 5700", "rx 64", "rx 65", "rx 66", "rx 67", "rx 68", "rx 69",
    ]) {
        return AmdArch::Rdna12;
    }
    if has(&[
        "vega", "rx 460", "rx 470", "rx 480", "rx 550", "rx 560", "rx 570", "rx 580", "rx 590",
    ]) {
        return AmdArch::PolarisVega;
    }
    AmdArch::Mainstream
}

/// Resolve the driver branch for a device: PCI device id first, model name as a
/// fallback. This is the single place the two classifiers are combined.
fn arch_for(device: &DeviceId) -> AmdArch {
    amd_arch_from_device_id(device.pci_device_id).unwrap_or_else(|| amd_arch(&device.model))
}

fn version_branch(version_attr: &str) -> AmdArch {
    let v = version_attr.to_lowercase();
    if v.contains("polaris and vega") {
        AmdArch::PolarisVega
    } else if v.contains("rdna1 and rdna2") {
        AmdArch::Rdna12
    } else {
        AmdArch::Mainstream
    }
}

fn public_version(version_attr: &str) -> &str {
    version_attr.split_whitespace().next().unwrap_or_default()
}

const AMD_INSTALLER_BASE: &str = "https://drivers.amd.com/drivers/";

/// Construct the Adrenalin installer `.exe` URL from the public version + branch.
/// Verified pattern (stable since 25.10.2): `whql-amd-software-adrenalin-edition-
/// {ver}-{variant}.exe`, variant = `win11-c` (combined, RDNA3+) / `win11-a`
/// (RDNA1/2). Beta/Optional drops the `whql-` prefix. Polaris/Vega has no
/// deterministic URL → empty string, so the UI falls back to a manual download
/// from the release-notes page. The download MUST send `Referer: amd.com` (set in
/// the install command) or the CDN 302s to a download-incomplete page.
fn build_installer_url(public_version: &str, arch: AmdArch, is_beta: bool) -> String {
    let variant = match arch {
        AmdArch::Mainstream => "win11-c",
        AmdArch::Rdna12 => "win11-a",
        AmdArch::PolarisVega => return String::new(),
    };
    let prefix = if is_beta { "" } else { "whql-" };
    format!(
        "{AMD_INSTALLER_BASE}{prefix}amd-software-adrenalin-edition-{public_version}-{variant}.exe"
    )
}

fn child_text<'a>(node: roxmltree::Node<'a, 'a>, tag: &str) -> Option<&'a str> {
    node.children()
        .find(|c| c.has_tag_name(tag))
        .and_then(|c| c.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// Pure parser over `amdversions.xml`. The document is newest-first, so the first
/// Windows `<driver>` matching the device's architecture branch is the latest.
pub fn parse_version_table(xml: &str, arch: AmdArch) -> Result<Option<DriverRelease>, DriverError> {
    Ok(parse_version_table_history(xml, arch)?.into_iter().next())
}

/// Pure parser returning every Windows `<driver>` matching the requested
/// architecture branch, newest-first, deduped by public version string. The
/// document already orders releases newest-first; this preserves that ordering
/// and never re-sorts.
pub fn parse_version_table_history(
    xml: &str,
    arch: AmdArch,
) -> Result<Vec<DriverRelease>, DriverError> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| DriverError::Parse(e.to_string()))?;
    let mut out = Vec::new();
    let mut seen_public = std::collections::BTreeSet::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("driver")) {
        if !node
            .attribute("operating-system")
            .map(|os| os.eq_ignore_ascii_case("Windows"))
            .unwrap_or(false)
        {
            continue;
        }
        let Some(version_attr) = node.attribute("version") else {
            continue;
        };
        if version_branch(version_attr) != arch {
            continue;
        }
        let Some(windows_version) = child_text(node, "windows-version") else {
            continue;
        };
        let public = public_version(version_attr);
        if !seen_public.insert(public.to_string()) {
            continue;
        }
        let is_beta = child_text(node, "whql")
            .map(|w| !w.eq_ignore_ascii_case("WHQL"))
            .unwrap_or(false);
        let release_notes = child_text(node, "download-url").map(str::to_string);
        out.push(DriverRelease {
            vendor: DriverVendor::Amd,
            version: DriverVersion::four_part_labeled(windows_version, public),
            channel: if is_beta {
                ReleaseChannel::Beta
            } else {
                ReleaseChannel::Stable
            },
            display_version: Some(public.to_string()),
            is_beta,
            download_url: build_installer_url(public, arch, is_beta),
            size_bytes: 0,
            signature_subject: c::PUBLISHER_SUBJECT.to_string(),
            released_at: child_text(node, "release-date").and_then(parse_amd_date),
            release_notes_url: release_notes,
            changelog: None,
        });
    }
    Ok(out)
}

fn parse_amd_date(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{NaiveDate, TimeZone, Utc};
    if let Ok(d) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(d.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%d", "%m/%d/%Y", "%B %d, %Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(raw, fmt) {
            return Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0)?).into();
        }
    }
    None
}

#[async_trait]
impl DriverSource for AmdGpuSource {
    fn id(&self) -> &'static str {
        "amd-gpu"
    }

    fn supports(&self, device: &DeviceId) -> bool {
        device.class == DeviceClass::Gpu && device.vendor == DriverVendor::Amd
    }

    async fn latest(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        _os: &OsTarget,
    ) -> Result<Option<DriverRelease>, DriverError> {
        let xml = fetch_version_table(client).await?;
        parse_version_table(&xml, arch_for(device))
    }

    async fn history(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        _os: &OsTarget,
        limit: usize,
    ) -> Result<Vec<DriverRelease>, DriverError> {
        let xml = fetch_version_table(client).await?;
        let mut releases = parse_version_table_history(&xml, arch_for(device))?;
        releases.truncate(limit);
        Ok(releases)
    }
}

async fn fetch_version_table(client: &reqwest::Client) -> Result<String, DriverError> {
    Ok(client
        .get(c::VERSION_TABLE_XML)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amd_device(device_id: u16, model: &str) -> DeviceId {
        DeviceId {
            class: DeviceClass::Gpu,
            vendor: DriverVendor::Amd,
            pci_vendor_id: 0x1002,
            pci_device_id: device_id,
            model: model.into(),
        }
    }

    #[test]
    fn installer_url_matches_verified_pattern() {
        assert_eq!(
            build_installer_url("26.6.1", AmdArch::Mainstream, false),
            "https://drivers.amd.com/drivers/whql-amd-software-adrenalin-edition-26.6.1-win11-c.exe"
        );
        assert_eq!(
            build_installer_url("26.6.1", AmdArch::Rdna12, false),
            "https://drivers.amd.com/drivers/whql-amd-software-adrenalin-edition-26.6.1-win11-a.exe"
        );
    }

    #[test]
    fn installer_url_beta_drops_whql_prefix() {
        assert_eq!(
            build_installer_url("26.2.1", AmdArch::Mainstream, true),
            "https://drivers.amd.com/drivers/amd-software-adrenalin-edition-26.2.1-win11-c.exe"
        );
    }

    #[test]
    fn installer_url_empty_for_polaris_vega() {
        assert!(build_installer_url("25.8.1", AmdArch::PolarisVega, false).is_empty());
    }

    #[test]
    fn amd_arch_from_device_id_classifies_legacy_branches() {
        assert_eq!(amd_arch_from_device_id(0x73BF), Some(AmdArch::Rdna12));
        assert_eq!(amd_arch_from_device_id(0x731F), Some(AmdArch::Rdna12));
        assert_eq!(amd_arch_from_device_id(0x67DF), Some(AmdArch::PolarisVega));
        assert_eq!(amd_arch_from_device_id(0x687F), Some(AmdArch::PolarisVega));
        assert_eq!(amd_arch_from_device_id(0x66AF), Some(AmdArch::PolarisVega));
    }

    #[test]
    fn amd_arch_from_device_id_returns_none_for_rdna3_plus_and_zero() {
        assert_eq!(amd_arch_from_device_id(0x744C), None);
        assert_eq!(amd_arch_from_device_id(0), None);
    }

    #[test]
    fn arch_for_prefers_device_id_over_name_then_falls_back() {
        assert_eq!(
            arch_for(&amd_device(0x67DF, "AMD Radeon Graphics")),
            AmdArch::PolarisVega
        );
        assert_eq!(
            arch_for(&amd_device(0, "AMD Radeon RX 6800 XT")),
            AmdArch::Rdna12
        );
        assert_eq!(
            arch_for(&amd_device(0x744C, "AMD Radeon RX 7900 XTX")),
            AmdArch::Mainstream
        );
    }

    #[test]
    fn supports_only_amd_gpus() {
        let source = AmdGpuSource;
        let amd = DeviceId {
            class: DeviceClass::Gpu,
            vendor: DriverVendor::Amd,
            pci_vendor_id: 0,
            pci_device_id: 0,
            model: "radeon rx 9070 xt".into(),
        };
        assert!(source.supports(&amd));
    }

    #[test]
    fn amd_arch_classifies_by_model() {
        assert_eq!(amd_arch("AMD Radeon RX 9070 XT"), AmdArch::Mainstream);
        assert_eq!(amd_arch("AMD Radeon RX 7900 XTX"), AmdArch::Mainstream);
        assert_eq!(amd_arch("AMD Radeon RX 6800 XT"), AmdArch::Rdna12);
        assert_eq!(amd_arch("AMD Radeon RX 5700 XT"), AmdArch::Rdna12);
        assert_eq!(amd_arch("AMD Radeon RX 580"), AmdArch::PolarisVega);
        assert_eq!(amd_arch("AMD Radeon Vega 64"), AmdArch::PolarisVega);
    }

    const FIXTURE: &str = r#"<?xml version="1.0"?>
    <root>
      <driver version="26.5.2 for Polaris and Vega" operating-system="Windows"><whql>WHQL</whql><download-url>https://amd/RN-RAD-WIN-26-5-2-polaris-vega.html</download-url><internal-version>23.19.25.01</internal-version><windows-version>31.0.21925.1001</windows-version><release-date>2026-05-14</release-date></driver>
      <driver version="26.5.2" operating-system="Windows"><whql>WHQL</whql><download-url>https://amd/RN-RAD-WIN-26-5-2.html</download-url><internal-version>26.10.07.05</internal-version><windows-version>32.0.31007.5012</windows-version><release-date>2026-05-14</release-date></driver>
      <driver version="26.5.2 for RDNA1 and RDNA2" operating-system="Windows"><whql>Optional</whql><download-url>https://amd/RN-RAD-WIN-26-5-2.html</download-url><windows-version>32.0.21043.10005</windows-version><release-date>2026-05-14</release-date></driver>
      <driver version="26.5.1" operating-system="Linux"><windows-version>0.0.0.0</windows-version></driver>
    </root>"#;

    #[test]
    fn picks_mainstream_branch_for_rdna3_plus() {
        let release = parse_version_table(FIXTURE, AmdArch::Mainstream)
            .unwrap()
            .expect("release");
        assert_eq!(release.vendor, DriverVendor::Amd);
        assert_eq!(release.version.display, "26.5.2");
        assert_eq!(release.version.raw, "32.0.31007.5012");
        assert_eq!(
            release.download_url,
            "https://drivers.amd.com/drivers/whql-amd-software-adrenalin-edition-26.5.2-win11-c.exe"
        );
        assert_eq!(
            release.release_notes_url.as_deref(),
            Some("https://amd/RN-RAD-WIN-26-5-2.html")
        );
        assert!(!release.is_beta);
        assert!(release.released_at.is_some());
    }

    #[test]
    fn picks_rdna12_branch_and_flags_optional_as_beta() {
        let release = parse_version_table(FIXTURE, AmdArch::Rdna12)
            .unwrap()
            .expect("release");
        assert_eq!(release.version.raw, "32.0.21043.10005");
        assert!(release.is_beta);
    }

    #[test]
    fn picks_polaris_vega_branch() {
        let release = parse_version_table(FIXTURE, AmdArch::PolarisVega)
            .unwrap()
            .expect("release");
        assert_eq!(release.version.raw, "31.0.21925.1001");
    }

    #[test]
    fn returns_none_when_no_windows_branch_matches_and_errors_on_bad_xml() {
        let no_match = r#"<root><driver version="1.0" operating-system="Linux"><windows-version>1.0.0.0</windows-version></driver></root>"#;
        assert!(parse_version_table(no_match, AmdArch::Mainstream)
            .unwrap()
            .is_none());
        assert!(parse_version_table("<unclosed", AmdArch::Mainstream).is_err());
    }

    const HISTORY_FIXTURE: &str = r#"<?xml version="1.0"?>
    <root>
      <driver version="26.5.2" operating-system="Windows"><whql>WHQL</whql><download-url>https://amd/RN-26-5-2.html</download-url><windows-version>32.0.31007.5012</windows-version><release-date>2026-05-14</release-date></driver>
      <driver version="26.5.2 for RDNA1 and RDNA2" operating-system="Windows"><whql>Optional</whql><download-url>https://amd/RN-26-5-2-rdna.html</download-url><windows-version>32.0.21043.10005</windows-version><release-date>2026-05-14</release-date></driver>
      <driver version="26.5.1" operating-system="Windows"><whql>WHQL</whql><download-url>https://amd/RN-26-5-1.html</download-url><windows-version>32.0.31007.4001</windows-version><release-date>2026-05-01</release-date></driver>
      <driver version="26.4.0" operating-system="Windows"><whql>WHQL</whql><download-url>https://amd/RN-26-4-0.html</download-url><windows-version>32.0.31000.7000</windows-version><release-date>2026-04-15</release-date></driver>
      <driver version="26.4.0 for RDNA1 and RDNA2" operating-system="Windows"><whql>WHQL</whql><download-url>https://amd/RN-26-4-0-rdna.html</download-url><windows-version>32.0.21040.7000</windows-version><release-date>2026-04-15</release-date></driver>
      <driver version="26.3.1 for Polaris and Vega" operating-system="Windows"><whql>WHQL</whql><download-url>https://amd/RN-26-3-1-pv.html</download-url><windows-version>31.0.21925.0500</windows-version><release-date>2026-03-15</release-date></driver>
      <driver version="26.3.0" operating-system="Linux"><windows-version>0.0.0.0</windows-version></driver>
    </root>"#;

    #[test]
    fn history_returns_all_mainstream_branch_versions_newest_first() {
        let releases = parse_version_table_history(HISTORY_FIXTURE, AmdArch::Mainstream).unwrap();
        let versions: Vec<_> = releases
            .iter()
            .map(|r| r.display_version.as_deref().unwrap())
            .collect();
        assert_eq!(versions, vec!["26.5.2", "26.5.1", "26.4.0"]);
        assert!(releases.iter().all(|r| !r.is_beta));
    }

    #[test]
    fn history_isolates_rdna12_branch_when_requested() {
        let releases = parse_version_table_history(HISTORY_FIXTURE, AmdArch::Rdna12).unwrap();
        let versions: Vec<_> = releases
            .iter()
            .map(|r| r.display_version.as_deref().unwrap())
            .collect();
        assert_eq!(versions, vec!["26.5.2", "26.4.0"]);
        assert!(releases[0].is_beta, "Optional channel should map to beta");
    }

    #[test]
    fn history_isolates_polaris_vega_branch() {
        let releases = parse_version_table_history(HISTORY_FIXTURE, AmdArch::PolarisVega).unwrap();
        let versions: Vec<_> = releases
            .iter()
            .map(|r| r.display_version.as_deref().unwrap())
            .collect();
        assert_eq!(versions, vec!["26.3.1"]);
    }

    #[test]
    fn history_dedupes_repeated_public_versions_keeping_first_match() {
        let xml = r#"<root>
          <driver version="26.5.2" operating-system="Windows"><whql>WHQL</whql><windows-version>32.0.31007.5012</windows-version></driver>
          <driver version="26.5.2" operating-system="Windows"><whql>Optional</whql><windows-version>32.0.31007.5099</windows-version></driver>
        </root>"#;
        let releases = parse_version_table_history(xml, AmdArch::Mainstream).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].version.raw, "32.0.31007.5012");
    }
}
