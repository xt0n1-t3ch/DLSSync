use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Read;
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
    Zip(#[from] zip::result::ZipError),
    #[error("missing: {0}")]
    Missing(String),
    #[error("integrity: expected sha256 {expected}, got {actual}")]
    Integrity { expected: String, actual: String },
    #[error(
        "catalog manifest has malformed sha256 ({reason}) for {filename} — refresh the manifest"
    )]
    BadCatalogSha { filename: String, reason: String },
    #[error("after {attempts} retries: {last}")]
    Retries { attempts: u32, last: String },
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

const RETRY_BACKOFF_MS: &[u64] = &[200, 800, 2000];

pub fn manifest_url() -> String {
    std::env::var("DLSSYNC_MANIFEST_URL").unwrap_or_else(|_| DEFAULT_MANIFEST_URL.to_string())
}

impl Catalog {
    pub async fn fetch(client: &reqwest::Client) -> Result<Self, CatalogError> {
        let url = manifest_url();
        Self::fetch_from(client, &url).await
    }

    pub async fn fetch_from(client: &reqwest::Client, url: &str) -> Result<Self, CatalogError> {
        let mut last_err = String::new();
        for (idx, backoff) in RETRY_BACKOFF_MS.iter().enumerate() {
            match try_fetch(client, url).await {
                Ok(c) => return Ok(c),
                Err(e) => {
                    last_err = e.to_string();
                    tracing::warn!(attempt = idx + 1, error = %last_err, "catalog fetch attempt failed");
                    if idx + 1 < RETRY_BACKOFF_MS.len() {
                        tokio::time::sleep(Duration::from_millis(*backoff)).await;
                    }
                }
            }
        }
        Err(CatalogError::Retries {
            attempts: RETRY_BACKOFF_MS.len() as u32,
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

const ZIP_MAGIC_LFH: &[u8; 4] = b"PK\x03\x04";
const ZIP_MAGIC_EOCD: &[u8; 4] = b"PK\x05\x06";
const ZIP_MAGIC_SPAN: &[u8; 4] = b"PK\x07\x08";

fn looks_like_zip(bytes: &[u8]) -> bool {
    bytes.len() >= 4
        && (bytes.starts_with(ZIP_MAGIC_LFH)
            || bytes.starts_with(ZIP_MAGIC_EOCD)
            || bytes.starts_with(ZIP_MAGIC_SPAN))
}

pub async fn download_and_extract_dll(
    client: &reqwest::Client,
    release: &Release,
    dest_dir: &Path,
) -> Result<PathBuf, CatalogError> {
    let bytes = client
        .get(&release.cdn_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    extract_dll_from_bytes(&bytes, release, dest_dir)
}

pub fn extract_dll_from_bytes(
    bytes: &[u8],
    release: &Release,
    dest_dir: &Path,
) -> Result<PathBuf, CatalogError> {
    let algo =
        HashAlgo::from_hex_len(&release.sha256).ok_or_else(|| CatalogError::BadCatalogSha {
            filename: release.filename.clone(),
            reason: format!(
                "got {} chars, expected 64 hex (SHA-256) or 32 hex (MD5)",
                release.sha256.len()
            ),
        })?;
    std::fs::create_dir_all(dest_dir)?;
    let out_path = dest_dir.join(&release.filename);

    let is_zip = looks_like_zip(bytes) || release.cdn_url.to_ascii_lowercase().ends_with(".zip");

    if is_zip {
        let cursor = std::io::Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(cursor)?;
        let mut found = false;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            if !entry.is_file() {
                continue;
            }
            let name = entry
                .enclosed_name()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| CatalogError::Missing(format!("unsafe entry: {}", entry.name())))?;
            let file_name = name
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if file_name.eq_ignore_ascii_case(&release.filename) {
                let mut out = std::fs::File::create(&out_path)?;
                std::io::copy(&mut entry, &mut out)?;
                found = true;
                break;
            }
        }
        if !found {
            return Err(CatalogError::Missing(format!(
                "{} not in zip {}",
                release.filename, release.cdn_url
            )));
        }
    } else {
        std::fs::write(&out_path, bytes)?;
    }

    let actual = hash_file_with(&out_path, algo)?;
    if !actual.eq_ignore_ascii_case(&release.sha256) {
        let _ = std::fs::remove_file(&out_path);
        return Err(CatalogError::Integrity {
            expected: release.sha256.clone(),
            actual,
        });
    }

    Ok(out_path)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgo {
    Sha256,
    Md5,
}

impl HashAlgo {
    pub fn from_hex_len(s: &str) -> Option<Self> {
        if !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        match s.len() {
            64 => Some(HashAlgo::Sha256),
            32 => Some(HashAlgo::Md5),
            _ => None,
        }
    }
}

pub fn hash_file_with(path: &Path, algo: HashAlgo) -> Result<String, CatalogError> {
    match algo {
        HashAlgo::Sha256 => hex_sha256_file(path),
        HashAlgo::Md5 => hex_md5_file(path),
    }
}

pub fn hex_md5_file(path: &Path) -> Result<String, CatalogError> {
    use md5::{Digest, Md5};
    let mut f = std::fs::File::open(path)?;
    let mut h = Md5::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{:02x}", b);
    }
    Ok(s)
}

pub fn hex_md5(bytes: &[u8]) -> String {
    use md5::{Digest, Md5};
    let mut h = Md5::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

pub fn hex_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

pub fn hex_sha256_file(path: &Path) -> Result<String, CatalogError> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{:02x}", b);
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;

    fn make_release(filename: &str, cdn_url: &str, dll_sha: &str, size: u64) -> Release {
        Release {
            version: "1.0.0".into(),
            version_packed: 0,
            filename: filename.into(),
            sha256: dll_sha.into(),
            size_bytes: size,
            signed: false,
            released_at: chrono::Utc::now(),
            source: "test".into(),
            cdn_url: cdn_url.into(),
            release_notes: None,
            signature_subject: None,
            channel: "stable".into(),
            is_dev: false,
            min_driver: None,
        }
    }

    const TEST_ZIP_URL: &str = "https://example.test/sdk.zip";
    const TEST_DLL_URL: &str = "https://example.test/nvngx_dlss.dll";
    const TEST_GENERIC_DLL_URL: &str = "https://example.test/file.dll";
    const TEST_X_ZIP_URL: &str = "https://example.test/x.zip";

    fn build_zip_with(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            for (name, content) in files {
                w.start_file(*name, opts).unwrap();
                w.write_all(content).unwrap();
            }
            w.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn extracts_dll_from_zip_when_zip_hash_differs_from_dll_hash() {
        let dll_bytes = b"this-is-the-dll-payload";
        let dll_sha = hex_sha256(dll_bytes);
        let zip_bytes = build_zip_with(&[
            ("bin/decoy.dll", b"unrelated"),
            ("bin/x64/target.dll", dll_bytes),
            ("docs/readme.txt", b"hello"),
        ]);
        let zip_sha = hex_sha256(&zip_bytes);
        assert_ne!(
            dll_sha, zip_sha,
            "zip and dll hashes must differ to exercise the regression"
        );

        let dir = tempfile::tempdir().unwrap();
        let release = make_release("target.dll", TEST_ZIP_URL, &dll_sha, dll_bytes.len() as u64);

        let out = extract_dll_from_bytes(&zip_bytes, &release, dir.path())
            .expect("extraction should succeed");
        assert_eq!(out.file_name().unwrap().to_str().unwrap(), "target.dll");
        let on_disk = std::fs::read(&out).unwrap();
        assert_eq!(on_disk, dll_bytes);
        let on_disk_sha = hex_sha256(&on_disk);
        assert_eq!(on_disk_sha, dll_sha);
    }

    #[test]
    fn rejects_when_extracted_dll_hash_mismatches() {
        let real_dll = b"the-real-bytes";
        let wrong_sha = hex_sha256(b"different-bytes");
        let zip_bytes = build_zip_with(&[("dir/target.dll", real_dll)]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "target.dll",
            TEST_ZIP_URL,
            &wrong_sha,
            real_dll.len() as u64,
        );

        let err = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap_err();
        match err {
            CatalogError::Integrity { expected, actual } => {
                assert_eq!(expected, wrong_sha);
                assert_eq!(actual, hex_sha256(real_dll));
            }
            other => panic!("expected Integrity, got {other:?}"),
        }
        assert!(
            !dir.path().join("target.dll").exists(),
            "failed extract must remove the stale file"
        );
    }

    #[test]
    fn rejects_when_filename_not_in_zip() {
        let zip_bytes = build_zip_with(&[("other.dll", b"x")]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release("target.dll", TEST_ZIP_URL, &hex_sha256(b"x"), 1);
        let err = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap_err();
        match err {
            CatalogError::Missing(msg) => assert!(msg.contains("target.dll")),
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn handles_direct_dll_download() {
        let dll_bytes = b"direct-dll-content";
        let dll_sha = hex_sha256(dll_bytes);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "nvngx_dlss.dll",
            TEST_DLL_URL,
            &dll_sha,
            dll_bytes.len() as u64,
        );

        let out = extract_dll_from_bytes(dll_bytes, &release, dir.path())
            .expect("direct write should succeed");
        assert_eq!(std::fs::read(&out).unwrap(), dll_bytes);
    }

    #[test]
    fn case_insensitive_filename_match() {
        let dll_bytes = b"casing-test";
        let dll_sha = hex_sha256(dll_bytes);
        let zip_bytes = build_zip_with(&[("bin/Target.DLL", dll_bytes)]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "target.dll",
            TEST_X_ZIP_URL,
            &dll_sha,
            dll_bytes.len() as u64,
        );
        let out = extract_dll_from_bytes(&zip_bytes, &release, dir.path())
            .expect("case-insensitive match");
        assert_eq!(std::fs::read(&out).unwrap(), dll_bytes);
    }

    #[test]
    fn looks_like_zip_detection() {
        assert!(looks_like_zip(b"PK\x03\x04rest"));
        assert!(looks_like_zip(b"PK\x05\x06"));
        assert!(looks_like_zip(b"PK\x07\x08more"));
        assert!(!looks_like_zip(b"MZnotzip"));
        assert!(!looks_like_zip(b""));
        assert!(!looks_like_zip(b"PK\x01"));
    }

    #[test]
    fn rejects_odd_length_catalog_hash() {
        let dir = tempfile::tempdir().unwrap();
        let release = make_release("target.dll", TEST_GENERIC_DLL_URL, "abcdef1234567890", 1);
        let err = extract_dll_from_bytes(b"any-bytes", &release, dir.path()).unwrap_err();
        match err {
            CatalogError::BadCatalogSha { filename, reason } => {
                assert_eq!(filename, "target.dll");
                assert!(reason.contains("16 chars"));
            }
            other => panic!("expected BadCatalogSha, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_hex_catalog_sha() {
        let dir = tempfile::tempdir().unwrap();
        let bad = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        let release = make_release("target.dll", TEST_GENERIC_DLL_URL, bad, 1);
        let err = extract_dll_from_bytes(b"x", &release, dir.path()).unwrap_err();
        assert!(matches!(err, CatalogError::BadCatalogSha { .. }));
    }

    #[test]
    fn accepts_md5_keyed_release_when_extracted_dll_matches() {
        let dll_bytes = b"community-archive-dll-payload";
        let dll_md5 = hex_md5(dll_bytes);
        assert_eq!(dll_md5.len(), 32);
        let zip_bytes = build_zip_with(&[("bin/x64/nvngx_dlss.dll", dll_bytes)]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "nvngx_dlss.dll",
            TEST_ZIP_URL,
            &dll_md5,
            dll_bytes.len() as u64,
        );
        let out = extract_dll_from_bytes(&zip_bytes, &release, dir.path())
            .expect("md5-keyed release should verify with md5");
        assert_eq!(std::fs::read(&out).unwrap(), dll_bytes);
    }

    #[test]
    fn rejects_md5_mismatch_on_extracted_dll() {
        let dll_bytes = b"real-bytes";
        let wrong_md5 = hex_md5(b"different");
        let zip_bytes = build_zip_with(&[("nvngx_dlss.dll", dll_bytes)]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "nvngx_dlss.dll",
            TEST_ZIP_URL,
            &wrong_md5,
            dll_bytes.len() as u64,
        );
        let err = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap_err();
        match err {
            CatalogError::Integrity { expected, actual } => {
                assert_eq!(expected, wrong_md5);
                assert_eq!(actual, hex_md5(dll_bytes));
            }
            other => panic!("expected Integrity, got {other:?}"),
        }
        assert!(!dir.path().join("nvngx_dlss.dll").exists());
    }

    #[test]
    fn hash_algo_from_hex_len() {
        assert_eq!(
            HashAlgo::from_hex_len(&"a".repeat(64)),
            Some(HashAlgo::Sha256)
        );
        assert_eq!(HashAlgo::from_hex_len(&"a".repeat(32)), Some(HashAlgo::Md5));
        assert_eq!(HashAlgo::from_hex_len(&"a".repeat(40)), None);
        assert_eq!(HashAlgo::from_hex_len(""), None);
        assert_eq!(HashAlgo::from_hex_len(&"z".repeat(64)), None);
    }

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
            cdn_url: TEST_GENERIC_DLL_URL.into(),
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
