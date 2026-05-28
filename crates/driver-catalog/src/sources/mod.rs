pub mod amd;
pub mod intel;
pub mod nvidia;

use crate::{
    update_status, DeviceId, DriverError, DriverRelease, DriverStatusReport, DriverVersion,
    OsTarget, UpdateStatus,
};
use async_trait::async_trait;

/// Default number of historical drivers requested per GPU when callers do not
/// pick their own. NVIDIA's Ajax service caps around 50; AMD and Intel typically
/// have fewer rows. This keeps the report bounded without truncating real
/// history.
pub const DEFAULT_HISTORY_LIMIT: usize = 50;

#[async_trait]
pub trait DriverSource: Send + Sync {
    fn id(&self) -> &'static str;
    fn supports(&self, device: &DeviceId) -> bool;
    async fn latest(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        os: &OsTarget,
    ) -> Result<Option<DriverRelease>, DriverError>;

    /// Historical drivers known compatible with this exact device on this OS,
    /// newest-first, deduped by version. Implementations that cannot enumerate
    /// history fall through to the default below, which wraps `latest()`.
    async fn history(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        os: &OsTarget,
        _limit: usize,
    ) -> Result<Vec<DriverRelease>, DriverError> {
        Ok(self.latest(client, device, os).await?.into_iter().collect())
    }
}

pub struct DriverRegistry {
    sources: Vec<Box<dyn DriverSource>>,
}

impl DriverRegistry {
    pub fn empty() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn with_default_gpu_sources() -> Self {
        let mut registry = Self::empty();
        registry.register(Box::new(nvidia::NvidiaGpuSource));
        registry.register(Box::new(intel::IntelGpuSource));
        registry.register(Box::new(amd::AmdGpuSource));
        registry
    }

    pub fn register(&mut self, source: Box<dyn DriverSource>) {
        self.sources.push(source);
    }

    pub fn source_for(&self, device: &DeviceId) -> Option<&dyn DriverSource> {
        self.sources
            .iter()
            .map(|source| source.as_ref())
            .find(|source| source.supports(device))
    }

    pub async fn resolve(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        os: &OsTarget,
        installed: DriverVersion,
    ) -> Result<DriverStatusReport, DriverError> {
        let Some(source) = self.source_for(device) else {
            return Ok(DriverStatusReport {
                device: device.clone(),
                installed,
                latest: None,
                status: UpdateStatus::Unsupported,
            });
        };
        let latest = source.latest(client, device, os).await?;
        let status = update_status(&installed, latest.as_ref());
        Ok(DriverStatusReport {
            device: device.clone(),
            installed,
            latest,
            status,
        })
    }

    /// Historical compatible drivers for the device, newest-first, deduped by
    /// version, capped at `limit`. Returns an empty Vec when no source backs
    /// the vendor.
    pub async fn history(
        &self,
        client: &reqwest::Client,
        device: &DeviceId,
        os: &OsTarget,
        limit: usize,
    ) -> Result<Vec<DriverRelease>, DriverError> {
        let Some(source) = self.source_for(device) else {
            return Ok(Vec::new());
        };
        let mut releases = source.history(client, device, os, limit).await?;
        dedupe_by_version(&mut releases);
        releases.truncate(limit);
        Ok(releases)
    }
}

/// In-place: drop entries whose `version.display` already appeared earlier.
/// Cheap because real driver histories are at most ~50 rows.
fn dedupe_by_version(releases: &mut Vec<DriverRelease>) {
    let mut seen = std::collections::BTreeSet::new();
    releases.retain(|r| seen.insert(r.version.display.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceClass, DriverVendor};

    fn gpu(vendor: DriverVendor) -> DeviceId {
        DeviceId {
            class: DeviceClass::Gpu,
            vendor,
            pci_vendor_id: 0,
            pci_device_id: 0,
            model: "test gpu".into(),
        }
    }

    #[test]
    fn default_registry_routes_each_gpu_vendor_to_its_source() {
        let registry = DriverRegistry::with_default_gpu_sources();
        assert_eq!(
            registry
                .source_for(&gpu(DriverVendor::Nvidia))
                .map(|s| s.id()),
            Some("nvidia-gpu")
        );
        assert_eq!(
            registry
                .source_for(&gpu(DriverVendor::Intel))
                .map(|s| s.id()),
            Some("intel-gpu")
        );
        assert_eq!(
            registry.source_for(&gpu(DriverVendor::Amd)).map(|s| s.id()),
            Some("amd-gpu")
        );
    }

    #[test]
    fn unsupported_vendor_resolves_to_no_source() {
        let registry = DriverRegistry::with_default_gpu_sources();
        assert!(registry.source_for(&gpu(DriverVendor::Other)).is_none());
    }

    #[test]
    fn registration_order_decides_first_match() {
        let mut registry = DriverRegistry::empty();
        registry.register(Box::new(nvidia::NvidiaGpuSource));
        assert_eq!(
            registry
                .source_for(&gpu(DriverVendor::Nvidia))
                .map(|s| s.id()),
            Some("nvidia-gpu")
        );
        assert!(registry.source_for(&gpu(DriverVendor::Amd)).is_none());
    }

    #[test]
    fn dedupe_by_version_keeps_first_occurrence_only() {
        use crate::{DriverChangelog, ReleaseChannel};
        let mk = |v: &str| DriverRelease {
            vendor: DriverVendor::Nvidia,
            version: DriverVersion::nvidia(v),
            channel: ReleaseChannel::Stable,
            display_version: None,
            is_beta: false,
            download_url: format!("https://x/{v}.exe"),
            size_bytes: 0,
            signature_subject: "NVIDIA Corporation".into(),
            released_at: None,
            release_notes_url: None,
            changelog: Some(DriverChangelog::default()),
        };
        let mut list = vec![mk("610.47"), mk("596.49"), mk("610.47"), mk("596.36")];
        dedupe_by_version(&mut list);
        let versions: Vec<_> = list.iter().map(|r| r.version.display.as_str()).collect();
        assert_eq!(versions, vec!["610.47", "596.49", "596.36"]);
    }
}
