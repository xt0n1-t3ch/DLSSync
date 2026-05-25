use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use backup_store::BackupStore;
use dll_catalog::{Catalog, DownloadCache};
use notifications_store::NotificationsStore;

use crate::commands::settings::AppSettings;
use crate::paths::AppPaths;
use crate::system_info::SystemInfo;

const UA: &str = concat!("DLSSync/", env!("CARGO_PKG_VERSION"));

const HTTP_CATALOG_TIMEOUT_SECS: u64 = 15;
const HTTP_CATALOG_REDIRECTS: usize = 5;
const HTTP_ART_TIMEOUT_SECS: u64 = 8;
const HTTP_ART_REDIRECTS: usize = 5;
const HTTP_DOWNLOAD_CONNECT_TIMEOUT_SECS: u64 = 10;
const HTTP_DOWNLOAD_REDIRECTS: usize = 10;
const HTTP_DOWNLOAD_POOL_IDLE_PER_HOST: usize = 2;

#[derive(Default)]
pub struct ApplyRegistry {
    inner: Mutex<HashMap<String, CancellationToken>>,
}

impl ApplyRegistry {
    pub fn register(&self, apply_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        self.inner
            .lock()
            .insert(apply_id.to_string(), token.clone());
        token
    }

    pub fn cancel(&self, apply_id: &str) -> bool {
        if let Some(token) = self.inner.lock().get(apply_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub fn cancel_all(&self) -> usize {
        let guard = self.inner.lock();
        for token in guard.values() {
            token.cancel();
        }
        guard.len()
    }

    pub fn release(&self, apply_id: &str) {
        self.inner.lock().remove(apply_id);
    }

    pub fn in_flight(&self) -> usize {
        self.inner.lock().len()
    }
}

pub struct AppState {
    pub catalog: Arc<RwLock<Option<Catalog>>>,
    pub catalog_cache_path: Arc<RwLock<Option<PathBuf>>>,
    pub backups: Arc<RwLock<Option<BackupStore>>>,
    pub notifications: Arc<RwLock<Option<NotificationsStore>>>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub paths: Arc<RwLock<Option<AppPaths>>>,
    pub system_info: Arc<RwLock<Option<SystemInfo>>>,
    pub http_catalog: reqwest::Client,
    pub http_downloads: reqwest::Client,
    pub http_art: reqwest::Client,
    pub download_cache: Arc<DownloadCache>,
    pub apply_registry: Arc<ApplyRegistry>,
}

impl AppState {
    pub fn new() -> Self {
        let http_catalog = reqwest::Client::builder()
            .user_agent(UA)
            .timeout(Duration::from_secs(HTTP_CATALOG_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::limited(HTTP_CATALOG_REDIRECTS))
            .build()
            .expect("reqwest catalog client");
        let http_downloads = reqwest::Client::builder()
            .user_agent(UA)
            .connect_timeout(Duration::from_secs(HTTP_DOWNLOAD_CONNECT_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::limited(HTTP_DOWNLOAD_REDIRECTS))
            .pool_max_idle_per_host(HTTP_DOWNLOAD_POOL_IDLE_PER_HOST)
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .build()
            .expect("reqwest downloads client");
        let http_art = reqwest::Client::builder()
            .user_agent(UA)
            .timeout(Duration::from_secs(HTTP_ART_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::limited(HTTP_ART_REDIRECTS))
            .build()
            .expect("reqwest art client");
        Self {
            catalog: Arc::new(RwLock::new(None)),
            catalog_cache_path: Arc::new(RwLock::new(None)),
            backups: Arc::new(RwLock::new(None)),
            notifications: Arc::new(RwLock::new(None)),
            settings: Arc::new(RwLock::new(AppSettings::default())),
            paths: Arc::new(RwLock::new(None)),
            system_info: Arc::new(RwLock::new(None)),
            http_catalog,
            http_downloads,
            http_art,
            download_cache: Arc::new(DownloadCache::new()),
            apply_registry: Arc::new(ApplyRegistry::default()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_register_and_cancel() {
        let r = ApplyRegistry::default();
        let token = r.register("apply-1");
        assert!(!token.is_cancelled());
        assert_eq!(r.in_flight(), 1);
        assert!(r.cancel("apply-1"));
        assert!(token.is_cancelled());
    }

    #[test]
    fn registry_cancel_unknown_returns_false() {
        let r = ApplyRegistry::default();
        assert!(!r.cancel("not-there"));
    }

    #[test]
    fn registry_cancel_all_flips_every_token() {
        let r = ApplyRegistry::default();
        let a = r.register("a");
        let b = r.register("b");
        assert_eq!(r.cancel_all(), 2);
        assert!(a.is_cancelled());
        assert!(b.is_cancelled());
    }

    #[test]
    fn registry_release_drops_entry() {
        let r = ApplyRegistry::default();
        r.register("a");
        assert_eq!(r.in_flight(), 1);
        r.release("a");
        assert_eq!(r.in_flight(), 0);
    }
}
