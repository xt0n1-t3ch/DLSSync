//! Local game-protection detection: anti-cheat binaries on disk plus
//! packer/protector fingerprints (Denuvo, VMProtect, Themida, Steam CEG …) in
//! the game executable. The named-dataset overlay (PCGamingWiki) is merged by
//! the command layer; this crate owns the on-disk + in-binary evidence.

pub mod binaries;
pub mod entropy;
pub mod pe_inspect;
pub mod signatures;

pub use signatures::{
    classify, match_anti_cheat_binary, match_anti_cheat_binary_with, HitSource, ProtectionKind,
};

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_SCAN_DEPTH: usize = 5;

/// One detected protection with its risk category and where the evidence came
/// from. `name` is the canonical engine/protector name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectionHit {
    pub name: String,
    pub kind: ProtectionKind,
    pub source: HitSource,
}

/// Detect protections from local files in a single directory walk: anti-cheat
/// binaries anywhere under the install directory, plus packer/protector
/// fingerprints in the main game executable (auto-selected as the largest
/// non-launcher `.exe`, or `exe_override` when given). `extra_binaries` adds
/// manifest-supplied `(needle, name)` anti-cheat signatures on top of the
/// compile-time baseline so a newly named engine is caught without an app
/// release (empty = baseline only). Results are name-deduped (binary scan wins
/// over PE inspect on collision) and sorted. The caller merges the named-dataset
/// layer on top.
pub fn detect_protections(
    install_dir: &Path,
    exe_override: Option<&Path>,
    max_depth: usize,
    extra_binaries: &[(String, String)],
) -> Vec<ProtectionHit> {
    let (mut hits, largest_exe) = binaries::scan(install_dir, max_depth, extra_binaries);
    let exe = exe_override.map(Path::to_path_buf).or(largest_exe);
    if let Some(exe) = exe {
        for hit in pe_inspect::inspect_pe(&exe) {
            if !hits.iter().any(|h| h.name == hit.name) {
                hits.push(hit);
            }
        }
    }
    hits.sort_by(|a, b| a.name.cmp(&b.name));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detect_protections_merges_binary_scan_and_pe_inspect() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("BEService.exe"), b"x").unwrap();
        let exe = dir.path().join("game.exe");
        let mut body = vec![0x90u8; 256];
        body.extend_from_slice(b"xx denuvo_atd xx");
        fs::write(&exe, &body).unwrap();

        let hits = detect_protections(dir.path(), Some(&exe), DEFAULT_SCAN_DEPTH, &[]);
        assert!(hits
            .iter()
            .any(|h| h.name == "BattlEye" && h.source == HitSource::Binary));
        assert!(hits
            .iter()
            .any(|h| h.name == "Denuvo Anti-Tamper" && h.source == HitSource::Pe));
    }

    #[test]
    fn detect_protections_clean_game_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("game.exe");
        fs::write(&exe, vec![0x90u8; 1024]).unwrap();
        assert!(detect_protections(dir.path(), Some(&exe), DEFAULT_SCAN_DEPTH, &[]).is_empty());
    }

    #[test]
    fn detect_protections_without_exe_still_scans_binaries() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("vgk.sys"), b"x").unwrap();
        let hits = detect_protections(dir.path(), None, DEFAULT_SCAN_DEPTH, &[]);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Riot Vanguard");
    }

    #[test]
    fn detect_protections_uses_manifest_extra_binary_signature() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("NewGuard64.sys"), b"x").unwrap();
        let exe = dir.path().join("game.exe");
        fs::write(&exe, vec![0x90u8; 1024]).unwrap();
        let baseline = detect_protections(dir.path(), Some(&exe), DEFAULT_SCAN_DEPTH, &[]);
        assert!(baseline.is_empty());

        let extra = vec![("newguard".to_string(), "New Guard AC".to_string())];
        let hits = detect_protections(dir.path(), Some(&exe), DEFAULT_SCAN_DEPTH, &extra);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "New Guard AC");
        assert_eq!(hits[0].source, HitSource::Binary);
    }
}
