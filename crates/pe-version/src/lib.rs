use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod authenticode;
pub use authenticode::{allowed_subjects, enforce_subject, read_authenticode, AuthenticodeInfo};

#[derive(Debug, thiserror::Error)]
pub enum VersionError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("pelite: {0}")]
    Pe(#[from] pelite::Error),
    #[error("find: {0}")]
    Find(#[from] pelite::resources::FindError),
    #[error("resource not found")]
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DllVersion {
    pub file_version: String,
    pub product_version: String,
    pub file_version_packed: u64,
    pub file_description: Option<String>,
    pub product_name: Option<String>,
    pub original_filename: Option<String>,
    pub company_name: Option<String>,
}

pub fn read_dll_version(path: &Path) -> Result<DllVersion, VersionError> {
    let bytes = std::fs::read(path)?;
    parse_bytes(&bytes)
}

pub fn parse_bytes(bytes: &[u8]) -> Result<DllVersion, VersionError> {
    use pelite::Wrap;
    let pe = pelite::PeFile::from_bytes(bytes)?;
    let resources = match pe {
        Wrap::T32(file) => {
            use pelite::pe32::Pe;
            file.resources()?
        }
        Wrap::T64(file) => {
            use pelite::pe64::Pe;
            file.resources()?
        }
    };
    extract_from_resources(&resources)
}

fn extract_from_resources(
    resources: &pelite::resources::Resources<'_>,
) -> Result<DllVersion, VersionError> {
    let vi = resources.version_info()?;
    let fixed = vi.fixed().ok_or(VersionError::Missing)?;
    let file_version = fixed.dwFileVersion.to_string();
    let product_version = fixed.dwProductVersion.to_string();
    let v = fixed.dwFileVersion;
    let file_version_packed = ((v.Major as u64) << 48)
        | ((v.Minor as u64) << 32)
        | ((v.Build as u64) << 16)
        | (v.Patch as u64);
    let lang = vi.translation().first().copied();

    let read = |key: &str| -> Option<String> { lang.and_then(|l| vi.value(l, key)) };

    Ok(DllVersion {
        file_version,
        product_version,
        file_version_packed,
        file_description: read("FileDescription"),
        product_name: read("ProductName"),
        original_filename: read("OriginalFilename"),
        company_name: read("CompanyName"),
    })
}

#[cfg(test)]
mod tests {
    fn pack(major: u16, minor: u16, build: u16, patch: u16) -> u64 {
        ((major as u64) << 48) | ((minor as u64) << 32) | ((build as u64) << 16) | (patch as u64)
    }

    #[test]
    fn packs_components_into_u64() {
        assert_eq!(pack(310, 6, 0, 0), 0x0136_0006_0000_0000);
        assert_eq!(pack(3, 10, 6, 0), 0x0003_000a_0006_0000);
        assert_eq!(pack(1, 0, 0, 0), 0x0001_0000_0000_0000);
    }

    #[test]
    fn parse_bytes_rejects_non_pe_input() {
        let err = super::parse_bytes(b"not a pe file at all").unwrap_err();
        match err {
            super::VersionError::Pe(_) => {}
            other => panic!("expected pelite error, got {other:?}"),
        }
    }
}
