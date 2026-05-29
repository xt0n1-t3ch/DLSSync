//! Pure driver-version + driver-date logic for the anti-downgrade guard.
//!
//! Windows Update / the Microsoft Update Catalog is known to surface driver
//! packages that are OLDER than what a device already runs. The guard never
//! offers a candidate unless it is provably newer than the installed driver.
//! "Newer" is decided on driver DATE first (the only field both WMI and WUA
//! report reliably) and on the dotted version as a tie-break.

/// A dotted numeric driver version, e.g. `31.0.101.2141`. Compared
/// component-by-component, zero-padded so `31.0.101.2141` > `31.0.101.999`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverVersion(pub Vec<u64>);

impl DriverVersion {
    /// Parse a dotted numeric version. Non-numeric tails are ignored; returns
    /// `None` when no leading numeric component is found.
    pub fn parse(raw: &str) -> Option<Self> {
        let mut parts = Vec::new();
        for seg in raw.trim().split('.') {
            let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() {
                break;
            }
            match digits.parse::<u64>() {
                Ok(n) => parts.push(n),
                Err(_) => break,
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(DriverVersion(parts))
        }
    }

    /// Order against another version, padding the shorter with zeros.
    pub fn cmp_padded(&self, other: &DriverVersion) -> std::cmp::Ordering {
        let len = self.0.len().max(other.0.len());
        for i in 0..len {
            let a = self.0.get(i).copied().unwrap_or(0);
            let b = other.0.get(i).copied().unwrap_or(0);
            match a.cmp(&b) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }
        std::cmp::Ordering::Equal
    }
}

/// Extract the first dotted version (`\d+(\.\d+){1,3}`) embedded in free text
/// such as a WUA update title ("Intel - Display - 31.0.101.2141"). Returns the
/// matched substring so it can be displayed and parsed.
pub fn extract_version(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0usize;
            while i < bytes.len() {
                if bytes[i].is_ascii_digit() {
                    i += 1;
                } else if bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
                    dots += 1;
                    i += 1;
                } else {
                    break;
                }
            }
            if dots >= 1 {
                return Some(text[start..i].to_string());
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Convert an OLE automation date (days since 1899-12-30, as returned by
/// `IWindowsDriverUpdate::DriverVerDate`) into an ISO `YYYY-MM-DD` string.
/// Uses Howard Hinnant's civil-from-days algorithm; pure integer math.
pub fn ole_date_to_iso(ole: f64) -> Option<String> {
    if !ole.is_finite() || ole <= 0.0 {
        return None;
    }
    let days_since_unix = ole.floor() as i64 - 25569;
    let (y, m, d) = civil_from_days(days_since_unix);
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// `(year, month, day)` from a count of days since 1970-01-01 (may be negative).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Decide whether `candidate` is strictly newer than `installed` for the
/// anti-downgrade guard. Date dominates; dotted version breaks ties; when
/// neither side is comparable the candidate is treated as NOT newer (refuse).
///
/// `*_date` are ISO `YYYY-MM-DD` (lexicographically ordered == chronological).
pub fn is_newer(
    candidate_date: Option<&str>,
    candidate_version: Option<&str>,
    installed_date: Option<&str>,
    installed_version: Option<&str>,
) -> bool {
    match (candidate_date, installed_date) {
        (Some(c), Some(i)) if c != i => return c > i,
        _ => {}
    }
    if let (Some(c), Some(i)) = (candidate_version, installed_version) {
        if let (Some(cv), Some(iv)) = (DriverVersion::parse(c), DriverVersion::parse(i)) {
            return cv.cmp_padded(&iv) == std::cmp::Ordering::Greater;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn parses_dotted_versions() {
        assert_eq!(
            DriverVersion::parse("31.0.101.2141").unwrap().0,
            vec![31, 0, 101, 2141]
        );
        assert_eq!(DriverVersion::parse("560.94").unwrap().0, vec![560, 94]);
        assert!(DriverVersion::parse("").is_none());
        assert!(DriverVersion::parse("n/a").is_none());
        assert_eq!(
            DriverVersion::parse("32.0.101.8801-beta").unwrap().0,
            vec![32, 0, 101, 8801]
        );
    }

    #[test]
    fn compares_padded() {
        let a = DriverVersion::parse("31.0.101.2141").unwrap();
        let b = DriverVersion::parse("31.0.101.999").unwrap();
        assert_eq!(a.cmp_padded(&b), Ordering::Greater);
        let c = DriverVersion::parse("31.0.101").unwrap();
        let d = DriverVersion::parse("31.0.101.0").unwrap();
        assert_eq!(c.cmp_padded(&d), Ordering::Equal);
        let e = DriverVersion::parse("10.0.19041.1").unwrap();
        let f = DriverVersion::parse("10.0.19041.2").unwrap();
        assert_eq!(e.cmp_padded(&f), Ordering::Less);
    }

    #[test]
    fn extracts_version_from_title() {
        assert_eq!(
            extract_version("Intel - Display - 31.0.101.2141").as_deref(),
            Some("31.0.101.2141")
        );
        assert_eq!(
            extract_version("Realtek Semiconductor Corp. - MEDIA - 6.0.9605.1").as_deref(),
            Some("6.0.9605.1")
        );
        assert_eq!(extract_version("No version here").as_deref(), None);
        assert_eq!(extract_version("Update 5 of 10").as_deref(), None);
    }

    #[test]
    fn ole_date_converts() {
        assert_eq!(ole_date_to_iso(45000.0).as_deref(), Some("2023-03-15"));
        assert_eq!(ole_date_to_iso(46172.0).as_deref(), Some("2026-05-30"));
        assert_eq!(ole_date_to_iso(0.0), None);
        assert_eq!(ole_date_to_iso(f64::NAN), None);
    }

    #[test]
    fn newer_by_date() {
        assert!(is_newer(Some("2026-05-15"), None, Some("2026-04-06"), None));
        assert!(!is_newer(
            Some("2026-03-01"),
            None,
            Some("2026-04-06"),
            None
        ));
        assert!(!is_newer(
            Some("2026-04-06"),
            None,
            Some("2026-04-06"),
            None
        ));
    }

    #[test]
    fn newer_by_version_when_dates_tie() {
        assert!(is_newer(
            Some("2026-04-06"),
            Some("31.0.101.2141"),
            Some("2026-04-06"),
            Some("31.0.101.999"),
        ));
        assert!(!is_newer(
            Some("2026-04-06"),
            Some("31.0.101.500"),
            Some("2026-04-06"),
            Some("31.0.101.999"),
        ));
    }

    #[test]
    fn refuses_when_uncomparable() {
        assert!(!is_newer(Some("2026-05-15"), None, None, None));
        assert!(!is_newer(None, None, None, None));
    }
}
