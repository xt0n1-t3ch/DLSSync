use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

use backup_store::BackupStore;
use dll_catalog::Catalog;

use crate::commands::settings::AppSettings;
use crate::paths::AppPaths;
use crate::system_info::SystemInfo;

pub struct AppState {
    pub catalog: Arc<RwLock<Option<Catalog>>>,
    pub catalog_cache_path: Arc<RwLock<Option<PathBuf>>>,
    pub backups: Arc<RwLock<Option<BackupStore>>>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub paths: Arc<RwLock<Option<AppPaths>>>,
    pub system_info: Arc<RwLock<Option<SystemInfo>>>,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("DLSSync/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self {
            catalog: Arc::new(RwLock::new(None)),
            catalog_cache_path: Arc::new(RwLock::new(None)),
            backups: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(AppSettings::default())),
            paths: Arc::new(RwLock::new(None)),
            system_info: Arc::new(RwLock::new(None)),
            http,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
