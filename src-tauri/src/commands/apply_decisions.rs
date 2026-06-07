use crate::commands::apply::{ApplyOutcome, ApplyRequest};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

pub fn group_id_for(cdn_url: &str) -> String {
    let mut h = Sha256::new();
    h.update(cdn_url.as_bytes());
    let digest = h.finalize();
    let mut s = String::with_capacity(16);
    for b in digest.iter().take(8) {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

pub fn classify_error(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("cancelled") {
        return "cancelled";
    }
    if lower.contains("error sending request")
        || lower.contains("decoding response body")
        || lower.contains("connection reset")
        || lower.contains("dns")
        || lower.contains("timed out")
        || lower.contains("stalled")
        || lower.contains("truncated")
        || lower.contains("size mismatch")
        || lower.contains("429")
        || lower.contains("503")
        || lower.contains("502")
        || lower.contains("504")
    {
        return "network";
    }
    if lower.contains("crypt_e_no_match")
        || lower.contains("allow unsigned")
        || lower.contains("authenticode signature could not be read")
        || lower.contains("no authenticode subject")
        || lower.contains("allowlist")
    {
        return "signature";
    }
    if lower.contains("locked by another process") || lower.contains("sharing_violation") {
        return "lock";
    }
    if lower.contains("access denied") || lower.contains("administrator") {
        return "permission";
    }
    if lower.contains("sha-256 mismatch") || lower.contains("integrity") || lower.contains("md5") {
        return "hash";
    }
    if lower.contains("not in zip") || lower.contains("dll not found") {
        return "missing";
    }
    if lower.contains("backup") {
        return "backup";
    }
    if lower.contains("streamline plugin")
        || lower.contains("injector mod")
        || lower.contains("version-locked")
    {
        return "streamline_locked";
    }
    "other"
}

/// First numeric dotted segment of a version string (the SDK major), or `None`.
pub(crate) fn version_major(version: &str) -> Option<u16> {
    version
        .split('.')
        .next()
        .map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
        })
        .filter(|p| !p.is_empty())
        .and_then(|p| p.parse().ok())
}

/// Pure decision for the Streamline guard, split out so it is testable without a
/// live `StateHandles`. An `sl.*` plugin is blocked only when the user has not
/// opted in (`update_streamline`) or the target crosses the installed Streamline
/// MAJOR; a same-major swap is allowed even under a DLSS Enabler. `None` = safe
/// to swap.
pub(crate) fn streamline_block_reason(
    filename: &str,
    allow_streamline: bool,
    installed_major: Option<u16>,
    target_major: Option<u16>,
) -> Option<String> {
    if !dll_scanner::is_streamline_plugin(filename) {
        return None;
    }
    if !allow_streamline {
        return Some(format!(
            "{filename} is an NVIDIA Streamline plugin (sl.*). Updating it without the matching \
             sl.interposer.dll can crash the game on launch. Enable 'Update NVIDIA Streamline \
             runtime' in Settings → Advanced to override."
        ));
    }
    if let (Some(installed), Some(target)) = (installed_major, target_major) {
        if installed != target {
            return Some(format!(
                "{filename} is NVIDIA Streamline v{installed}.x in this game but the update is \
                 Streamline v{target}.x. The Streamline plug-ins are version-locked as a matched \
                 set — mixing major releases (v{installed} with v{target}) crashes the game on \
                 launch. Skipped; only a same-major Streamline set update is applied."
            ));
        }
    }
    None
}

pub(crate) fn enrich_signature_error(reason: &str) -> String {
    let lower = reason.to_ascii_lowercase();
    let no_match = lower.contains("crypt_e_no_match")
        || lower.contains("notsigned")
        || lower.contains("not_signed")
        || lower.contains("could not be read")
        || lower.contains("no authenticode subject");
    if no_match {
        format!(
            "{reason}\n\nHint: this DLL ships unsigned by the vendor. \
             Enable 'Allow unsigned DLLs' in Settings → Advanced to override (SHA-256 \
             integrity is still enforced)."
        )
    } else {
        reason.to_string()
    }
}

pub(crate) fn failure_outcome(
    request: &ApplyRequest,
    _group_id: &str,
    error: String,
) -> ApplyOutcome {
    ApplyOutcome {
        apply_id: request.apply_id.clone(),
        success: false,
        backup_id: None,
        previous_version: None,
        new_version: None,
        error: Some(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrich_appends_hint_on_crypt_no_match() {
        let out = enrich_signature_error("CryptQueryObject: 0x80092009 (CRYPT_E_NO_MATCH)");
        assert!(out.contains("Allow unsigned DLLs"));
        assert!(out.contains("CRYPT_E_NO_MATCH"));
    }

    #[test]
    fn enrich_appends_hint_on_no_subject() {
        let out = enrich_signature_error("no Authenticode subject extracted (status: NotSigned)");
        assert!(out.contains("Allow unsigned DLLs"));
    }

    #[test]
    fn enrich_leaves_subject_allowlist_errors_alone() {
        let original = "Authenticode subject 'WrongCorp' not in nvidia allowlist";
        let out = enrich_signature_error(original);
        assert_eq!(out, original);
    }

    #[test]
    fn classify_recognizes_network_class() {
        assert_eq!(
            classify_error("catalog error: http: error decoding response body"),
            "network"
        );
        assert_eq!(
            classify_error("catalog error: http: error sending request for url"),
            "network"
        );
        assert_eq!(classify_error("stalled: no bytes for 60 s"), "network");
        assert_eq!(
            classify_error("truncated: received 100 bytes of 200"),
            "network"
        );
    }

    #[test]
    fn classify_recognizes_signature_class() {
        assert_eq!(
            classify_error("Authenticode subject 'X' not in nvidia allowlist"),
            "signature"
        );
        assert_eq!(
            classify_error("CryptQueryObject: 0x80092009 (CRYPT_E_NO_MATCH)"),
            "signature"
        );
    }

    #[test]
    fn classify_recognizes_lock_class() {
        assert_eq!(
            classify_error("file is locked by another process (X.dll)"),
            "lock"
        );
    }

    #[test]
    fn classify_recognizes_hash_class() {
        assert_eq!(
            classify_error("SHA-256 mismatch: expected abc got def"),
            "hash"
        );
    }

    #[test]
    fn group_id_is_deterministic_and_stable() {
        let a = group_id_for(
            "https://github.com/intel/xess/releases/download/v3.0.1/XeSS_SDK_3.0.1.zip",
        );
        let b = group_id_for(
            "https://github.com/intel/xess/releases/download/v3.0.1/XeSS_SDK_3.0.1.zip",
        );
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn group_id_differs_per_url() {
        let a = group_id_for("https://example.test/a.zip");
        let b = group_id_for("https://example.test/b.zip");
        assert_ne!(a, b);
    }

    #[test]
    fn streamline_guard_allows_ngx_dll_regardless_of_prefs() {
        assert!(streamline_block_reason("nvngx_dlss.dll", false, None, None).is_none());
        assert!(streamline_block_reason("nvngx_dlssg.dll", false, Some(1), Some(2)).is_none());
        assert!(streamline_block_reason("libxess.dll", false, None, None).is_none());
    }

    #[test]
    fn streamline_guard_blocks_plugin_when_streamline_opt_in_off() {
        let reason = streamline_block_reason("sl.dlss_g.dll", false, None, None).unwrap();
        assert!(reason.contains("Streamline"));
        assert!(reason.contains("Settings"));
        assert_eq!(classify_error(&reason), "streamline_locked");
    }

    #[test]
    fn streamline_guard_allows_same_major_plugin_when_opted_in() {
        assert!(streamline_block_reason("sl.dlss_g.dll", true, Some(2), Some(2)).is_none());
        assert!(streamline_block_reason("sl.reflex.dll", true, None, None).is_none());
    }

    #[test]
    fn streamline_guard_allows_same_major_swap_under_dlss_enabler() {
        assert!(streamline_block_reason("sl.dlss_g.dll", true, Some(2), Some(2)).is_none());
        assert!(streamline_block_reason("sl.reflex.dll", true, Some(2), Some(2)).is_none());
    }

    #[test]
    fn streamline_guard_blocks_sl_dlss_g_cross_major_nexus_subnautica2() {
        let reason = streamline_block_reason("sl.dlss_g.dll", true, Some(2), Some(310)).unwrap();
        assert!(reason.contains("version-locked"));
        assert_eq!(classify_error(&reason), "streamline_locked");
    }

    #[test]
    fn streamline_guard_blocks_cross_major_swap_even_when_opted_in() {
        let reason = streamline_block_reason("sl.interposer.dll", true, Some(1), Some(2)).unwrap();
        assert!(reason.contains("version-locked"));
        assert!(reason.contains("v1"));
        assert!(reason.contains("v2"));
        assert_eq!(classify_error(&reason), "streamline_locked");
    }

    #[test]
    fn version_major_parses_first_numeric_segment() {
        assert_eq!(version_major("2.11.1"), Some(2));
        assert_eq!(version_major("1.5.0.0"), Some(1));
        assert_eq!(version_major("310.6.0.0"), Some(310));
        assert_eq!(version_major(""), None);
    }
}
