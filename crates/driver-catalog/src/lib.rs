pub mod consts;
pub mod sources;
pub mod version;

pub use sources::{DriverRegistry, DriverSource};
pub use version::DriverVersion;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeviceClass {
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DriverVendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsFamily {
    Windows10X64,
    Windows11X64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsTarget {
    pub family: OsFamily,
    pub dch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceId {
    pub class: DeviceClass,
    pub vendor: DriverVendor,
    pub pci_vendor_id: u16,
    pub pci_device_id: u16,
    pub model: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseChannel {
    Stable,
    Beta,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DriverChangelog {
    #[serde(default)]
    pub highlights: Vec<String>,
    #[serde(default)]
    pub fixed: Vec<String>,
    #[serde(default)]
    pub notes_page_url: Option<String>,
}

impl DriverChangelog {
    pub fn is_empty(&self) -> bool {
        self.highlights.is_empty() && self.fixed.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverRelease {
    pub vendor: DriverVendor,
    pub version: DriverVersion,
    pub channel: ReleaseChannel,
    #[serde(default)]
    pub display_version: Option<String>,
    #[serde(default)]
    pub is_beta: bool,
    pub download_url: String,
    pub size_bytes: u64,
    pub signature_subject: String,
    pub released_at: Option<chrono::DateTime<chrono::Utc>>,
    pub release_notes_url: Option<String>,
    #[serde(default)]
    pub changelog: Option<DriverChangelog>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    UpToDate,
    UpdateAvailable,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverStatusReport {
    pub device: DeviceId,
    pub installed: DriverVersion,
    pub latest: Option<DriverRelease>,
    pub status: UpdateStatus,
}

#[derive(Debug, thiserror::Error)]
pub enum DriverError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("parse: {0}")]
    Parse(String),
    #[error("no driver source for {class:?}/{vendor:?}")]
    NoSource {
        class: DeviceClass,
        vendor: DriverVendor,
    },
}

pub fn update_status(installed: &DriverVersion, latest: Option<&DriverRelease>) -> UpdateStatus {
    match latest {
        None => UpdateStatus::Unknown,
        Some(release) if installed.packed == 0 => {
            let _ = release;
            UpdateStatus::Unknown
        }
        Some(release) if release.version.is_newer_than(installed) => UpdateStatus::UpdateAvailable,
        Some(_) => UpdateStatus::UpToDate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_at(version: DriverVersion) -> DriverRelease {
        DriverRelease {
            vendor: DriverVendor::Nvidia,
            version,
            channel: ReleaseChannel::Stable,
            display_version: None,
            is_beta: false,
            download_url: "https://example.test/driver.exe".into(),
            size_bytes: 0,
            signature_subject: "NVIDIA Corporation".into(),
            released_at: None,
            release_notes_url: None,
            changelog: None,
        }
    }

    #[test]
    fn status_is_update_available_when_latest_is_newer() {
        let installed = DriverVersion::nvidia("572.16");
        let latest = release_at(DriverVersion::nvidia("572.83"));
        assert_eq!(
            update_status(&installed, Some(&latest)),
            UpdateStatus::UpdateAvailable
        );
    }

    #[test]
    fn status_is_up_to_date_when_equal_or_older() {
        let installed = DriverVersion::nvidia("572.83");
        let same = release_at(DriverVersion::nvidia("572.83"));
        let older = release_at(DriverVersion::nvidia("566.36"));
        assert_eq!(
            update_status(&installed, Some(&same)),
            UpdateStatus::UpToDate
        );
        assert_eq!(
            update_status(&installed, Some(&older)),
            UpdateStatus::UpToDate
        );
    }

    #[test]
    fn status_is_unknown_without_latest_or_without_installed() {
        let installed = DriverVersion::nvidia("572.16");
        assert_eq!(update_status(&installed, None), UpdateStatus::Unknown);
        let unknown_installed = DriverVersion::unknown();
        let latest = release_at(DriverVersion::nvidia("572.16"));
        assert_eq!(
            update_status(&unknown_installed, Some(&latest)),
            UpdateStatus::Unknown
        );
    }
}
