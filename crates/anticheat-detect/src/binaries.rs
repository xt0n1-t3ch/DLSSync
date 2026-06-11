use crate::signatures::{classify, match_anti_cheat_binary_with, HitSource};
use crate::ProtectionHit;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Executables whose names mark them as launchers/installers/crash handlers,
/// not the protected game binary — skipped when picking the main executable.
const EXE_SKIP_TOKENS: &[&str] = &[
    "unins",
    "setup",
    "vcredist",
    "dxsetup",
    "crashpad",
    "crashreport",
    "crashhandler",
    "launcher",
    "redist",
    "dotnet",
    "ueprereqsetup",
    "easyanticheat",
    "battleye",
];

/// Single depth-limited walk that both matches anti-cheat binary filenames and
/// tracks the largest plausible game executable (the one most likely to carry a
/// packer/anti-tamper). `extra` adds manifest-supplied `(needle, name)`
/// signatures on top of the compile-time baseline (empty = baseline only).
/// Returns (anti-cheat hits, largest exe path).
pub fn scan(
    root: &Path,
    max_depth: usize,
    extra: &[(String, String)],
) -> (Vec<ProtectionHit>, Option<PathBuf>) {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut hits = Vec::new();
    let mut largest: Option<(u64, PathBuf)> = None;
    let walker = jwalk::WalkDir::new(root)
        .max_depth(max_depth)
        .skip_hidden(false);
    for entry in walker {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(anti_cheat) = match_anti_cheat_binary_with(&name, extra) {
            if seen.insert(anti_cheat.to_string()) {
                hits.push(ProtectionHit {
                    name: anti_cheat.to_string(),
                    kind: classify(anti_cheat),
                    source: HitSource::Binary,
                });
            }
        }
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".exe") && !EXE_SKIP_TOKENS.iter().any(|t| lower.contains(t)) {
            if let Ok(meta) = entry.metadata() {
                let size = meta.len();
                if largest.as_ref().is_none_or(|(s, _)| size > *s) {
                    largest = Some((size, entry.path()));
                }
            }
        }
    }
    hits.sort_by(|a, b| a.name.cmp(&b.name));
    (hits, largest.map(|(_, p)| p))
}

/// Anti-cheat binary scan only (no executable tracking) against the compile-time
/// baseline. Thin wrapper over [`scan`].
pub fn scan_dir(root: &Path, max_depth: usize) -> Vec<ProtectionHit> {
    scan(root, max_depth, &[]).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::ProtectionKind;
    use std::fs;

    #[test]
    fn scan_dir_finds_nested_anti_cheat_binaries_deduped_and_classified() {
        let dir = tempfile::tempdir().unwrap();
        let eac = dir.path().join("EasyAntiCheat");
        fs::create_dir_all(&eac).unwrap();
        fs::write(eac.join("EasyAntiCheat_x64.dll"), b"x").unwrap();
        fs::write(eac.join("EasyAntiCheat.exe"), b"x").unwrap();
        fs::write(dir.path().join("BEService.exe"), b"x").unwrap();
        fs::write(dir.path().join("game.exe"), b"x").unwrap();
        let hits = scan_dir(dir.path(), crate::DEFAULT_SCAN_DEPTH);
        let names: Vec<&str> = hits.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names, vec!["BattlEye", "Easy Anti-Cheat"]);
        assert!(hits.iter().all(|h| h.kind == ProtectionKind::AntiCheat));
        assert!(hits.iter().all(|h| h.source == HitSource::Binary));
    }

    #[test]
    fn scan_dir_returns_empty_for_clean_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("game.exe"), b"x").unwrap();
        assert!(scan_dir(dir.path(), crate::DEFAULT_SCAN_DEPTH).is_empty());
    }

    #[test]
    fn scan_dir_missing_path_returns_empty_without_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        assert!(scan_dir(&missing, crate::DEFAULT_SCAN_DEPTH).is_empty());
    }

    #[test]
    fn scan_dir_respects_depth_limit() {
        let dir = tempfile::tempdir().unwrap();
        let deep = dir.path().join("a").join("b").join("c").join("d").join("e");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("vgk.sys"), b"x").unwrap();
        assert!(scan_dir(dir.path(), 2).is_empty());
        assert_eq!(scan_dir(dir.path(), 8).len(), 1);
    }
}
