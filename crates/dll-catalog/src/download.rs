use crate::CatalogError;
use bytes::Bytes;
use futures_util::StreamExt;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, OnceCell};
use tokio_util::sync::CancellationToken;

pub const DEFAULT_MAX_RETRIES: u32 = 3;
pub const DEFAULT_CHUNK_TIMEOUT: Duration = Duration::from_secs(60);
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(300);
pub const DEFAULT_PROGRESS_INTERVAL: Duration = Duration::from_millis(120);

const RETRY_BACKOFF_MS: &[u64] = &[500, 2_000, 5_000];
const RETRY_JITTER_MAX_MS: u64 = 500;

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub url: String,
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub bytes_per_sec: f64,
    pub attempt: u32,
}

#[derive(Clone)]
pub struct DownloadOptions {
    pub max_retries: u32,
    pub chunk_timeout: Duration,
    /// Hard ceiling on bytes buffered per attempt, independent of the server's
    /// `Content-Length` claim — bounds a stream that never ends.
    pub max_total_bytes: u64,
    pub progress_tx: Option<mpsc::UnboundedSender<DownloadProgress>>,
    pub cancel: Option<CancellationToken>,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            chunk_timeout: DEFAULT_CHUNK_TIMEOUT,
            max_total_bytes: crate::zip::MAX_ZIP_TOTAL_BYTES,
            progress_tx: None,
            cancel: None,
        }
    }
}

type CacheEntry = Arc<OnceCell<Result<Arc<Bytes>, Arc<CatalogError>>>>;

#[derive(Clone, Default)]
pub struct DownloadCache {
    inner: Arc<Mutex<HashMap<String, (CacheEntry, Instant)>>>,
    ttl: Duration,
}

impl DownloadCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl: DEFAULT_CACHE_TTL,
        }
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    pub fn get_or_init(&self, url: &str) -> CacheEntry {
        let mut guard = self.inner.lock();
        if let Some((entry, last_used)) = guard.get_mut(url) {
            *last_used = Instant::now();
            return entry.clone();
        }
        let entry: CacheEntry = Arc::new(OnceCell::new());
        guard.insert(url.to_string(), (entry.clone(), Instant::now()));
        entry
    }

    pub fn evict_idle(&self) -> usize {
        let mut guard = self.inner.lock();
        let cutoff = Instant::now()
            .checked_sub(self.ttl)
            .unwrap_or_else(Instant::now);
        let before = guard.len();
        guard.retain(|_, (_, last_used)| *last_used >= cutoff);
        before - guard.len()
    }

    pub fn clear(&self) {
        self.inner.lock().clear();
    }

    fn forget(&self, url: &str, entry: &CacheEntry) {
        let mut guard = self.inner.lock();
        if let Some((existing, _)) = guard.get(url) {
            if Arc::ptr_eq(existing, entry) {
                guard.remove(url);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }
}

pub async fn fetch_shared(
    cache: &DownloadCache,
    client: &reqwest::Client,
    url: &str,
    opts: DownloadOptions,
) -> Result<Arc<Bytes>, CatalogError> {
    let entry = cache.get_or_init(url);
    let url_owned = url.to_string();
    let client = client.clone();
    let init_opts = opts.clone();
    let result = entry
        .get_or_init(|| async move {
            match download_with_retry(&client, &url_owned, init_opts).await {
                Ok(bytes) => Ok(Arc::new(bytes)),
                Err(e) => Err(Arc::new(e)),
            }
        })
        .await;
    match result {
        Ok(bytes) => Ok(bytes.clone()),
        Err(err) => {
            cache.forget(url, &entry);
            Err(CatalogError::Cached(err.to_string()))
        }
    }
}

async fn download_with_retry(
    client: &reqwest::Client,
    url: &str,
    opts: DownloadOptions,
) -> Result<Bytes, CatalogError> {
    let max_attempts = opts.max_retries.max(1);
    let mut last_err: Option<CatalogError> = None;
    for attempt in 1..=max_attempts {
        if let Some(cancel) = &opts.cancel {
            if cancel.is_cancelled() {
                return Err(CatalogError::Cancelled);
            }
        }
        match download_once(client, url, attempt, &opts).await {
            Ok(bytes) => return Ok(bytes),
            Err(err) => {
                let should_retry = is_retriable(&err) && attempt < max_attempts;
                tracing::warn!(
                    attempt,
                    url,
                    retry = should_retry,
                    error = %err,
                    "download attempt failed"
                );
                if !should_retry {
                    return Err(err);
                }
                let backoff_ms = RETRY_BACKOFF_MS
                    .get((attempt - 1) as usize)
                    .copied()
                    .unwrap_or(5_000);
                let jitter = pseudo_jitter_ms(attempt as u64, url);
                tokio::time::sleep(Duration::from_millis(backoff_ms + jitter)).await;
                last_err = Some(err);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        CatalogError::Missing("download exhausted retries with no recorded error".into())
    }))
}

async fn download_once(
    client: &reqwest::Client,
    url: &str,
    attempt: u32,
    opts: &DownloadOptions,
) -> Result<Bytes, CatalogError> {
    let response = client.get(url).send().await?.error_for_status()?;
    let content_length = response.content_length();
    let total = content_length;
    if total.is_some_and(|len| len > opts.max_total_bytes) {
        return Err(CatalogError::Unsafe(format!(
            "download larger than the {} byte ceiling",
            opts.max_total_bytes
        )));
    }
    // Never trust Content-Length as an allocation hint: a hostile/misconfigured
    // server reporting e.g. 4 GiB would otherwise force an immediate giant
    // reservation (OOM/abort). Cap the pre-allocation at the same ceiling the zip
    // extractor enforces; the streamed read below still grows as needed.
    let pre_alloc = total
        .unwrap_or(8 * 1024 * 1024)
        .min(crate::zip::MAX_ZIP_TOTAL_BYTES) as usize;
    let mut buf: Vec<u8> = Vec::with_capacity(pre_alloc);
    let mut stream = response.bytes_stream();
    let started = Instant::now();
    let mut last_emit = Instant::now();
    loop {
        if let Some(cancel) = &opts.cancel {
            if cancel.is_cancelled() {
                return Err(CatalogError::Cancelled);
            }
        }
        let next = tokio::time::timeout(opts.chunk_timeout, stream.next()).await;
        let chunk = match next {
            Ok(Some(c)) => c?,
            Ok(None) => break,
            Err(_) => {
                return Err(CatalogError::Stalled {
                    seconds: opts.chunk_timeout.as_secs(),
                });
            }
        };
        buf.extend_from_slice(&chunk);
        if buf.len() as u64 > opts.max_total_bytes {
            return Err(CatalogError::Unsafe(format!(
                "download stream exceeded the {} byte ceiling",
                opts.max_total_bytes
            )));
        }
        if let Some(tx) = &opts.progress_tx {
            if last_emit.elapsed() >= DEFAULT_PROGRESS_INTERVAL {
                let elapsed = started.elapsed().as_secs_f64().max(0.001);
                let bps = (buf.len() as f64) / elapsed;
                let _ = tx.send(DownloadProgress {
                    url: url.to_string(),
                    bytes_downloaded: buf.len() as u64,
                    bytes_total: total,
                    bytes_per_sec: bps,
                    attempt,
                });
                last_emit = Instant::now();
            }
        }
    }
    if let Some(tx) = &opts.progress_tx {
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        let _ = tx.send(DownloadProgress {
            url: url.to_string(),
            bytes_downloaded: buf.len() as u64,
            bytes_total: total,
            bytes_per_sec: (buf.len() as f64) / elapsed,
            attempt,
        });
    }
    if let Some(expected) = content_length {
        if (buf.len() as u64) < expected {
            return Err(CatalogError::Truncated {
                got: buf.len() as u64,
                expected,
            });
        }
    }
    Ok(Bytes::from(buf))
}

fn is_retriable(err: &CatalogError) -> bool {
    match err {
        CatalogError::Http(e) => {
            e.is_connect()
                || e.is_timeout()
                || e.is_body()
                || e.is_request()
                || matches!(
                    e.status().map(|s| s.as_u16()),
                    Some(408 | 429 | 500 | 502 | 503 | 504)
                )
        }
        CatalogError::Stalled { .. } | CatalogError::Truncated { .. } => true,
        _ => false,
    }
}

fn pseudo_jitter_ms(seed: u64, url: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in url.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    (h.wrapping_add(seed.wrapping_mul(0x9E3779B97F4A7C15))) % RETRY_JITTER_MAX_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn cache_returns_same_entry_for_repeated_url() {
        let cache = DownloadCache::new();
        let a = cache.get_or_init("https://example.test/a");
        let b = cache.get_or_init("https://example.test/a");
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cache_distinct_urls_yield_distinct_entries() {
        let cache = DownloadCache::new();
        let a = cache.get_or_init("https://example.test/a");
        let b = cache.get_or_init("https://example.test/b");
        assert!(!Arc::ptr_eq(&a, &b));
        assert_eq!(cache.len(), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cache_clear_removes_all_entries() {
        let cache = DownloadCache::new();
        cache.get_or_init("https://example.test/a");
        cache.get_or_init("https://example.test/b");
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cache_evict_idle_clears_expired() {
        let cache = DownloadCache::with_ttl(Duration::from_millis(1));
        cache.get_or_init("https://example.test/a");
        tokio::time::sleep(Duration::from_millis(20)).await;
        let evicted = cache.evict_idle();
        assert_eq!(evicted, 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn jitter_is_bounded() {
        for seed in 0..50u64 {
            let v = pseudo_jitter_ms(seed, "https://example.test/url");
            assert!(v < RETRY_JITTER_MAX_MS);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_shared_does_not_cache_errors() {
        let cache = DownloadCache::new();
        let client = reqwest::Client::new();
        let url = "http://127.0.0.1:1/never";
        let opts = DownloadOptions {
            max_retries: 1,
            chunk_timeout: Duration::from_millis(200),
            ..Default::default()
        };
        let first = fetch_shared(&cache, &client, url, opts.clone()).await;
        assert!(first.is_err());
        assert!(
            cache.is_empty(),
            "a failed download must not stay cached for the session"
        );
        let second = fetch_shared(&cache, &client, url, opts).await;
        assert!(second.is_err());
        assert!(cache.is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn forget_only_removes_matching_entry() {
        let cache = DownloadCache::new();
        let url = "https://example.test/a";
        let original = cache.get_or_init(url);
        cache.forget(url, &original);
        assert!(cache.is_empty());

        let original = cache.get_or_init(url);
        let replacement = {
            cache.inner.lock().remove(url);
            cache.get_or_init(url)
        };
        assert!(!Arc::ptr_eq(&original, &replacement));
        cache.forget(url, &original);
        assert_eq!(
            cache.len(),
            1,
            "forget must not evict a newer entry under the same url"
        );
    }
}
