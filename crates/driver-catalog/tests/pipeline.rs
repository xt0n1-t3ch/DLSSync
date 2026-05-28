//! End-to-end resolution pipeline across the three GPU families. Drives the
//! `DriverRegistry` orchestration (vendor routing → latest → `update_status` →
//! report, plus deduped history) with a fake source so the path is exercised
//! deterministically without touching the network. The live HTTP/parse layer is
//! covered by each source's own unit tests, and download/verify/launch by
//! `driver-install`'s wiremock + state-machine tests.

use async_trait::async_trait;
use driver_catalog::sources::DriverSource;
use driver_catalog::{
    DeviceClass, DeviceId, DriverError, DriverRegistry, DriverRelease, DriverVendor, DriverVersion,
    OsFamily, OsTarget, ReleaseChannel, UpdateStatus,
};

struct FakeSource {
    vendor: DriverVendor,
    releases: Vec<DriverRelease>,
}

#[async_trait]
impl DriverSource for FakeSource {
    fn id(&self) -> &'static str {
        "fake"
    }

    fn supports(&self, device: &DeviceId) -> bool {
        device.class == DeviceClass::Gpu && device.vendor == self.vendor
    }

    async fn latest(
        &self,
        _client: &reqwest::Client,
        _device: &DeviceId,
        _os: &OsTarget,
    ) -> Result<Option<DriverRelease>, DriverError> {
        Ok(self.releases.first().cloned())
    }

    async fn history(
        &self,
        _client: &reqwest::Client,
        _device: &DeviceId,
        _os: &OsTarget,
        limit: usize,
    ) -> Result<Vec<DriverRelease>, DriverError> {
        Ok(self.releases.iter().take(limit).cloned().collect())
    }
}

fn release(vendor: DriverVendor, version: DriverVersion) -> DriverRelease {
    DriverRelease {
        vendor,
        version,
        channel: ReleaseChannel::Stable,
        display_version: None,
        is_beta: false,
        download_url: "https://vendor/driver.exe".into(),
        size_bytes: 1,
        signature_subject: "subject".into(),
        released_at: None,
        release_notes_url: Some("https://vendor/notes".into()),
        changelog: None,
    }
}

fn device(vendor: DriverVendor, device_id: u16, model: &str) -> DeviceId {
    DeviceId {
        class: DeviceClass::Gpu,
        vendor,
        pci_vendor_id: 0,
        pci_device_id: device_id,
        model: model.into(),
    }
}

fn os() -> OsTarget {
    OsTarget {
        family: OsFamily::Windows11X64,
        dch: true,
    }
}

fn registry() -> DriverRegistry {
    let mut registry = DriverRegistry::empty();
    registry.register(Box::new(FakeSource {
        vendor: DriverVendor::Nvidia,
        releases: vec![release(
            DriverVendor::Nvidia,
            DriverVersion::nvidia("610.47"),
        )],
    }));
    registry.register(Box::new(FakeSource {
        vendor: DriverVendor::Amd,
        releases: vec![release(
            DriverVendor::Amd,
            DriverVersion::four_part_labeled("32.0.31007.5012", "26.5.2"),
        )],
    }));
    registry.register(Box::new(FakeSource {
        vendor: DriverVendor::Intel,
        releases: vec![
            release(
                DriverVendor::Intel,
                DriverVersion::four_part("32.0.101.8801"),
            ),
            release(
                DriverVendor::Intel,
                DriverVersion::four_part("32.0.101.8626"),
            ),
            release(
                DriverVendor::Intel,
                DriverVersion::four_part("32.0.101.8801"),
            ),
        ],
    }));
    registry
}

#[tokio::test]
async fn nvidia_pipeline_reports_update_available_when_latest_is_newer() {
    let client = reqwest::Client::new();
    let report = registry()
        .resolve(
            &client,
            &device(
                DriverVendor::Nvidia,
                0x2705,
                "NVIDIA GeForce RTX 4070 Ti SUPER",
            ),
            &os(),
            DriverVersion::nvidia("591.74"),
        )
        .await
        .expect("resolve");
    assert_eq!(report.status, UpdateStatus::UpdateAvailable);
    assert_eq!(report.latest.unwrap().version.display, "610.47");
}

#[tokio::test]
async fn amd_pipeline_reports_up_to_date_when_installed_matches_latest() {
    let client = reqwest::Client::new();
    let report = registry()
        .resolve(
            &client,
            &device(DriverVendor::Amd, 0x73BF, "AMD Radeon RX 6900 XT"),
            &os(),
            DriverVersion::four_part("32.0.31007.5012"),
        )
        .await
        .expect("resolve");
    assert_eq!(report.status, UpdateStatus::UpToDate);
}

#[tokio::test]
async fn intel_pipeline_is_unknown_when_installed_version_is_unparseable() {
    let client = reqwest::Client::new();
    let report = registry()
        .resolve(
            &client,
            &device(DriverVendor::Intel, 0x9A49, "Intel Iris Xe Graphics"),
            &os(),
            DriverVersion::unknown(),
        )
        .await
        .expect("resolve");
    assert_eq!(report.status, UpdateStatus::Unknown);
    assert!(
        report.latest.is_some(),
        "a candidate release is still surfaced"
    );
}

#[tokio::test]
async fn unsupported_vendor_resolves_without_a_source() {
    let client = reqwest::Client::new();
    let report = registry()
        .resolve(
            &client,
            &device(DriverVendor::Other, 0x1234, "Some other GPU"),
            &os(),
            DriverVersion::four_part("1.0.0.0"),
        )
        .await
        .expect("resolve");
    assert_eq!(report.status, UpdateStatus::Unsupported);
    assert!(report.latest.is_none());
}

#[tokio::test]
async fn history_pipeline_dedupes_by_version_and_honours_the_limit() {
    let client = reqwest::Client::new();
    let history = registry()
        .history(
            &client,
            &device(DriverVendor::Intel, 0x9A49, "Intel Iris Xe Graphics"),
            &os(),
            10,
        )
        .await
        .expect("history");
    let versions: Vec<_> = history.iter().map(|r| r.version.display.as_str()).collect();
    assert_eq!(versions, vec!["32.0.101.8801", "32.0.101.8626"]);
}
