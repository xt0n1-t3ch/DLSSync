use crate::error::AppResult;
use crate::state::AppState;
use anticheat_detect::{classify, HitSource, ProtectionHit, ProtectionKind, DEFAULT_SCAN_DEPTH};
use dll_catalog::{AntiCheatBinary, AntiCheatIndex};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;
use tauri::State;

const PCGW_APPID_REDIRECT: &str = "https://www.pcgamingwiki.com/api/appid.php?appid=";
const PCGW_NAME_SEARCH: &str = "https://www.pcgamingwiki.com/w/index.php?search=";
const PCGW_GLOSSARY_ANTI_CHEAT: &str = "https://www.pcgamingwiki.com/wiki/Glossary:Anti-cheat";
const PCGW_GLOSSARY_DRM: &str =
    "https://www.pcgamingwiki.com/wiki/Glossary:Digital_rights_management";

/// Percent-encode every byte that is not unreserved per RFC 3986. Plain ASCII
/// alphanumerics and `-._~` pass through; everything else (spaces, punctuation,
/// non-ASCII) becomes `%HH`. Used for the PCGW search URL fallback so a manual
/// launcher entry without a Steam app id still lands on a matching wiki page.
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{byte:02X}"));
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetectedAntiCheat {
    pub anticheat: String,
    pub kind: ProtectionKind,
    pub source: HitSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AntiCheatReport {
    pub detected: Vec<DetectedAntiCheat>,
    pub status: Option<String>,
    pub source_url: Option<String>,
}

/// Adapt the manifest's anti-cheat binary signatures into the `(needle, name)`
/// pairs the detection crate consumes, dropping rows with a blank `needle` or
/// `name` (an empty needle would substring-match every filename) and lowercasing
/// the needle to match the static baseline's convention. Empty input → empty
/// output → the compile-time baseline alone applies.
fn manifest_binary_signatures(entries: &[AntiCheatBinary]) -> Vec<(String, String)> {
    entries
        .iter()
        .filter_map(|e| {
            let needle = e.needle.trim();
            let name = e.name.trim();
            if needle.is_empty() || name.is_empty() {
                return None;
            }
            Some((needle.to_ascii_lowercase(), name.to_string()))
        })
        .collect()
}

/// Resolve the "Learn more" target for an anti-cheat banner.
///
/// Precedence (each falls through to the next when its key is missing):
/// 1. Per-game PCGW redirect via the Steam app id (`api/appid.php?appid={id}`).
/// 2. Name-based PCGW search (`w/index.php?search={encoded_name}`) so a manual
///    launcher entry still lands on the matching wiki page when one exists.
/// 3. Glossary entry for the strongest detected protection kind.
///
/// Returns `None` only when nothing was detected — the banner is hidden too.
fn learn_more_url(
    app_id: Option<&str>,
    name: Option<&str>,
    kinds: &[ProtectionKind],
) -> Option<String> {
    if kinds.is_empty() {
        return None;
    }
    if let Some(id) = app_id.and_then(|s| s.parse::<u32>().ok()) {
        return Some(format!("{PCGW_APPID_REDIRECT}{id}"));
    }
    if let Some(query) = name.map(str::trim).filter(|s| !s.is_empty()) {
        return Some(format!("{PCGW_NAME_SEARCH}{}", percent_encode(query)));
    }
    if kinds.contains(&ProtectionKind::AntiCheat) {
        return Some(PCGW_GLOSSARY_ANTI_CHEAT.to_string());
    }
    Some(PCGW_GLOSSARY_DRM.to_string())
}

/// Merge local evidence (anti-cheat binaries on disk + protector fingerprints in
/// the game executable) with the named dataset entry. Local hits win on name
/// collision (a binary or PE match is stronger than a dataset guess); each
/// detection carries its risk `kind` (anti-cheat → ban risk, anti-tamper →
/// launch-fail risk, DRM → informational) and its evidence `source`.
fn combine(
    local: Vec<ProtectionHit>,
    dataset_names: &[String],
    dataset_status: Option<String>,
    app_id: Option<&str>,
    game_name: Option<&str>,
) -> AntiCheatReport {
    let mut detected: Vec<DetectedAntiCheat> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for hit in local {
        if seen.insert(hit.name.clone()) {
            detected.push(DetectedAntiCheat {
                anticheat: hit.name,
                kind: hit.kind,
                source: hit.source,
            });
        }
    }
    for name in dataset_names {
        if seen.insert(name.clone()) {
            detected.push(DetectedAntiCheat {
                anticheat: name.clone(),
                kind: classify(name),
                source: HitSource::Dataset,
            });
        }
    }
    detected.sort_by(|a, b| a.anticheat.cmp(&b.anticheat));
    let kinds: Vec<ProtectionKind> = detected.iter().map(|d| d.kind).collect();
    let source_url = learn_more_url(app_id, game_name, &kinds);
    AntiCheatReport {
        detected,
        status: dataset_status,
        source_url,
    }
}

#[tauri::command]
pub async fn detect_anticheat(
    state: State<'_, AppState>,
    install_dir: String,
    app_id: Option<String>,
    name: String,
) -> AppResult<AntiCheatReport> {
    crate::paths::PathGuard::assert_safe_scan_dir(Path::new(&install_dir))
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let mut index = AntiCheatIndex::embedded();
    let extra_binaries: Vec<(String, String)> = {
        let guard = state.catalog.read();
        if let Some(catalog) = guard.as_ref() {
            if let Some(manifest) = catalog.anticheat.as_ref() {
                index.merge(manifest);
            }
            manifest_binary_signatures(&catalog.anti_cheat_binaries)
        } else {
            Vec::new()
        }
    };

    let scan_dir = install_dir.clone();
    let local: Vec<ProtectionHit> = tokio::task::spawn_blocking(move || {
        anticheat_detect::detect_protections(
            Path::new(&scan_dir),
            None,
            DEFAULT_SCAN_DEPTH,
            &extra_binaries,
        )
    })
    .await
    .map_err(|e| crate::error::AppError::Other(format!("protection scan task: {e}")))?;

    let (dataset_names, dataset_status) = index
        .lookup(app_id.as_deref(), &name)
        .map(|entry| {
            let mut names = entry.anticheats.clone();
            names.extend(entry.anti_tamper.clone());
            (names, entry.status.clone())
        })
        .unwrap_or_default();

    Ok(combine(
        local,
        &dataset_names,
        dataset_status,
        app_id.as_deref(),
        Some(name.as_str()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bin(needle: &str, name: &str) -> AntiCheatBinary {
        AntiCheatBinary {
            needle: needle.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn manifest_binary_signatures_empty_yields_empty() {
        assert!(manifest_binary_signatures(&[]).is_empty());
    }

    #[test]
    fn manifest_binary_signatures_lowercases_and_keeps_valid_rows() {
        let pairs = manifest_binary_signatures(&[bin("NewGuard64", "New Guard AC")]);
        assert_eq!(
            pairs,
            vec![("newguard64".to_string(), "New Guard AC".to_string())]
        );
    }

    #[test]
    fn manifest_binary_signatures_drops_blank_needle_or_name() {
        let pairs = manifest_binary_signatures(&[
            bin("", "Has No Needle"),
            bin("   ", "Whitespace Needle"),
            bin("hasnoname", ""),
            bin("keep", "Keep AC"),
        ]);
        assert_eq!(pairs, vec![("keep".to_string(), "Keep AC".to_string())]);
    }

    #[test]
    fn detection_merges_static_baseline_with_manifest_binaries() {
        use anticheat_detect::match_anti_cheat_binary_with;
        let extra = manifest_binary_signatures(&[bin("newguard", "New Guard AC")]);
        // Static baseline still matches its own rows…
        assert_eq!(
            match_anti_cheat_binary_with("EasyAntiCheat_x64.dll", &extra),
            Some("Easy Anti-Cheat")
        );
        // …and the manifest row adds a previously-unknown engine.
        assert_eq!(
            match_anti_cheat_binary_with("NewGuard64.sys", &extra),
            Some("New Guard AC")
        );
    }

    fn hit(name: &str, kind: ProtectionKind, source: HitSource) -> ProtectionHit {
        ProtectionHit {
            name: name.to_string(),
            kind,
            source,
        }
    }

    #[test]
    fn combine_keeps_local_over_dataset_and_dedupes() {
        let report = combine(
            vec![hit(
                "Easy Anti-Cheat",
                ProtectionKind::AntiCheat,
                HitSource::Binary,
            )],
            &["Easy Anti-Cheat".into(), "BattlEye".into()],
            Some("Broken".into()),
            Some("1245620"),
            Some("Elden Ring"),
        );
        assert_eq!(report.detected.len(), 2);
        assert_eq!(report.detected[0].anticheat, "BattlEye");
        assert_eq!(report.detected[0].source, HitSource::Dataset);
        assert_eq!(report.detected[1].anticheat, "Easy Anti-Cheat");
        assert_eq!(report.detected[1].source, HitSource::Binary);
        assert_eq!(report.status.as_deref(), Some("Broken"));
        assert_eq!(
            report.source_url.as_deref(),
            Some("https://www.pcgamingwiki.com/api/appid.php?appid=1245620")
        );
    }

    #[test]
    fn combine_classifies_dataset_names_into_kinds() {
        let report = combine(
            Vec::new(),
            &["Denuvo Anti-Tamper".into(), "BattlEye".into()],
            None,
            None,
            None,
        );
        let denuvo = report
            .detected
            .iter()
            .find(|d| d.anticheat == "Denuvo Anti-Tamper")
            .unwrap();
        assert_eq!(denuvo.kind, ProtectionKind::AntiTamper);
        let be = report
            .detected
            .iter()
            .find(|d| d.anticheat == "BattlEye")
            .unwrap();
        assert_eq!(be.kind, ProtectionKind::AntiCheat);
    }

    #[test]
    fn combine_empty_yields_no_detection_and_no_learn_more() {
        let report = combine(Vec::new(), &[], None, None, None);
        assert!(report.detected.is_empty());
        assert!(report.status.is_none());
        assert!(report.source_url.is_none());
    }

    #[test]
    fn combine_pe_hit_uses_name_search_when_no_appid_but_name_present() {
        let report = combine(
            vec![hit(
                "Denuvo Anti-Tamper",
                ProtectionKind::AntiTamper,
                HitSource::Pe,
            )],
            &[],
            None,
            None,
            Some("Assassin's Creed Shadows"),
        );
        assert_eq!(report.detected.len(), 1);
        assert_eq!(report.detected[0].source, HitSource::Pe);
        assert_eq!(report.detected[0].kind, ProtectionKind::AntiTamper);
        assert_eq!(
            report.source_url.as_deref(),
            Some("https://www.pcgamingwiki.com/w/index.php?search=Assassin%27s%20Creed%20Shadows")
        );
    }

    #[test]
    fn learn_more_url_per_game_pcgw_when_appid_numeric() {
        let url = learn_more_url(Some("3159330"), None, &[ProtectionKind::AntiTamper]).unwrap();
        assert_eq!(
            url,
            "https://www.pcgamingwiki.com/api/appid.php?appid=3159330"
        );
    }

    #[test]
    fn learn_more_url_appid_wins_over_name() {
        let url = learn_more_url(
            Some("3159330"),
            Some("Whatever"),
            &[ProtectionKind::AntiCheat],
        )
        .unwrap();
        assert!(url.ends_with("appid=3159330"));
    }

    #[test]
    fn learn_more_url_name_search_when_no_appid_but_name_present() {
        let url = learn_more_url(
            None,
            Some("Resident Evil Requiem"),
            &[ProtectionKind::AntiTamper],
        )
        .unwrap();
        assert_eq!(
            url,
            "https://www.pcgamingwiki.com/w/index.php?search=Resident%20Evil%20Requiem"
        );
    }

    #[test]
    fn learn_more_url_glossary_anti_cheat_when_no_appid_no_name_and_anti_cheat_present() {
        let url = learn_more_url(
            None,
            None,
            &[ProtectionKind::AntiTamper, ProtectionKind::AntiCheat],
        )
        .unwrap();
        assert_eq!(url, PCGW_GLOSSARY_ANTI_CHEAT);
    }

    #[test]
    fn learn_more_url_glossary_drm_when_no_appid_no_name_and_anti_tamper_only() {
        let url = learn_more_url(None, None, &[ProtectionKind::AntiTamper]).unwrap();
        assert_eq!(url, PCGW_GLOSSARY_DRM);
    }

    #[test]
    fn learn_more_url_none_when_no_detection_even_with_appid() {
        assert!(learn_more_url(Some("3159330"), Some("X"), &[]).is_none());
        assert!(learn_more_url(None, None, &[]).is_none());
    }

    #[test]
    fn learn_more_url_name_search_when_appid_unparseable() {
        let url = learn_more_url(
            Some("not-a-number"),
            Some("Half-Life 2"),
            &[ProtectionKind::AntiCheat],
        )
        .unwrap();
        assert_eq!(
            url,
            "https://www.pcgamingwiki.com/w/index.php?search=Half-Life%202"
        );
    }

    #[test]
    fn percent_encode_keeps_unreserved_and_escapes_the_rest() {
        assert_eq!(percent_encode("Half-Life 2"), "Half-Life%202");
        assert_eq!(percent_encode("Assassin's Creed"), "Assassin%27s%20Creed");
        assert_eq!(percent_encode("abcXYZ012-._~"), "abcXYZ012-._~");
        assert_eq!(percent_encode("a&b=c?d"), "a%26b%3Dc%3Fd");
    }
}
