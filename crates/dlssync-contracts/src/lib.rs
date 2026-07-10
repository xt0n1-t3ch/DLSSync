use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum DistributionChannel {
    Standard,
    Nexus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    Installed,
    Portable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CatalogRefreshTrigger {
    Automatic,
    ManualUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CatalogDelta {
    pub added: u32,
    pub updated: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CatalogProvenance {
    pub manifest_url: String,
    pub manifest_repository: String,
    pub generated_at: String,
    pub checked_at: String,
    pub signature_verified: bool,
    pub public_key_fingerprint: String,
    pub source_commit: Option<String>,
    pub trigger: CatalogRefreshTrigger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CatalogRefreshResult {
    pub refreshed: bool,
    pub blocked_by_policy: bool,
    pub provenance: CatalogProvenance,
    pub delta: CatalogDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CatalogStatus {
    pub distribution: DistributionChannel,
    pub install_mode: InstallMode,
    pub automatic_refresh_enabled: bool,
    pub manual_refresh_enabled: bool,
    pub app_updates_enabled: bool,
    pub provenance: CatalogProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum OperationActor {
    Gui,
    Cli,
    Background,
}

impl OperationActor {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gui => "gui",
            Self::Cli => "cli",
            Self::Background => "background",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Scan,
    CatalogRefresh,
    Plan,
    DllApply,
    Rollback,
    DriverInstall,
}

impl OperationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scan => "scan",
            Self::CatalogRefresh => "catalog_refresh",
            Self::Plan => "plan",
            Self::DllApply => "dll_apply",
            Self::Rollback => "rollback",
            Self::DriverInstall => "driver_install",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Started,
    Succeeded,
    Failed,
    Cancelled,
}

impl OperationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct OperationRecord {
    pub id: String,
    pub created_at: String,
    pub actor: OperationActor,
    pub kind: OperationKind,
    pub status: OperationStatus,
    pub target: Option<String>,
    pub summary: String,
    pub details: BTreeMap<String, String>,
    pub duration_ms: Option<u32>,
    pub backup_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct JournalFilter {
    pub target: Option<String>,
    pub kind: Option<OperationKind>,
    pub status: Option<OperationStatus>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct TrustEvidence {
    pub source_url: String,
    pub expected_sha256: String,
    pub observed_sha256: Option<String>,
    pub signature_subject: Option<String>,
    pub signature_verified: bool,
    pub anti_cheat_risk: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct UpdatePlanItem {
    pub id: String,
    pub game_id: String,
    pub game_name: String,
    pub dll_path: String,
    pub family: String,
    pub current_version: Option<String>,
    pub target_version: String,
    pub backup_path: String,
    pub selected: bool,
    pub trust: TrustEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct UpdatePlan {
    pub id: String,
    pub created_at: String,
    pub catalog_generated_at: String,
    pub fingerprint: String,
    pub stale: bool,
    pub items: Vec<UpdatePlanItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ScannedComponent {
    pub family: String,
    pub path: String,
    pub current_version: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ScannedGame {
    pub id: String,
    pub name: String,
    pub launcher: String,
    pub install_dir: String,
    pub components: Vec<ScannedComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ApplyPlanResult {
    pub plan_id: String,
    pub applied: u32,
    pub backup_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct RollbackPlanResult {
    pub plan_id: String,
    pub restored: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub context: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_serialize_to_stable_snake_case_values() {
        assert_eq!(
            serde_json::to_string(&CatalogRefreshTrigger::ManualUser).unwrap(),
            "\"manual_user\""
        );
        assert_eq!(
            serde_json::to_string(&OperationKind::CatalogRefresh).unwrap(),
            "\"catalog_refresh\""
        );
    }
}
