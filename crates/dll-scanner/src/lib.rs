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
    DirectStorage,
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
            | DllFamily::FsrLoader => "amd",
            DllFamily::DirectStorage => "microsoft",
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
            DllFamily::DirectStorage => "direct_storage",
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
        ("dstorage.dll", DllFamily::DirectStorage),
        ("dstoragecore.dll", DllFamily::DirectStorage),
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
}
