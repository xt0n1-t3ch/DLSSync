use crate::DriverInstallError;
use pe_version::{enforce_subject, read_authenticode, AuthenticodeInfo};
use std::path::Path;

pub fn verify_signature(path: &Path, vendor: &str) -> Result<AuthenticodeInfo, DriverInstallError> {
    let info = read_authenticode(path).ok_or_else(|| {
        DriverInstallError::Signature(
            "Authenticode verification unavailable on this platform".to_string(),
        )
    })?;
    enforce_subject(&info, vendor).map_err(DriverInstallError::Signature)?;
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsigned_file() {
        let dir = std::env::temp_dir().join("dlssync-driver-install-verify-tests");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("fake-driver.exe");
        std::fs::write(&path, b"not a real installer").unwrap();
        assert!(verify_signature(&path, "nvidia").is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[cfg(windows)]
    #[test]
    fn accepts_microsoft_signed_system_binary() {
        let windir = std::env::var_os("SystemRoot")
            .unwrap_or_else(|| std::ffi::OsString::from(r"C:\Windows"));
        for leaf in ["System32\\kernel32.dll", "System32\\user32.dll"] {
            let candidate = std::path::Path::new(&windir).join(leaf);
            if !candidate.exists() {
                continue;
            }
            if read_authenticode(&candidate)
                .and_then(|info| info.subject_cn)
                .is_some()
            {
                assert!(verify_signature(&candidate, "microsoft").is_ok());
                return;
            }
        }
        eprintln!("no signed system binary available — skipped");
    }
}
