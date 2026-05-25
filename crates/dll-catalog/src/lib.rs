pub mod download;
pub mod hash;
pub mod zip;

pub use download::{
    fetch_shared, DownloadCache, DownloadOptions, DownloadProgress, DEFAULT_CACHE_TTL,
    DEFAULT_CHUNK_TIMEOUT, DEFAULT_MAX_RETRIES,
};
pub use hash::{hash_file_with, hex_md5, hex_md5_file, hex_sha256, hex_sha256_file, HashAlgo};
pub use zip::{
    extract_dll_from_bytes, looks_like_zip, MAX_UNCOMPRESSED_ENTRY_BYTES, MAX_ZIP_TOTAL_BYTES,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip: {0}")]
    Zip(#[from] ::zip::result::ZipError),
    #[error("missing: {0}")]
    Missing(String),
    #[error("unsafe archive: {0}")]
    Unsafe(String),
    #[error("integrity: expected sha256 {expected}, got {actual}")]
    Integrity { expected: String, actual: String },
    #[error("truncated: received {got} bytes of {expected}")]
    Truncated { got: u64, expected: u64 },
    #[error("stalled: no bytes for {seconds} s")]
    Stalled { seconds: u64 },
    #[error("cancelled by user")]
    Cancelled,
    #[error(
        "catalog manifest has malformed sha256 ({reason}) for {filename} — refresh the manifest"
    )]
    BadCatalogSha { filename: String, reason: String },
    #[error("after {attempts} retries: {last}")]
    Retries { attempts: u32, last: String },
    #[error("cached error: {0}")]
    Cached(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub schema_version: u32,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub vendors: BTreeMap<String, BTreeMap<String, FamilyEntry>>,
    #[serde(default)]
    pub incompatible_games: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyEntry {
    pub latest: String,
    pub releases: Vec<Release>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub version: String,
    pub version_packed: u64,
    pub filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub signed: bool,
    pub released_at: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub cdn_url: String,
    #[serde(default)]
    pub release_notes: Option<String>,
    #[serde(default)]
    pub signature_subject: Option<String>,
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub is_dev: bool,
    #[serde(default)]
    pub min_driver: Option<String>,
}

fn default_channel() -> String {
    "stable".to_string()
}

pub const DEFAULT_MANIFEST_URL: &str =
    "https://cdn.jsdelivr.net/gh/xt0n1-t3ch/dlssync-manifest@main/manifest.json";

pub const MANIFEST_ENV_VAR: &str = "DLSSYNC_MANIFEST_URL";

const MANIFEST_RETRY_BACKOFF_MS: &[u64] = &[200, 800, 2000];

pub fn manifest_url() -> String {
    std::env::var(MANIFEST_ENV_VAR).unwrap_or_else(|_| DEFAULT_MANIFEST_URL.to_string())
}

impl Catalog {
    pub async fn fetch(client: &reqwest::Client) -> Result<Self, CatalogError> {
        let url = manifest_url();
        Self::fetch_from(client, &url).await
    }

    pub async fn fetch_from(client: &reqwest::Client, url: &str) -> Result<Self, CatalogError> {
        let mut last_err = String::new();
        for (idx, backoff) in MANIFEST_RETRY_BACKOFF_MS.iter().enumerate() {
            match try_fetch(client, url).await {
                Ok(c) => return Ok(c),
                Err(e) => {
                    last_err = e.to_string();
                    tracing::warn!(attempt = idx + 1, error = %last_err, "catalog fetch attempt failed");
                    if idx + 1 < MANIFEST_RETRY_BACKOFF_MS.len() {
                        tokio::time::sleep(Duration::from_millis(*backoff)).await;
                    }
                }
            }
        }
        Err(CatalogError::Retries {
            attempts: MANIFEST_RETRY_BACKOFF_MS.len() as u32,
            last: last_err,
        })
    }

    pub async fn fetch_with_cache(
        client: &reqwest::Client,
        cache_path: &Path,
    ) -> Result<Self, CatalogError> {
        match Self::fetch(client).await {
            Ok(c) => {
                if let Some(parent) = cache_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let body = serde_json::to_vec_pretty(&c)?;
                std::fs::write(cache_path, body)?;
                Ok(c)
            }
            Err(e) => {
                if cache_path.exists() {
                    tracing::warn!(error = %e, "falling back to cached catalog");
                    let body = std::fs::read(cache_path)?;
                    let c: Catalog = serde_json::from_slice(&body)?;
                    Ok(c)
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn releases(&self, vendor: &str, family: &str) -> Vec<Release> {
        self.vendors
            .get(vendor)
            .and_then(|v| v.get(family))
            .map(|f| f.releases.clone())
            .unwrap_or_default()
    }

    pub fn latest(&self, vendor: &str, family: &str) -> Option<Release> {
        let v = self.vendors.get(vendor)?;
        let f = v.get(family)?;
        f.releases.iter().find(|r| r.version == f.latest).cloned()
    }

    pub fn find(&self, vendor: &str, family: &str, version: &str) -> Option<Release> {
        let v = self.vendors.get(vendor)?;
        let f = v.get(family)?;
        f.releases.iter().find(|r| r.version == version).cloned()
    }

    pub fn find_latest_for_file(
        &self,
        vendor: &str,
        family: &str,
        filename: &str,
    ) -> Option<Release> {
        let v = self.vendors.get(vendor)?;
        let f = v.get(family)?;
        f.releases
            .iter()
            .filter(|r| r.filename.eq_ignore_ascii_case(filename))
            .max_by_key(|r| r.version_packed)
            .cloned()
    }
}

async fn try_fetch(client: &reqwest::Client, url: &str) -> Result<Catalog, CatalogError> {
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let c: Catalog = serde_json::from_slice(&bytes)?;
    Ok(c)
}

pub async fn download_and_extract_dll(
    client: &reqwest::Client,
    release: &Release,
    dest_dir: &Path,
) -> Result<PathBuf, CatalogError> {
    let cache = DownloadCache::new();
    download_and_extract_dll_cached(
        &cache,
        client,
        release,
        dest_dir,
        DownloadOptions::default(),
    )
    .await
}

pub async fn download_and_extract_dll_cached(
    cache: &DownloadCache,
    client: &reqwest::Client,
    release: &Release,
    dest_dir: &Path,
    opts: DownloadOptions,
) -> Result<PathBuf, CatalogError> {
    let bytes = fetch_shared(cache, client, &release.cdn_url, opts).await?;
    extract_dll_from_bytes(&bytes, release, dest_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_with(filename: &str, version: &str, packed: u64, sha: &str) -> Release {
        Release {
            version: version.into(),
            version_packed: packed,
            filename: filename.into(),
            sha256: sha.into(),
            size_bytes: 100,
            signed: false,
            released_at: chrono::Utc::now(),
            source: "test".into(),
            cdn_url: "https://example.test/x.dll".into(),
            release_notes: None,
            signature_subject: None,
            channel: "stable".into(),
            is_dev: false,
            min_driver: None,
        }
    }

    fn catalog_with(family: &str, releases: Vec<Release>) -> Catalog {
        let mut families = BTreeMap::new();
        let latest = releases
            .iter()
            .max_by_key(|r| r.version_packed)
            .map(|r| r.version.clone())
            .unwrap_or_default();
        families.insert(family.to_string(), FamilyEntry { latest, releases });
        let mut vendors = BTreeMap::new();
        vendors.insert("intel".to_string(), families);
        Catalog {
            schema_version: 2,
            generated_at: chrono::Utc::now(),
            vendors,
            incompatible_games: vec![],
        }
    }

    #[test]
    fn find_latest_for_file_picks_highest_packed_version() {
        let c = catalog_with(
            "xess_sr",
            vec![
                release_with("libxess.dll", "2.0.0", 200, "shaA"),
                release_with("libxess.dll", "3.0.1", 301, "shaC"),
                release_with("libxess.dll", "2.5.0", 250, "shaB"),
            ],
        );
        let r = c
            .find_latest_for_file("intel", "xess_sr", "libxess.dll")
            .unwrap();
        assert_eq!(r.version, "3.0.1");
        assert_eq!(r.sha256, "shaC");
    }

    #[test]
    fn find_latest_for_file_is_case_insensitive() {
        let c = catalog_with(
            "xess_sr",
            vec![release_with("libxess.dll", "3.0.1", 301, "shaX")],
        );
        let r = c
            .find_latest_for_file("intel", "xess_sr", "LIBXESS.DLL")
            .unwrap();
        assert_eq!(r.sha256, "shaX");
    }

    #[test]
    fn find_latest_for_file_filters_by_filename() {
        let c = catalog_with(
            "xess_sr",
            vec![
                release_with("libxess.dll", "3.0.1", 301, "main"),
                release_with("libxess_dx11.dll", "3.0.1", 301, "dx11"),
            ],
        );
        let r = c
            .find_latest_for_file("intel", "xess_sr", "libxess_dx11.dll")
            .unwrap();
        assert_eq!(r.sha256, "dx11");
    }

    #[test]
    fn find_latest_for_file_returns_none_on_unknown_vendor() {
        let c = catalog_with(
            "xess_sr",
            vec![release_with("libxess.dll", "3.0.1", 301, "x")],
        );
        assert!(c
            .find_latest_for_file("nvidia", "xess_sr", "libxess.dll")
            .is_none());
    }

    #[test]
    fn find_latest_for_file_returns_none_on_missing_filename() {
        let c = catalog_with(
            "xess_sr",
            vec![release_with("libxess.dll", "3.0.1", 301, "x")],
        );
        assert!(c
            .find_latest_for_file("intel", "xess_sr", "nvngx_dlss.dll")
            .is_none());
    }
}
