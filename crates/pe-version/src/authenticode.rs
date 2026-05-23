use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticodeInfo {
    pub trusted: bool,
    pub subject_cn: Option<String>,
    pub subject_dn: Option<String>,
    pub issuer_dn: Option<String>,
    pub status: String,
}

#[cfg(windows)]
pub fn read_authenticode(path: &Path) -> Option<AuthenticodeInfo> {
    Some(win::read(path).unwrap_or_else(|status| AuthenticodeInfo {
        trusted: false,
        subject_cn: None,
        subject_dn: None,
        issuer_dn: None,
        status,
    }))
}

#[cfg(not(windows))]
pub fn read_authenticode(_path: &Path) -> Option<AuthenticodeInfo> {
    None
}

#[cfg(windows)]
mod win {
    use super::AuthenticodeInfo;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr::null_mut;
    use windows_sys::Win32::Security::Cryptography::*;

    const ENCODING: u32 = X509_ASN_ENCODING | PKCS_7_ASN_ENCODING;

    pub fn read(path: &Path) -> Result<AuthenticodeInfo, String> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut encoding: u32 = 0;
        let mut content_type: u32 = 0;
        let mut format_type: u32 = 0;
        let mut h_store: HCERTSTORE = null_mut();
        let mut h_msg: *mut std::ffi::c_void = null_mut();

        let ok = unsafe {
            CryptQueryObject(
                CERT_QUERY_OBJECT_FILE,
                wide.as_ptr() as *const _,
                CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
                CERT_QUERY_FORMAT_FLAG_BINARY,
                0,
                &mut encoding,
                &mut content_type,
                &mut format_type,
                &mut h_store,
                &mut h_msg,
                null_mut(),
            )
        };
        if ok == 0 {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            return Err(format!(
                "CryptQueryObject: 0x{err:08X} ({})",
                explain_err(err)
            ));
        }

        let result = (|| -> Result<AuthenticodeInfo, String> {
            let mut size: u32 = 0;
            let r1 = unsafe {
                CryptMsgGetParam(h_msg, CMSG_SIGNER_INFO_PARAM, 0, null_mut(), &mut size)
            };
            if r1 == 0 || size == 0 {
                return Err("signer info unavailable".to_string());
            }
            let mut signer_buf = vec![0u8; size as usize];
            let r2 = unsafe {
                CryptMsgGetParam(
                    h_msg,
                    CMSG_SIGNER_INFO_PARAM,
                    0,
                    signer_buf.as_mut_ptr() as *mut _,
                    &mut size,
                )
            };
            if r2 == 0 {
                return Err("CryptMsgGetParam read failed".to_string());
            }
            let signer = unsafe { &*(signer_buf.as_ptr() as *const CMSG_SIGNER_INFO) };

            let mut cert_info: CERT_INFO = unsafe { std::mem::zeroed() };
            cert_info.Issuer = signer.Issuer;
            cert_info.SerialNumber = signer.SerialNumber;

            let cert_ctx = unsafe {
                CertFindCertificateInStore(
                    h_store,
                    ENCODING,
                    0,
                    CERT_FIND_SUBJECT_CERT,
                    &cert_info as *const _ as *const _,
                    null_mut(),
                )
            };
            if cert_ctx.is_null() {
                return Err("signer cert not in attached store".to_string());
            }

            let subject_cn = cert_name_string(cert_ctx, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0);
            let subject_dn = cert_name_string(cert_ctx, CERT_NAME_RDN_TYPE, 0);
            let issuer_dn = cert_name_string(cert_ctx, CERT_NAME_RDN_TYPE, CERT_NAME_ISSUER_FLAG);

            unsafe {
                CertFreeCertificateContext(cert_ctx);
            }
            Ok(AuthenticodeInfo {
                trusted: true,
                subject_cn,
                subject_dn,
                issuer_dn,
                status: "Signed".to_string(),
            })
        })();

        unsafe {
            CertCloseStore(h_store, 0);
            CryptMsgClose(h_msg);
        }
        result
    }

    fn cert_name_string(
        cert_ctx: *const CERT_CONTEXT,
        display_type: u32,
        flags: u32,
    ) -> Option<String> {
        unsafe {
            let needed =
                CertGetNameStringW(cert_ctx, display_type, flags, null_mut(), null_mut(), 0);
            if needed <= 1 {
                return None;
            }
            let mut buf = vec![0u16; needed as usize];
            let written = CertGetNameStringW(
                cert_ctx,
                display_type,
                flags,
                null_mut(),
                buf.as_mut_ptr(),
                needed,
            );
            if written <= 1 {
                return None;
            }
            let s = String::from_utf16_lossy(&buf[..(written as usize).saturating_sub(1)]);
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
    }

    fn explain_err(code: u32) -> &'static str {
        match code {
            0x80092002 => "CRYPT_E_BAD_ENCODE",
            0x80092009 => "CRYPT_E_NO_MATCH",
            0x8009100D => "CRYPT_E_INVALID_MSG_TYPE",
            0x80092004 => "CRYPT_E_NOT_FOUND",
            0x80092005 => "CRYPT_E_NO_KEY_PROPERTY",
            _ => "Win32 error",
        }
    }
}

pub fn allowed_subjects(vendor: &str) -> &'static [&'static str] {
    match vendor {
        "nvidia" => &["NVIDIA Corporation", "Nvidia Corporation"],
        "intel" => &[
            "Intel Corporation",
            "Intel(R) Software Development Products",
        ],
        "amd" => &[
            "Advanced Micro Devices, Inc.",
            "Advanced Micro Devices",
            "ATI Technologies Inc.",
        ],
        "microsoft" => &[
            "Microsoft Corporation",
            "Microsoft Windows",
            "Microsoft Windows Hardware Compatibility Publisher",
        ],
        _ => &[],
    }
}

pub fn enforce_subject(info: &AuthenticodeInfo, vendor: &str) -> Result<(), String> {
    let allowed = allowed_subjects(vendor);
    if allowed.is_empty() {
        return Err(format!("unknown vendor '{vendor}'"));
    }
    let Some(cn) = info.subject_cn.as_deref() else {
        return Err(format!(
            "no Authenticode subject extracted (status: {})",
            info.status
        ));
    };
    if allowed.iter().any(|a| a.eq_ignore_ascii_case(cn)) {
        Ok(())
    } else {
        Err(format!(
            "Authenticode subject '{cn}' not in {vendor} allowlist {allowed:?}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_cn_basic() {
        let info = AuthenticodeInfo {
            trusted: true,
            subject_cn: Some("NVIDIA Corporation".into()),
            subject_dn: None,
            issuer_dn: None,
            status: "Signed".into(),
        };
        assert!(enforce_subject(&info, "nvidia").is_ok());
        assert!(enforce_subject(&info, "amd").is_err());
    }

    #[test]
    fn extract_cn_rejects_missing_subject() {
        let info = AuthenticodeInfo {
            trusted: false,
            subject_cn: None,
            subject_dn: None,
            issuer_dn: None,
            status: "NotSigned".into(),
        };
        assert!(enforce_subject(&info, "nvidia").is_err());
    }

    #[test]
    fn allowlist_is_case_insensitive() {
        let info = AuthenticodeInfo {
            trusted: true,
            subject_cn: Some("nvidia corporation".into()),
            subject_dn: None,
            issuer_dn: None,
            status: "Signed".into(),
        };
        assert!(enforce_subject(&info, "nvidia").is_ok());
    }

    fn windows_system32_path(leaf: &str) -> std::path::PathBuf {
        let windir = std::env::var_os("SystemRoot")
            .or_else(|| std::env::var_os("WINDIR"))
            .unwrap_or_else(|| std::ffi::OsString::from(r"C:\Windows"));
        std::path::PathBuf::from(windir).join("System32").join(leaf)
    }

    #[test]
    fn live_authenticode_extract_from_signed_dll() {
        const PROBES: &[&str] = &["kernel32.dll", "user32.dll", "advapi32.dll"];
        for leaf in PROBES {
            let p = windows_system32_path(leaf);
            if !p.exists() {
                continue;
            }
            let info = read_authenticode(&p).expect("never panics");
            if info.subject_cn.is_some() {
                let cn = info.subject_cn.as_deref().unwrap_or("");
                assert!(
                    cn.to_ascii_lowercase().contains("microsoft"),
                    "expected Microsoft subject in {leaf}, got {cn:?} status={}",
                    info.status
                );
                return;
            }
        }
        eprintln!("live signed DLL probe skipped — no candidate file present");
    }

    #[test]
    fn read_returns_unsigned_for_garbage_file() {
        let dir = std::env::temp_dir().join("dlssync-pe-version-tests");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("unsigned.bin");
        std::fs::write(&path, b"not a real PE").unwrap();
        let info = read_authenticode(&path).expect("never panics");
        assert!(!info.trusted);
        assert!(info.subject_cn.is_none());
        let _ = std::fs::remove_file(&path);
    }
}
