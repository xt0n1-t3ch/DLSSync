use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use backup_store::BackupStore;
use dll_catalog::{Catalog, DownloadCache};
use dlssync_application::{product_config, DistributionPolicy};
use dlssync_contracts::{CatalogProvenance, DistributionChannel, InstallMode};
use notifications_store::NotificationsStore;
use operation_journal::JournalStore;

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
    pub catalog_provenance: Arc<RwLock<Option<CatalogProvenance>>>,
    pub catalog_cache_path: Arc<RwLock<Option<PathBuf>>>,
    pub backups: Arc<RwLock<Option<BackupStore>>>,
    pub notifications: Arc<RwLock<Option<NotificationsStore>>>,
    pub journal: Arc<RwLock<Option<JournalStore>>>,
    pub distribution_policy: Arc<RwLock<DistributionPolicy>>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub paths: Arc<RwLock<Option<AppPaths>>>,
    pub system_info: Arc<RwLock<Option<SystemInfo>>>,
    /// Coordinator for `ensure_system_info` so two concurrent callers don't both
    /// pay the WMI/DXGI collection cost. Holding the lock guarantees only one
    /// `collect` runs at a time; the cached value behind `system_info` is the
    /// fast path and stays in `parking_lot::RwLock` for non-async reads.
    pub collect_system_info_lock: Arc<tokio::sync::Mutex<()>>,
    pub http_catalog: reqwest::Client,
    pub http_downloads: reqwest::Client,
    pub http_art: reqwest::Client,
    pub download_cache: Arc<DownloadCache>,
    pub apply_registry: Arc<ApplyRegistry>,
}

impl AppState {
    pub fn new() -> Self {
        let config = product_config().expect("embedded product.toml must be valid");
        let channel = if cfg!(feature = "nexus") {
            DistributionChannel::Nexus
        } else {
            DistributionChannel::Standard
        };
        let distribution_policy =
            DistributionPolicy::resolve(&config, channel, InstallMode::Installed);
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
            catalog_provenance: Arc::new(RwLock::new(None)),
            catalog_cache_path: Arc::new(RwLock::new(None)),
            backups: Arc::new(RwLock::new(None)),
            notifications: Arc::new(RwLock::new(None)),
            journal: Arc::new(RwLock::new(None)),
            distribution_policy: Arc::new(RwLock::new(distribution_policy)),
            settings: Arc::new(RwLock::new(AppSettings::default())),
            paths: Arc::new(RwLock::new(None)),
            system_info: Arc::new(RwLock::new(None)),
            collect_system_info_lock: Arc::new(tokio::sync::Mutex::new(())),
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

/// Double-checked locking helper. Returns the cached value if present;
/// otherwise acquires `coordinator` and runs `collect` exactly once.
/// Concurrent callers wait on the coordinator and share the same result —
/// they re-check the cache after acquiring so a winning collector's value
/// is reused, not re-computed. The cache stays in `parking_lot::RwLock` so
/// non-async readers do not pay an async-await on the hot path; only the
/// expensive `collect` is serialized.
pub async fn coordinate_singleton<T, F, Fut, E>(
    cache: &RwLock<Option<T>>,
    coordinator: &tokio::sync::Mutex<()>,
    collect: F,
) -> Result<T, E>
where
    T: Clone,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    if let Some(v) = cache.read().as_ref().cloned() {
        return Ok(v);
    }
    let _guard = coordinator.lock().await;
    if let Some(v) = cache.read().as_ref().cloned() {
        return Ok(v);
    }
    let v = collect().await?;
    *cache.write() = Some(v.clone());
    Ok(v)
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

    #[tokio::test]
    async fn coordinate_singleton_runs_collect_exactly_once_under_concurrency() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache: Arc<RwLock<Option<u32>>> = Arc::new(RwLock::new(None));
        let coordinator: Arc<tokio::sync::Mutex<()>> = Arc::new(tokio::sync::Mutex::new(()));
        let counter = Arc::new(AtomicUsize::new(0));

        let mut tasks = Vec::new();
        for _ in 0..8 {
            let cache = cache.clone();
            let coordinator = coordinator.clone();
            let counter = counter.clone();
            tasks.push(tokio::spawn(async move {
                coordinate_singleton(&cache, &coordinator, || async {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<u32, ()>(42)
                })
                .await
                .unwrap()
            }));
        }

        for t in tasks {
            assert_eq!(t.await.unwrap(), 42);
        }

        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "collect ran more than once under 8-way concurrency"
        );
    }

    #[tokio::test]
    async fn coordinate_singleton_returns_cached_value_on_fast_path() {
        let cache: Arc<RwLock<Option<u32>>> = Arc::new(RwLock::new(Some(7)));
        let coordinator: Arc<tokio::sync::Mutex<()>> = Arc::new(tokio::sync::Mutex::new(()));
        let result = coordinate_singleton(&cache, &coordinator, || async {
            panic!("collect must not run when the cache is already populated");
            #[allow(unreachable_code)]
            Ok::<u32, ()>(0)
        })
        .await
        .unwrap();
        assert_eq!(result, 7);
    }
}
