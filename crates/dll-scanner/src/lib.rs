use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const SHA_SKIP_THRESHOLD_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DllFamily {
    DlssSr,
    DlssFg,
    DlssRr,
    Streamline,
    StreamlineCommon,
    StreamlinePcl,
    StreamlineNis,
    StreamlineDirectSr,
    Reflex,
    XessSr,
    XessSrDx11,
    XessFg,
    Xell,
    FsrUpscaler,
    FsrUpscalerVk,
    FsrFg,
    FsrLoader,
    FsrDenoiser,
    DirectStorage,
    DirectStorageCore,
}

impl DllFamily {
    pub fn vendor(&self) -> &'static str {
        match self {
            DllFamily::DlssSr
            | DllFamily::DlssFg
            | DllFamily::DlssRr
            | DllFamily::Streamline
            | DllFamily::StreamlineCommon
            | DllFamily::StreamlinePcl
            | DllFamily::StreamlineNis
            | DllFamily::StreamlineDirectSr
            | DllFamily::Reflex => "nvidia",
            DllFamily::XessSr | DllFamily::XessSrDx11 | DllFamily::XessFg | DllFamily::Xell => {
                "intel"
            }
            DllFamily::FsrUpscaler
            | DllFamily::FsrUpscalerVk
            | DllFamily::FsrFg
            | DllFamily::FsrLoader
            | DllFamily::FsrDenoiser => "amd",
            DllFamily::DirectStorage | DllFamily::DirectStorageCore => "microsoft",
        }
    }

    pub fn catalog_key(&self) -> &'static str {
        match self {
            DllFamily::DlssSr => "dlss_sr",
            DllFamily::DlssFg => "dlss_fg",
            DllFamily::DlssRr => "dlss_rr",
            DllFamily::Streamline
            | DllFamily::StreamlineCommon
            | DllFamily::StreamlinePcl
            | DllFamily::StreamlineNis
            | DllFamily::StreamlineDirectSr => "streamline",
            DllFamily::Reflex => "reflex",
            DllFamily::XessSr | DllFamily::XessSrDx11 => "xess_sr",
            DllFamily::XessFg => "xess_fg",
            DllFamily::Xell => "xell",
            DllFamily::FsrUpscaler | DllFamily::FsrUpscalerVk | DllFamily::FsrLoader => {
                "fsr_upscaler"
            }
            DllFamily::FsrFg => "fsr_fg",
            DllFamily::FsrDenoiser => "fsr_denoiser",
            DllFamily::DirectStorage => "direct_storage",
            DllFamily::DirectStorageCore => "direct_storage_core",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DllRecord {
    pub family: DllFamily,
    pub path: PathBuf,
    pub current_version: Option<String>,
    pub file_description: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// Marker DLLs that signal a DLSS/FG injector mod (DLSS Enabler, OptiScaler,
/// dlssg-to-fsr3) owns the Streamline set in this game tree. Matched
/// case-insensitively by EXACT file name — a bare `nvngx.dll` is an injector
/// proxy (real games ship `nvngx_dlss.dll` with the underscore), so generic
/// loaders like `dxgi.dll`/`version.dll` are deliberately excluded to keep
/// false positives at zero.
pub const DLSS_ENABLER_MARKERS: &[&str] = &[
    "dlss-enabler.dll",
    "dlss-enabler-upscaler.dll",
    "nvngx.dll",
    "optiscaler.dll",
    "dlssg_to_fsr3_amd_is_better.dll",
];

/// Bounded depth for the DLSS Enabler scan. The enabler DLLs sit near the game
/// root, so a shallow early-exit walk is far cheaper than the full DLL scan and
/// keeps enabler detection a separate, non-breaking command from `scan_install`.
const DLSS_ENABLER_SCAN_DEPTH: usize = 4;

/// Whether DLSS Enabler is installed in this game tree (bounded, early-exit walk).
/// Kept separate from `scan_install` so the DLL-detection contract stays a plain
/// `Vec<DllRecord>` — a partially-updated frontend/backend can never blank the
/// whole library on a shape mismatch.
pub fn detect_dlss_enabler(root: &Path) -> bool {
    enabler_present_within(root, 0)
}

fn enabler_present_within(dir: &Path, depth: usize) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() {
            if is_dlss_enabler_marker(&entry.file_name().to_string_lossy()) {
                return true;
            }
        } else if file_type.is_dir() && depth < DLSS_ENABLER_SCAN_DEPTH {
            let name = entry.file_name();
            if !SKIP_DIRS.contains(name.to_string_lossy().as_ref()) {
                subdirs.push(entry.path());
            }
        }
    }
    subdirs.iter().any(|d| enabler_present_within(d, depth + 1))
}

/// An NVIDIA Streamline plugin/interposer DLL (`sl.*.dll`). These form a single
/// version-locked set: swapping one component (e.g. `sl.dlss_g.dll`) without the
/// matching `sl.interposer.dll` mismatches Streamline's ABI struct versions and
/// crashes the game on launch. Distinct from the NGX runtime DLLs
/// (`nvngx_dlss.dll`), which the driver loads independently and are safe to swap.
pub fn is_streamline_plugin(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    lower.starts_with("sl.") && lower.ends_with(".dll")
}

/// Whether a single file name is a DLSS Enabler marker.
pub fn is_dlss_enabler_marker(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    DLSS_ENABLER_MARKERS.iter().any(|m| lower == *m)
}

/// Look for DLSS Enabler markers in `start` and up to `max_ancestors` parent
/// directories. Streamline plugins live in deeply nested engine plugin folders
/// while the enabler DLL sits near the game root, so the apply-time guard walks
/// upward from the DLL being replaced.
pub fn dlss_enabler_present_near(start: &Path, max_ancestors: usize) -> bool {
    let mut dir = Some(start);
    let mut hops = 0usize;
    while let Some(current) = dir {
        if dir_has_dlss_enabler(current) {
            return true;
        }
        if hops >= max_ancestors {
            break;
        }
        hops += 1;
        dir = current.parent();
    }
    false
}

fn dir_has_dlss_enabler(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        if is_dlss_enabler_marker(&entry.file_name().to_string_lossy()) {
            return true;
        }
    }
    false
}

static KNOWN_DLLS: Lazy<Vec<(&'static str, DllFamily)>> = Lazy::new(|| {
    vec![
        ("nvngx_dlss.dll", DllFamily::DlssSr),
        ("nvngx_dlssg.dll", DllFamily::DlssFg),
        ("nvngx_dlssd.dll", DllFamily::DlssRr),
        ("sl.dlss.dll", DllFamily::DlssSr),
        ("sl.dlss_g.dll", DllFamily::DlssFg),
        ("sl.dlss_d.dll", DllFamily::DlssRr),
        ("sl.interposer.dll", DllFamily::Streamline),
        ("sl.common.dll", DllFamily::StreamlineCommon),
        ("sl.pcl.dll", DllFamily::StreamlinePcl),
        ("sl.nis.dll", DllFamily::StreamlineNis),
        ("sl.directsr.dll", DllFamily::StreamlineDirectSr),
        ("sl.reflex.dll", DllFamily::Reflex),
        ("libxess.dll", DllFamily::XessSr),
        ("libxess_dx11.dll", DllFamily::XessSrDx11),
        ("libxess_fg.dll", DllFamily::XessFg),
        ("libxell.dll", DllFamily::Xell),
        ("amd_fidelityfx_dx12.dll", DllFamily::FsrUpscaler),
        ("amd_fidelityfx_vk.dll", DllFamily::FsrUpscalerVk),
        ("amd_fidelityfx_upscaler_dx12.dll", DllFamily::FsrUpscaler),
        ("amd_fidelityfx_framegeneration_dx12.dll", DllFamily::FsrFg),
        ("amd_fidelityfx_loader_dx12.dll", DllFamily::FsrLoader),
        ("ffx_fsr3upscaler_x64.dll", DllFamily::FsrUpscaler),
        ("ffx_frameinterpolation_x64.dll", DllFamily::FsrFg),
        ("amd_fidelityfx_denoiser_dx12.dll", DllFamily::FsrDenoiser),
        ("dstorage.dll", DllFamily::DirectStorage),
        ("dstoragecore.dll", DllFamily::DirectStorageCore),
    ]
});

static SKIP_DIRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "$Recycle.Bin",
        "System Volume Information",
        ".git",
        "node_modules",
        "__pycache__",
        ".vs",
        ".idea",
        "Logs",
        "Cache",
        "Crashes",
    ]
    .into_iter()
    .collect()
});

pub fn scan_install(root: &Path) -> Result<Vec<DllRecord>, ScanError> {
    let mut records = Vec::new();
    let walker = jwalk::WalkDir::new(root)
        .skip_hidden(false)
        .process_read_dir(|_, _, _, children| {
            children.retain(|child| {
                if let Ok(c) = child {
                    if c.file_type().is_dir() {
                        let n = c.file_name();
                        return !SKIP_DIRS.contains(n.to_string_lossy().as_ref());
                    }
                }
                true
            });
        });
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name();
        let lname = name.to_string_lossy().to_lowercase();
        if !lname.ends_with(".dll") {
            continue;
        }
        for (known, family) in KNOWN_DLLS.iter() {
            if lname == *known {
                let path = entry.path();
                let (current_version, file_description) = match pe_version::read_dll_version(&path)
                {
                    Ok(v) => (Some(v.file_version), v.file_description),
                    Err(_) => (None, None),
                };
                let sha256 = hash_file_capped(&path).ok().flatten();
                records.push(DllRecord {
                    family: *family,
                    path,
                    current_version,
                    file_description,
                    sha256,
                });
                break;
            }
        }
    }
    Ok(records)
}

pub fn hash_file_capped(path: &Path) -> std::io::Result<Option<String>> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > SHA_SKIP_THRESHOLD_BYTES {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest.iter() {
        out.push_str(&format!("{:02x}", b));
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn hash_file_capped_returns_hex_for_normal_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.dll");
        std::fs::write(&p, b"hello world").unwrap();
        let h = hash_file_capped(&p).unwrap().unwrap();
        assert_eq!(
            h,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn hash_file_capped_returns_none_above_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("huge.dll");
        let mut f = std::fs::File::create(&p).unwrap();
        let chunk = vec![0u8; 1024 * 1024];
        let chunks_needed = (SHA_SKIP_THRESHOLD_BYTES / chunk.len() as u64) + 1;
        for _ in 0..chunks_needed {
            f.write_all(&chunk).unwrap();
        }
        f.sync_all().unwrap();
        drop(f);
        let h = hash_file_capped(&p).unwrap();
        assert!(h.is_none());
    }

    #[test]
    fn hash_file_capped_handles_zero_byte_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("empty.dll");
        std::fs::File::create(&p).unwrap();
        let h = hash_file_capped(&p).unwrap().unwrap();
        assert_eq!(
            h,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hash_file_capped_propagates_missing_file_error() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("ghost.dll");
        let res = hash_file_capped(&p);
        assert!(res.is_err());
    }

    #[test]
    fn streamline_plugins_recognized_case_insensitively() {
        assert!(is_streamline_plugin("sl.dlss.dll"));
        assert!(is_streamline_plugin("sl.dlss_g.dll"));
        assert!(is_streamline_plugin("SL.Reflex.DLL"));
        assert!(is_streamline_plugin("sl.interposer.dll"));
        assert!(!is_streamline_plugin("nvngx_dlss.dll"));
        assert!(!is_streamline_plugin("nvngx_dlssg.dll"));
        assert!(!is_streamline_plugin("libxess.dll"));
        assert!(!is_streamline_plugin("slime.dll"));
    }

    #[test]
    fn dlss_enabler_markers_match_injector_dlls_not_generic_loaders() {
        assert!(is_dlss_enabler_marker("dlss-enabler.dll"));
        assert!(is_dlss_enabler_marker("DLSS-Enabler-Upscaler.dll"));
        assert!(is_dlss_enabler_marker("nvngx.dll"));
        assert!(is_dlss_enabler_marker("OptiScaler.dll"));
        assert!(!is_dlss_enabler_marker("nvngx_dlss.dll"));
        assert!(!is_dlss_enabler_marker("dxgi.dll"));
        assert!(!is_dlss_enabler_marker("sl.dlss.dll"));
    }

    #[test]
    fn dlss_enabler_present_near_walks_up_to_the_game_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("dlss-enabler.dll"), b"x").unwrap();
        let nested = root
            .path()
            .join("Engine/Plugins/Runtime/Nvidia/Streamline/Binaries/ThirdParty/Win64");
        std::fs::create_dir_all(&nested).unwrap();
        assert!(dlss_enabler_present_near(&nested, 8));
        assert!(!dlss_enabler_present_near(&nested, 2));
    }

    #[test]
    fn detect_dlss_enabler_finds_marker_in_nested_dir_and_keeps_scan_contract() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("Binaries/Win64");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("dlss-enabler.dll"), b"x").unwrap();
        std::fs::write(root.path().join("sl.dlss_g.dll"), b"x").unwrap();
        assert!(detect_dlss_enabler(root.path()));
        let records = scan_install(root.path()).unwrap();
        assert!(records
            .iter()
            .any(|r| r.path.file_name().unwrap() == "sl.dlss_g.dll"));
    }

    #[test]
    fn detect_dlss_enabler_is_false_without_markers() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("nvngx_dlss.dll"), b"x").unwrap();
        assert!(!detect_dlss_enabler(root.path()));
    }

    #[test]
    fn directstorage_core_and_fsr_denoiser_have_distinct_catalog_keys() {
        assert_eq!(
            DllFamily::DirectStorageCore.catalog_key(),
            "direct_storage_core"
        );
        assert_eq!(DllFamily::DirectStorageCore.vendor(), "microsoft");
        assert_eq!(DllFamily::FsrDenoiser.catalog_key(), "fsr_denoiser");
        assert_eq!(DllFamily::FsrDenoiser.vendor(), "amd");
    }

    #[test]
    fn scan_maps_dstoragecore_and_denoiser_to_their_own_families() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("dstoragecore.dll"), b"x").unwrap();
        std::fs::write(root.path().join("amd_fidelityfx_denoiser_dx12.dll"), b"x").unwrap();
        let recs = scan_install(root.path()).unwrap();
        assert!(recs
            .iter()
            .any(|r| r.family == DllFamily::DirectStorageCore));
        assert!(recs.iter().any(|r| r.family == DllFamily::FsrDenoiser));
    }
}
