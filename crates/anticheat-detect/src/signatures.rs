use serde::{Deserialize, Serialize};

/// Risk category a detected protection falls into. Drives the warning copy and
/// tone: anti-cheat carries account-ban risk, anti-tamper carries launch-fail
/// risk, store DRM is informational.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionKind {
    AntiCheat,
    AntiTamper,
    Drm,
}

/// Where a detection came from: a matched binary filename on disk, the parsed
/// PE structure of the game executable, or the bundled/manifest dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitSource {
    Binary,
    Pe,
    Dataset,
}

/// Anti-cheat engine binaries by lowercase filename substring. A match on disk
/// is strong evidence the engine ships with the game.
pub const ANTI_CHEAT_BINARIES: &[(&str, &str)] = &[
    ("easyanticheat", "Easy Anti-Cheat"),
    ("eac_launcher", "Easy Anti-Cheat"),
    ("beservice", "BattlEye"),
    ("beclient", "BattlEye"),
    ("battleye", "BattlEye"),
    ("vgk.sys", "Riot Vanguard"),
    ("vgc.exe", "Riot Vanguard"),
    ("vgtray", "Riot Vanguard"),
    ("gamemon.des", "nProtect GameGuard"),
    ("npggnt.des", "nProtect GameGuard"),
    ("gameguard", "nProtect GameGuard"),
    ("xhunter1.sys", "XIGNCODE3"),
    ("x3.xem", "XIGNCODE3"),
    ("xigncode", "XIGNCODE3"),
    ("eaanticheat", "EA anticheat"),
    ("pnkbstr", "PunkBuster"),
    ("mhyprot", "miHoYo Protect"),
    ("ace-base", "Anti-Cheat Expert"),
    ("anticheatexpert", "Anti-Cheat Expert"),
    ("hshield", "HackShield"),
    ("nexonanalytics", "Nexon Game Security"),
    ("denuvo_anti_cheat", "Denuvo Anti-Cheat"),
    ("ricochet", "Ricochet Anti-Cheat"),
    ("equ8", "EQU8"),
];

/// PE section names that fingerprint a packer/protector, with the protection
/// name and its risk category. Matched case-insensitively against section names
/// of the game executable.
pub const PROTECTOR_SECTIONS: &[(&str, &str, ProtectionKind)] = &[
    (".vmp0", "VMProtect", ProtectionKind::AntiTamper),
    (".vmp1", "VMProtect", ProtectionKind::AntiTamper),
    (".vmp2", "VMProtect", ProtectionKind::AntiTamper),
    (
        ".themida",
        "Themida / WinLicense",
        ProtectionKind::AntiTamper,
    ),
    (
        ".winlice",
        "Themida / WinLicense",
        ProtectionKind::AntiTamper,
    ),
    ("enigma1", "Enigma Protector", ProtectionKind::AntiTamper),
    ("enigma2", "Enigma Protector", ProtectionKind::AntiTamper),
    (".enigma1", "Enigma Protector", ProtectionKind::AntiTamper),
    (".enigma2", "Enigma Protector", ProtectionKind::AntiTamper),
    (".bind", "Steam CEG DRM", ProtectionKind::Drm),
];

/// ASCII byte markers searched across the whole executable. Denuvo v5.0+ embeds
/// `denuvo_atd`; recent custom builds (e.g. Assassin's Creed Shadows, 2025) drop
/// that token but still carry a bare `Denuvo` string, so all three cases are
/// matched. Earlier/stripped builds carry no marker and are caught by the
/// section fingerprint below.
pub const STRING_MARKERS: &[(&[u8], &str, ProtectionKind)] = &[
    (
        b"denuvo_atd",
        "Denuvo Anti-Tamper",
        ProtectionKind::AntiTamper,
    ),
    (b"Denuvo", "Denuvo Anti-Tamper", ProtectionKind::AntiTamper),
    (b"DENUVO", "Denuvo Anti-Tamper", ProtectionKind::AntiTamper),
    (b"denuvo", "Denuvo Anti-Tamper", ProtectionKind::AntiTamper),
    (b"VMProtect", "VMProtect", ProtectionKind::AntiTamper),
];

/// Denuvo restructures the PE section table into a distinctive cluster of short,
/// non-standard section names. Several together is a strong Denuvo fingerprint
/// even when the name string is stripped — none appear in normally-compiled
/// MSVC/Clang binaries. (`.00cfg`, `.text`, `.data`, `.rdata`, `.pdata`,
/// `.rsrc`, `.reloc`, `.tls` are normal and excluded.)
pub const DENUVO_SECTION_CLUSTER: &[&str] = &[
    ".arch", ".shared", ".data1", ".data2", ".sxdata", ".xtext", ".xtls", ".nidata", ".didata",
    ".trace", ".srdata", ".text1",
];

/// Minimum number of [`DENUVO_SECTION_CLUSTER`] sections present to flag Denuvo
/// by section fingerprint alone.
pub const DENUVO_SECTION_CLUSTER_MIN: usize = 4;

/// Entropy at or above this (bits/byte, 0-8 scale) marks a section as packed or
/// encrypted. Normal compiled code sits in 4.0-6.5.
pub const PACKED_ENTROPY_THRESHOLD: f64 = 7.5;

/// A normal compiled PE has well under this many sections. Denuvo and friends
/// explode the section table; combined with high entropy this is a strong
/// heuristic signal for an unnamed protector.
pub const PROTECTOR_SECTION_COUNT_HINT: usize = 14;

/// Classify a protection name into its risk category. One sink so the dataset,
/// the binary scan, and the PE inspector all agree.
pub fn classify(name: &str) -> ProtectionKind {
    let lower = name.to_ascii_lowercase();
    if lower.contains("anti-tamper")
        || lower.contains("antitamper")
        || lower.contains("denuvo anti-tamper")
        || lower.contains("vmprotect")
        || lower.contains("themida")
        || lower.contains("winlicense")
        || lower.contains("enigma")
        || lower.contains("arxan")
        || lower.contains("safedisc")
        || lower.contains("securom")
        || lower.contains("starforce")
    {
        return ProtectionKind::AntiTamper;
    }
    if lower.contains("connect")
        || lower.contains("ea app")
        || lower.contains("ea play")
        || lower.contains("origin")
        || lower.contains("steam ceg")
        || lower.contains("ceg")
        || lower.contains("rockstar")
        || lower.contains("epic games")
        || lower.contains("microsoft store")
        || lower.contains("gfwl")
    {
        return ProtectionKind::Drm;
    }
    ProtectionKind::AntiCheat
}

/// Match a filename against the anti-cheat binary table.
pub fn match_anti_cheat_binary(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    ANTI_CHEAT_BINARIES
        .iter()
        .find(|(needle, _)| lower.contains(needle))
        .map(|(_, ac)| *ac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_routes_each_protection_to_its_kind() {
        assert_eq!(classify("Denuvo Anti-Tamper"), ProtectionKind::AntiTamper);
        assert_eq!(classify("Arxan Anti-Tamper"), ProtectionKind::AntiTamper);
        assert_eq!(classify("VMProtect"), ProtectionKind::AntiTamper);
        assert_eq!(classify("Easy Anti-Cheat"), ProtectionKind::AntiCheat);
        assert_eq!(classify("BattlEye"), ProtectionKind::AntiCheat);
        assert_eq!(classify("Denuvo Anti-Cheat"), ProtectionKind::AntiCheat);
        assert_eq!(classify("Ubisoft Connect"), ProtectionKind::Drm);
        assert_eq!(classify("EA app"), ProtectionKind::Drm);
        assert_eq!(classify("Steam CEG DRM"), ProtectionKind::Drm);
    }

    #[test]
    fn match_anti_cheat_binary_is_case_insensitive() {
        assert_eq!(
            match_anti_cheat_binary("EasyAntiCheat_x64.dll"),
            Some("Easy Anti-Cheat")
        );
        assert_eq!(
            match_anti_cheat_binary("BEService_x64.exe"),
            Some("BattlEye")
        );
        assert_eq!(match_anti_cheat_binary("vgk.sys"), Some("Riot Vanguard"));
        assert_eq!(
            match_anti_cheat_binary("EAAntiCheat.GameService.exe"),
            Some("EA anticheat")
        );
        assert_eq!(match_anti_cheat_binary("nvngx_dlss.dll"), None);
    }

    #[test]
    fn match_anti_cheat_binary_does_not_false_positive_on_system_files() {
        assert_eq!(match_anti_cheat_binary("10eaac7.msi"), None);
        assert_eq!(
            match_anti_cheat_binary("022eaacc0a939e7ce75ee007f09946ec.ni.dll"),
            None
        );
        assert_eq!(match_anti_cheat_binary("la57setup.exe"), None);
        assert_eq!(match_anti_cheat_binary("comres.dll.mui"), None);
    }
}
