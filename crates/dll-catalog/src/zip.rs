use crate::hash::{hash_file_with, HashAlgo};
use crate::{CatalogError, Release};
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub const MAX_UNCOMPRESSED_ENTRY_BYTES: u64 = 200 * 1024 * 1024;
pub const MAX_ZIP_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;

const ZIP_MAGIC_LFH: &[u8; 4] = b"PK\x03\x04";
const ZIP_MAGIC_EOCD: &[u8; 4] = b"PK\x05\x06";
const ZIP_MAGIC_SPAN: &[u8; 4] = b"PK\x07\x08";

pub fn looks_like_zip(bytes: &[u8]) -> bool {
    bytes.len() >= 4
        && (bytes.starts_with(ZIP_MAGIC_LFH)
            || bytes.starts_with(ZIP_MAGIC_EOCD)
            || bytes.starts_with(ZIP_MAGIC_SPAN))
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
        let cursor = Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(cursor)?;
        let mut total_uncompressed: u64 = 0;
        let mut candidates: Vec<(usize, String)> = Vec::new();
        for i in 0..zip.len() {
            let entry = zip.by_index(i)?;
            if !entry.is_file() {
                continue;
            }
            let size = entry.size();
            if size > MAX_UNCOMPRESSED_ENTRY_BYTES {
                return Err(CatalogError::Unsafe(format!(
                    "zip entry '{}' exceeds size cap ({} > {} bytes)",
                    entry.name(),
                    size,
                    MAX_UNCOMPRESSED_ENTRY_BYTES
                )));
            }
            total_uncompressed = total_uncompressed.saturating_add(size);
            if total_uncompressed > MAX_ZIP_TOTAL_BYTES {
                return Err(CatalogError::Unsafe(format!(
                    "zip total uncompressed exceeds cap ({} > {} bytes)",
                    total_uncompressed, MAX_ZIP_TOTAL_BYTES
                )));
            }
            let name = entry
                .enclosed_name()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| {
                    CatalogError::Unsafe(format!("unsafe zip entry path: {}", entry.name()))
                })?;
            reject_unsafe_components(&name, entry.name())?;
            candidates.push((i, normalize_zip_path(&name)));
        }
        let target = select_zip_entry(&candidates, release).ok_or_else(|| {
            CatalogError::Missing(format!(
                "{} not in zip {}",
                release.zip_entry.as_deref().unwrap_or(&release.filename),
                release.cdn_url
            ))
        })?;
        let mut entry = zip.by_index(target)?;
        let mut out = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;
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

fn normalize_zip_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Pick the zip entry index to extract. With `release.zip_entry` set, match that
/// exact path (case-insensitive). Otherwise match the basename, preferring a path
/// outside `/development/` so a signed `bin/x64/` binary always wins over the
/// unsigned development copy that shares the same filename.
fn select_zip_entry(candidates: &[(usize, String)], release: &Release) -> Option<usize> {
    if let Some(want) = &release.zip_entry {
        let want = want.replace('\\', "/");
        return candidates
            .iter()
            .find(|(_, path)| path.eq_ignore_ascii_case(&want))
            .map(|(i, _)| *i);
    }
    let basename_matches: Vec<&(usize, String)> = candidates
        .iter()
        .filter(|(_, path)| {
            path.rsplit('/')
                .next()
                .unwrap_or(path)
                .eq_ignore_ascii_case(&release.filename)
        })
        .collect();
    basename_matches
        .iter()
        .find(|(_, path)| !path.to_ascii_lowercase().contains("/development/"))
        .or_else(|| basename_matches.first())
        .map(|(i, _)| *i)
}

fn reject_unsafe_components(path: &Path, raw_name: &str) -> Result<(), CatalogError> {
    let raw_lower = raw_name.to_ascii_lowercase();
    if raw_lower.contains(':') {
        return Err(CatalogError::Unsafe(format!(
            "zip entry path contains ':' (NTFS ADS): {}",
            raw_name
        )));
    }
    for ch in raw_name.chars() {
        if matches!(ch, '\u{0000}'..='\u{001F}') {
            return Err(CatalogError::Unsafe(format!(
                "zip entry path contains control character: {}",
                raw_name.escape_debug()
            )));
        }
        let code = ch as u32;
        if (0xD800..=0xDFFF).contains(&code) {
            return Err(CatalogError::Unsafe(format!(
                "zip entry path contains surrogate character: {}",
                raw_name.escape_debug()
            )));
        }
    }
    for component in path.components() {
        if matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::RootDir
        ) {
            return Err(CatalogError::Unsafe(format!(
                "zip entry path has unsafe component: {}",
                raw_name
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::{hex_md5, hex_sha256};
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;

    const TEST_ZIP_URL: &str = "https://example.test/sdk.zip";
    const TEST_DLL_URL: &str = "https://example.test/nvngx_dlss.dll";
    const TEST_X_ZIP_URL: &str = "https://example.test/x.zip";

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
            hash_algorithm: "sha256".into(),
            zip_entry: None,
        }
    }

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
        let dir = tempfile::tempdir().unwrap();
        let release = make_release("target.dll", TEST_ZIP_URL, &dll_sha, dll_bytes.len() as u64);
        let out = extract_dll_from_bytes(&zip_bytes, &release, dir.path())
            .expect("extraction should succeed");
        assert_eq!(out.file_name().unwrap().to_str().unwrap(), "target.dll");
        assert_eq!(std::fs::read(&out).unwrap(), dll_bytes);
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
        assert!(matches!(err, CatalogError::Integrity { .. }));
        assert!(!dir.path().join("target.dll").exists());
    }

    #[test]
    fn rejects_when_filename_not_in_zip() {
        let zip_bytes = build_zip_with(&[("other.dll", b"x")]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release("target.dll", TEST_ZIP_URL, &hex_sha256(b"x"), 1);
        let err = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap_err();
        assert!(matches!(err, CatalogError::Missing(_)));
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
        let out = extract_dll_from_bytes(dll_bytes, &release, dir.path()).unwrap();
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
        let out = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), dll_bytes);
    }

    #[test]
    fn zip_entry_exact_path_wins_over_basename_collision() {
        let prod = b"production-payload";
        let dev = b"development-payload";
        let zip_bytes = build_zip_with(&[
            ("bin/x64/development/sl.dlss_g.dll", dev),
            ("bin/x64/sl.dlss_g.dll", prod),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let mut release = make_release(
            "sl.dlss_g.dll",
            TEST_ZIP_URL,
            &hex_sha256(prod),
            prod.len() as u64,
        );
        release.zip_entry = Some("bin/x64/sl.dlss_g.dll".into());
        let out = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), prod);
    }

    #[test]
    fn basename_fallback_prefers_production_over_development() {
        let prod = b"signed-production";
        let dev = b"unsigned-development";
        let zip_bytes = build_zip_with(&[
            ("bin/x64/development/sl.interposer.dll", dev),
            ("bin/x64/sl.interposer.dll", prod),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "sl.interposer.dll",
            TEST_ZIP_URL,
            &hex_sha256(prod),
            prod.len() as u64,
        );
        let out = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), prod);
    }

    #[test]
    fn zip_entry_missing_path_is_missing_error() {
        let zip_bytes = build_zip_with(&[("bin/x64/sl.dlss.dll", b"x")]);
        let dir = tempfile::tempdir().unwrap();
        let mut release = make_release("sl.dlss.dll", TEST_ZIP_URL, &hex_sha256(b"x"), 1);
        release.zip_entry = Some("bin/x64/development/sl.dlss.dll".into());
        let err = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap_err();
        assert!(matches!(err, CatalogError::Missing(_)));
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
    fn rejects_zip_entry_exceeding_size_cap() {
        let big = vec![0u8; (MAX_UNCOMPRESSED_ENTRY_BYTES + 1) as usize];
        let zip_bytes = build_zip_with(&[("target.dll", &big)]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "target.dll",
            TEST_ZIP_URL,
            &hex_sha256(&big),
            big.len() as u64,
        );
        let err = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap_err();
        assert!(matches!(err, CatalogError::Unsafe(_)));
    }

    #[test]
    fn rejects_zip_entry_with_path_traversal() {
        let dll = b"x";
        let zip_bytes = build_zip_with(&[("../escape.dll", dll)]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release("escape.dll", TEST_ZIP_URL, &hex_sha256(dll), 1);
        let err = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap_err();
        match err {
            CatalogError::Missing(_) | CatalogError::Unsafe(_) => {}
            other => panic!("expected Missing or Unsafe, got {other:?}"),
        }
    }

    #[test]
    fn accepts_md5_keyed_release_when_extracted_dll_matches() {
        let dll_bytes = b"community-archive-dll-payload";
        let dll_md5 = hex_md5(dll_bytes);
        let zip_bytes = build_zip_with(&[("bin/x64/nvngx_dlss.dll", dll_bytes)]);
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "nvngx_dlss.dll",
            TEST_ZIP_URL,
            &dll_md5,
            dll_bytes.len() as u64,
        );
        let out = extract_dll_from_bytes(&zip_bytes, &release, dir.path()).unwrap();
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
        assert!(matches!(err, CatalogError::Integrity { .. }));
        assert!(!dir.path().join("nvngx_dlss.dll").exists());
    }

    #[test]
    fn rejects_odd_length_catalog_hash() {
        let dir = tempfile::tempdir().unwrap();
        let release = make_release(
            "target.dll",
            "https://example.test/file.dll",
            "abcdef1234567890",
            1,
        );
        let err = extract_dll_from_bytes(b"any-bytes", &release, dir.path()).unwrap_err();
        assert!(matches!(err, CatalogError::BadCatalogSha { .. }));
    }
}
