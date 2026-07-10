use chrono::Utc;
use dlssync_contracts::{
    CatalogRefreshTrigger, DistributionChannel, InstallMode, UpdatePlan, UpdatePlanItem,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub mod execution;
pub mod ports;
pub mod scan;
pub use execution::{apply_update_plan, rollback_update_plan, ExecutionError};
pub use scan::{plan_items, scan_installed_games, scan_path, ScanUseCaseError};

const PRODUCT_CONFIG: &str = include_str!("../../../product.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct ProductConfig {
    pub product: ProductIdentity,
    pub catalog: CatalogConfig,
    pub links: ProductLinks,
    pub distribution: DistributionConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductIdentity {
    pub name: String,
    pub repository: String,
    pub manifest_repository: String,
    pub nexus: String,
    pub homepage: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogConfig {
    pub canonical_manifest: String,
    pub signature_suffix: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductLinks {
    pub releases: String,
    pub releases_latest: String,
    pub issues: String,
    pub new_issue: String,
    pub author: String,
    pub sponsor: String,
    pub kofi: String,
    pub anticheat_faq: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistributionConfig {
    pub standard: ChannelConfig,
    pub nexus: ChannelConfig,
    pub portable: PortableConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelConfig {
    pub app_updates: bool,
    pub automatic_catalog_refresh: bool,
    pub manual_catalog_refresh: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PortableConfig {
    pub app_updates: bool,
    pub automatic_catalog_refresh: bool,
    pub manual_catalog_refresh: bool,
    pub data_marker: String,
}

pub fn product_config() -> Result<ProductConfig, toml::de::Error> {
    toml::from_str(PRODUCT_CONFIG)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistributionPolicy {
    pub channel: DistributionChannel,
    pub install_mode: InstallMode,
    pub app_updates: bool,
    pub automatic_catalog_refresh: bool,
    pub manual_catalog_refresh: bool,
}

impl DistributionPolicy {
    pub fn resolve(
        config: &ProductConfig,
        channel: DistributionChannel,
        install_mode: InstallMode,
    ) -> Self {
        let channel_config = match channel {
            DistributionChannel::Standard => &config.distribution.standard,
            DistributionChannel::Nexus => &config.distribution.nexus,
        };
        let values = match install_mode {
            // Portable keeps its data beside the exe, but it must still honor the channel's
            // network policy: a Nexus build carrying portable.flag must not silently re-enable
            // automatic catalog refresh. Take the more restrictive of the two capability sets.
            InstallMode::Portable => {
                let portable = &config.distribution.portable;
                ChannelConfig {
                    app_updates: portable.app_updates && channel_config.app_updates,
                    automatic_catalog_refresh: portable.automatic_catalog_refresh
                        && channel_config.automatic_catalog_refresh,
                    manual_catalog_refresh: portable.manual_catalog_refresh
                        && channel_config.manual_catalog_refresh,
                }
            }
            InstallMode::Installed => channel_config.clone(),
        };
        Self {
            channel,
            install_mode,
            app_updates: values.app_updates,
            automatic_catalog_refresh: values.automatic_catalog_refresh,
            manual_catalog_refresh: values.manual_catalog_refresh,
        }
    }

    pub const fn permits_catalog_refresh(self, trigger: CatalogRefreshTrigger) -> bool {
        match trigger {
            CatalogRefreshTrigger::Automatic => self.automatic_catalog_refresh,
            CatalogRefreshTrigger::ManualUser => self.manual_catalog_refresh,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRootResolution {
    pub mode: InstallMode,
    pub root: PathBuf,
}

pub fn resolve_data_root(
    executable: &Path,
    installed_root: PathBuf,
    explicit_override: Option<PathBuf>,
    portable_marker: &str,
) -> DataRootResolution {
    if let Some(root) = explicit_override {
        return DataRootResolution {
            mode: InstallMode::Portable,
            root,
        };
    }
    let executable_dir = executable.parent().unwrap_or_else(|| Path::new("."));
    if executable_dir.join(portable_marker).is_file() {
        return DataRootResolution {
            mode: InstallMode::Portable,
            root: executable_dir.join("data"),
        };
    }
    DataRootResolution {
        mode: InstallMode::Installed,
        root: installed_root,
    }
}

pub fn build_update_plan(catalog_generated_at: &str, mut items: Vec<UpdatePlanItem>) -> UpdatePlan {
    items.sort_by(|a, b| a.id.cmp(&b.id));
    let fingerprint = plan_fingerprint(catalog_generated_at, &items);
    UpdatePlan {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        catalog_generated_at: catalog_generated_at.to_string(),
        fingerprint,
        stale: false,
        items,
    }
}

pub fn build_update_plan_at(
    catalog_generated_at: &str,
    mut items: Vec<UpdatePlanItem>,
    backup_root: &Path,
) -> UpdatePlan {
    let id = uuid::Uuid::new_v4().to_string();
    for item in &mut items {
        let filename = Path::new(&item.dll_path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("component.dll"));
        item.backup_path = backup_root
            .join(&id)
            .join(&item.game_id)
            .join(filename)
            .display()
            .to_string();
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    let fingerprint = plan_fingerprint(catalog_generated_at, &items);
    UpdatePlan {
        id,
        created_at: Utc::now().to_rfc3339(),
        catalog_generated_at: catalog_generated_at.to_string(),
        fingerprint,
        stale: false,
        items,
    }
}

pub fn mark_plan_stale(
    plan: &mut UpdatePlan,
    catalog_generated_at: &str,
    items: &[UpdatePlanItem],
) {
    plan.stale = plan.fingerprint != plan_fingerprint(catalog_generated_at, items);
}

fn plan_fingerprint(catalog_generated_at: &str, items: &[UpdatePlanItem]) -> String {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    let mut hasher = Sha256::new();
    hasher.update(catalog_generated_at.as_bytes());
    hasher.update([0]);
    hasher.update(serde_json::to_vec(&sorted).expect("update plan items serialize"));
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dlssync_contracts::TrustEvidence;
    use tempfile::tempdir;

    fn item(id: &str, version: &str) -> UpdatePlanItem {
        UpdatePlanItem {
            id: id.into(),
            game_id: "game".into(),
            game_name: "Game".into(),
            dll_path: format!("C:/Game/{id}.dll"),
            family: "dlss_sr".into(),
            current_version: Some("1.0.0".into()),
            target_version: version.into(),
            backup_path: format!("C:/Backup/{id}.dll"),
            selected: true,
            trust: TrustEvidence {
                source_url: "https://vendor.example/release.zip".into(),
                expected_sha256: "a".repeat(64),
                observed_sha256: None,
                signature_subject: Some("NVIDIA Corporation".into()),
                signature_verified: false,
                anti_cheat_risk: None,
            },
        }
    }

    #[test]
    fn product_config_is_complete_and_https_only() {
        let config = product_config().unwrap();
        assert_eq!(config.product.name, "DLSSync");
        assert!(config.catalog.canonical_manifest.starts_with("https://"));
        assert_eq!(config.catalog.signature_suffix, ".sig");
        assert!(config
            .links
            .releases_latest
            .starts_with(&config.product.repository));
        assert!(config.links.new_issue.starts_with(&config.links.issues));
    }

    #[test]
    fn nexus_blocks_automatic_but_allows_manual_catalog_refresh() {
        let config = product_config().unwrap();
        let policy = DistributionPolicy::resolve(
            &config,
            DistributionChannel::Nexus,
            InstallMode::Installed,
        );
        assert!(!policy.app_updates);
        assert!(!policy.permits_catalog_refresh(CatalogRefreshTrigger::Automatic));
        assert!(policy.permits_catalog_refresh(CatalogRefreshTrigger::ManualUser));
    }

    #[test]
    fn standard_allows_both_refresh_modes() {
        let config = product_config().unwrap();
        let policy = DistributionPolicy::resolve(
            &config,
            DistributionChannel::Standard,
            InstallMode::Installed,
        );
        assert!(policy.app_updates);
        assert!(policy.permits_catalog_refresh(CatalogRefreshTrigger::Automatic));
        assert!(policy.permits_catalog_refresh(CatalogRefreshTrigger::ManualUser));
    }

    #[test]
    fn portable_nexus_still_blocks_automatic_catalog_refresh() {
        let config = product_config().unwrap();
        let policy =
            DistributionPolicy::resolve(&config, DistributionChannel::Nexus, InstallMode::Portable);
        assert!(!policy.app_updates);
        assert!(!policy.permits_catalog_refresh(CatalogRefreshTrigger::Automatic));
        assert!(policy.permits_catalog_refresh(CatalogRefreshTrigger::ManualUser));
    }

    #[test]
    fn portable_standard_keeps_automatic_catalog_refresh() {
        let config = product_config().unwrap();
        let policy = DistributionPolicy::resolve(
            &config,
            DistributionChannel::Standard,
            InstallMode::Portable,
        );
        assert!(policy.permits_catalog_refresh(CatalogRefreshTrigger::Automatic));
        assert!(policy.permits_catalog_refresh(CatalogRefreshTrigger::ManualUser));
    }

    #[test]
    fn portable_marker_routes_everything_beside_executable() {
        let dir = tempdir().unwrap();
        let exe = dir.path().join("DLSSync.exe");
        std::fs::write(&exe, b"").unwrap();
        std::fs::write(dir.path().join("portable.flag"), b"").unwrap();
        let resolution = resolve_data_root(
            &exe,
            PathBuf::from("C:/Users/Test/DLSSync"),
            None,
            "portable.flag",
        );
        assert_eq!(resolution.mode, InstallMode::Portable);
        assert_eq!(resolution.root, dir.path().join("data"));
    }

    #[test]
    fn plan_fingerprint_is_order_independent_and_detects_drift() {
        let mut plan =
            build_update_plan("2026-07-10T00:00:00Z", vec![item("b", "2"), item("a", "2")]);
        let same = vec![item("a", "2"), item("b", "2")];
        mark_plan_stale(&mut plan, "2026-07-10T00:00:00Z", &same);
        assert!(!plan.stale);
        let changed = vec![item("a", "3"), item("b", "2")];
        mark_plan_stale(&mut plan, "2026-07-10T00:00:00Z", &changed);
        assert!(plan.stale);
    }
}
