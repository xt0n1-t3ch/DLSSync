use serde::ser::SerializeStruct;
use serde::Serialize;

/// Top-level error type returned by all Tauri commands.
///
/// Serializes to `{ kind: "<variant>", message: "<human text>" }` so the
/// frontend can branch on `kind` for calm error messaging (e.g. `"validation"`
/// shows a field-level hint, `"catalog"` shows a reload prompt).
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("launcher scan failed: {0}")]
    Launcher(#[from] launcher_scan::ScanError),

    #[error("dll scan failed: {0}")]
    DllScan(#[from] dll_scanner::ScanError),

    #[error("catalog error: {0}")]
    Catalog(#[from] dll_catalog::CatalogError),

    #[error("pe parse error: {0}")]
    PeVersion(#[from] pe_version::VersionError),

    #[error("backup error: {0}")]
    Backup(#[from] backup_store::BackupError),

    #[error("notifications error: {0}")]
    Notifications(#[from] notifications_store::NotificationsError),

    /// Input failed a security or integrity check (path traversal, URL allowlist,
    /// update-ID format, etc.). `kind = "validation"` on the wire — the frontend
    /// surfaces a calm hint instead of a generic error modal.
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let kind = match self {
            AppError::Io(_) => "io",
            AppError::Launcher(_) => "launcher",
            AppError::DllScan(_) => "dll_scan",
            AppError::Catalog(_) => "catalog",
            AppError::PeVersion(_) => "pe_version",
            AppError::Backup(_) => "backup",
            AppError::Notifications(_) => "notifications",
            AppError::Validation(_) => "validation",
            AppError::Other(_) => "other",
        };
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", kind)?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

/// Convenience alias used by every Tauri command handler.
pub type AppResult<T> = Result<T, AppError>;
