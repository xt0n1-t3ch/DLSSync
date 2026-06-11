use url::Url;

/// Known GPU driver vendors whose download URLs are subject to the per-vendor
/// https allowlist enforced by [`validate_driver_url`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverVendor {
    Nvidia,
    Amd,
    Intel,
}

impl DriverVendor {
    /// Parse a case-insensitive vendor string (`"nvidia"`, `"amd"`, `"intel"`).
    /// Returns `None` for unrecognised values so callers can surface a clear error.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "nvidia" => Some(Self::Nvidia),
            "amd" => Some(Self::Amd),
            "intel" => Some(Self::Intel),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nvidia => "nvidia",
            Self::Amd => "amd",
            Self::Intel => "intel",
        }
    }

    fn allowed_host_suffixes(self) -> &'static [&'static str] {
        match self {
            Self::Nvidia => &[".nvidia.com", ".geforce.com"],
            Self::Amd => &[".amd.com", ".radeon.com"],
            Self::Intel => &[".intel.com"],
        }
    }
}

/// Errors produced by [`validate_driver_url`] when a proposed download URL
/// violates the network-policy rules.
#[derive(Debug, thiserror::Error)]
pub enum NetPolicyError {
    #[error("unknown driver vendor: {0}")]
    UnknownVendor(String),
    #[error("download url is not a valid absolute url")]
    MalformedUrl,
    #[error("download url must use https, got: {0}")]
    NotHttps(String),
    #[error("download url has no host")]
    NoHost,
    #[error("download host {host} is not on the {vendor} allowlist")]
    HostNotAllowed { vendor: &'static str, host: String },
}

/// Returns `true` when `host` exactly equals `bare` (the suffix without the
/// leading dot) **or** ends with the full `suffix` (e.g. `.nvidia.com`).
/// Prevents lookalike matches: `evil-nvidia.com` does NOT end with `.nvidia.com`.
fn host_matches_suffix(host: &str, suffix: &str) -> bool {
    let bare = suffix.trim_start_matches('.');
    host == bare || host.ends_with(suffix)
}

/// Validate that `url` is an https download URL whose host belongs to `vendor`'s
/// official domains. Rejects unknown vendors, non-https schemes (no cleartext or
/// other-protocol downgrade), missing hosts, IP literals, and any host outside
/// the per-vendor allowlist. Returns the parsed [`DriverVendor`] on success so
/// callers can reuse the validated value instead of re-parsing the raw string.
pub fn validate_driver_url(vendor: &str, url: &str) -> Result<DriverVendor, NetPolicyError> {
    let parsed_vendor =
        DriverVendor::parse(vendor).ok_or_else(|| NetPolicyError::UnknownVendor(vendor.into()))?;

    let parsed = Url::parse(url).map_err(|_| NetPolicyError::MalformedUrl)?;
    if parsed.scheme() != "https" {
        return Err(NetPolicyError::NotHttps(parsed.scheme().into()));
    }

    let host = match parsed.host() {
        Some(url::Host::Domain(domain)) => domain.to_ascii_lowercase(),
        _ => return Err(NetPolicyError::NoHost),
    };

    if parsed_vendor
        .allowed_host_suffixes()
        .iter()
        .any(|suffix| host_matches_suffix(&host, suffix))
    {
        Ok(parsed_vendor)
    } else {
        Err(NetPolicyError::HostNotAllowed {
            vendor: parsed_vendor.as_str(),
            host,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_official_vendor_hosts() {
        assert!(validate_driver_url(
            "nvidia",
            "https://us.download.nvidia.com/Windows/580.00/580.00-desktop-win10-win11-64bit.exe"
        )
        .is_ok());
        assert!(validate_driver_url(
            "amd",
            "https://drivers.amd.com/drivers/installer/whql-amd-software-adrenalin.exe"
        )
        .is_ok());
        assert!(validate_driver_url(
            "intel",
            "https://downloadmirror.intel.com/12345/gfx_win_101.exe"
        )
        .is_ok());
    }

    #[test]
    fn rejects_unknown_vendor() {
        assert!(matches!(
            validate_driver_url("evil", "https://drivers.amd.com/x.exe"),
            Err(NetPolicyError::UnknownVendor(_))
        ));
    }

    #[test]
    fn rejects_off_domain_host() {
        assert!(matches!(
            validate_driver_url("nvidia", "https://evil.example.com/payload.exe"),
            Err(NetPolicyError::HostNotAllowed { .. })
        ));
    }

    #[test]
    fn rejects_lookalike_suffix() {
        assert!(matches!(
            validate_driver_url("amd", "https://evil-amd.com/payload.exe"),
            Err(NetPolicyError::HostNotAllowed { .. })
        ));
        assert!(matches!(
            validate_driver_url("amd", "https://amd.com.evil.com/payload.exe"),
            Err(NetPolicyError::HostNotAllowed { .. })
        ));
    }

    #[test]
    fn rejects_userinfo_host_confusion() {
        assert!(matches!(
            validate_driver_url("amd", "https://drivers.amd.com@evil.com/payload.exe"),
            Err(NetPolicyError::HostNotAllowed { .. })
        ));
    }

    #[test]
    fn rejects_non_https() {
        assert!(matches!(
            validate_driver_url("nvidia", "http://us.download.nvidia.com/x.exe"),
            Err(NetPolicyError::NotHttps(_))
        ));
        assert!(matches!(
            validate_driver_url("nvidia", "file:///c:/windows/system32/evil.exe"),
            Err(NetPolicyError::NotHttps(_))
        ));
    }

    #[test]
    fn rejects_ip_literal_and_malformed() {
        assert!(matches!(
            validate_driver_url("intel", "https://203.0.113.5/x.exe"),
            Err(NetPolicyError::NoHost)
        ));
        assert!(matches!(
            validate_driver_url("amd", "not-a-url"),
            Err(NetPolicyError::MalformedUrl)
        ));
    }

    #[test]
    fn cross_vendor_host_is_rejected() {
        assert!(matches!(
            validate_driver_url("nvidia", "https://drivers.amd.com/x.exe"),
            Err(NetPolicyError::HostNotAllowed { .. })
        ));
    }
}
