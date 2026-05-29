use serde::{Deserialize, Serialize};

pub mod ids {
    pub const DLSS_SR_ENABLE_OVERRIDE: u32 = 0x10E4_1E01;
    pub const DLSS_RR_ENABLE_OVERRIDE: u32 = 0x10E4_1E02;
    pub const DLSS_FG_ENABLE_OVERRIDE: u32 = 0x10E4_1E03;
    pub const DLSS_SR_FORCED_PRESET: u32 = 0x10E4_1DF3;
    pub const DLSS_FG_FORCED_PRESET: u32 = 0x10E4_1DF1;
    pub const DLSS_FG_FORCED_MODE: u32 = 0x1030_8298;
    pub const DLSS_MFG_FIXED_COUNT: u32 = 0x104D_6667;
    pub const DLSS_MFG_TARGET_FRAME_RATE: u32 = 0x10CF_4125;
}

const VALUE_ON: u32 = 0x0000_0001;
const PRESET_RECOMMENDED: u32 = 0x00FF_FFFF;
const PRESET_RECOMMENDED_LEGACY: u32 = 0x00FF_FFFE;
const FRAME_GEN_MODE_FIXED: u32 = 0x0000_0002;
const FRAME_GEN_MODE_DYNAMIC: u32 = 0x0000_0004;

pub const RESETTABLE_IDS: &[u32] = &[
    ids::DLSS_SR_ENABLE_OVERRIDE,
    ids::DLSS_RR_ENABLE_OVERRIDE,
    ids::DLSS_FG_ENABLE_OVERRIDE,
    ids::DLSS_SR_FORCED_PRESET,
    ids::DLSS_FG_FORCED_PRESET,
    ids::DLSS_FG_FORCED_MODE,
    ids::DLSS_MFG_FIXED_COUNT,
    ids::DLSS_MFG_TARGET_FRAME_RATE,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DlssPreset {
    Default,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    Recommended,
}

impl DlssPreset {
    pub fn to_value(self) -> u32 {
        match self {
            DlssPreset::Default => 0,
            DlssPreset::A => 1,
            DlssPreset::B => 2,
            DlssPreset::C => 3,
            DlssPreset::D => 4,
            DlssPreset::E => 5,
            DlssPreset::F => 6,
            DlssPreset::G => 7,
            DlssPreset::H => 8,
            DlssPreset::I => 9,
            DlssPreset::J => 0x0A,
            DlssPreset::K => 0x0B,
            DlssPreset::L => 0x0C,
            DlssPreset::M => 0x0D,
            DlssPreset::N => 0x0E,
            DlssPreset::O => 0x0F,
            DlssPreset::Recommended => PRESET_RECOMMENDED,
        }
    }

    pub fn from_value(value: u32) -> Option<Self> {
        Some(match value {
            0 => DlssPreset::Default,
            1 => DlssPreset::A,
            2 => DlssPreset::B,
            3 => DlssPreset::C,
            4 => DlssPreset::D,
            5 => DlssPreset::E,
            6 => DlssPreset::F,
            7 => DlssPreset::G,
            8 => DlssPreset::H,
            9 => DlssPreset::I,
            0x0A => DlssPreset::J,
            0x0B => DlssPreset::K,
            0x0C => DlssPreset::L,
            0x0D => DlssPreset::M,
            0x0E => DlssPreset::N,
            0x0F => DlssPreset::O,
            PRESET_RECOMMENDED | PRESET_RECOMMENDED_LEGACY => DlssPreset::Recommended,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameGenMode {
    AppControlled,
    Fixed,
    Dynamic,
}

impl FrameGenMode {
    pub fn to_value(self) -> u32 {
        match self {
            FrameGenMode::AppControlled => 0,
            FrameGenMode::Fixed => FRAME_GEN_MODE_FIXED,
            FrameGenMode::Dynamic => FRAME_GEN_MODE_DYNAMIC,
        }
    }

    pub fn from_value(value: u32) -> Option<Self> {
        Some(match value {
            0 => FrameGenMode::AppControlled,
            FRAME_GEN_MODE_FIXED => FrameGenMode::Fixed,
            FRAME_GEN_MODE_DYNAMIC => FrameGenMode::Dynamic,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameGenCount {
    AppControlled,
    X2,
    X3,
    X4,
}

impl FrameGenCount {
    pub fn to_value(self) -> u32 {
        match self {
            FrameGenCount::AppControlled => 0,
            FrameGenCount::X2 => 1,
            FrameGenCount::X3 => 2,
            FrameGenCount::X4 => 3,
        }
    }

    pub fn from_value(value: u32) -> Option<Self> {
        Some(match value {
            0 => FrameGenCount::AppControlled,
            1 => FrameGenCount::X2,
            2 => FrameGenCount::X3,
            3 => FrameGenCount::X4,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrsSetting {
    pub id: u32,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "scope")]
pub enum OverrideScope {
    Global,
    PerGame { executable_path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DlssOverrideConfig {
    pub enable_sr_dll_override: bool,
    pub sr_preset: Option<DlssPreset>,
    pub enable_fg_dll_override: bool,
    pub fg_preset: Option<DlssPreset>,
    pub fg_mode: Option<FrameGenMode>,
    pub fg_fixed_count: Option<FrameGenCount>,
    pub fg_dynamic_target_fps: Option<u32>,
}

impl DlssOverrideConfig {
    pub fn to_drs_settings(&self) -> Vec<DrsSetting> {
        let mut settings = Vec::new();
        if self.enable_sr_dll_override {
            settings.push(DrsSetting {
                id: ids::DLSS_SR_ENABLE_OVERRIDE,
                value: VALUE_ON,
            });
        }
        if let Some(preset) = self.sr_preset {
            settings.push(DrsSetting {
                id: ids::DLSS_SR_FORCED_PRESET,
                value: preset.to_value(),
            });
        }
        if self.enable_fg_dll_override {
            settings.push(DrsSetting {
                id: ids::DLSS_FG_ENABLE_OVERRIDE,
                value: VALUE_ON,
            });
        }
        if let Some(preset) = self.fg_preset {
            settings.push(DrsSetting {
                id: ids::DLSS_FG_FORCED_PRESET,
                value: preset.to_value(),
            });
        }
        if let Some(mode) = self.fg_mode {
            settings.push(DrsSetting {
                id: ids::DLSS_FG_FORCED_MODE,
                value: mode.to_value(),
            });
        }
        if let Some(count) = self.fg_fixed_count {
            settings.push(DrsSetting {
                id: ids::DLSS_MFG_FIXED_COUNT,
                value: count.to_value(),
            });
        }
        if let Some(target_fps) = self.fg_dynamic_target_fps.filter(|&fps| fps != 0) {
            settings.push(DrsSetting {
                id: ids::DLSS_MFG_TARGET_FRAME_RATE,
                value: target_fps,
            });
        }
        settings
    }

    pub fn from_drs_settings(settings: &[(u32, Option<u32>)]) -> Self {
        let get = |id: u32| -> Option<u32> {
            settings
                .iter()
                .find_map(|&(i, v)| (i == id).then_some(v))
                .flatten()
        };
        DlssOverrideConfig {
            enable_sr_dll_override: get(ids::DLSS_SR_ENABLE_OVERRIDE) == Some(VALUE_ON),
            sr_preset: get(ids::DLSS_SR_FORCED_PRESET).and_then(DlssPreset::from_value),
            enable_fg_dll_override: get(ids::DLSS_FG_ENABLE_OVERRIDE) == Some(VALUE_ON),
            fg_preset: get(ids::DLSS_FG_FORCED_PRESET).and_then(DlssPreset::from_value),
            fg_mode: get(ids::DLSS_FG_FORCED_MODE).and_then(FrameGenMode::from_value),
            fg_fixed_count: get(ids::DLSS_MFG_FIXED_COUNT).and_then(FrameGenCount::from_value),
            fg_dynamic_target_fps: get(ids::DLSS_MFG_TARGET_FRAME_RATE).filter(|&fps| fps != 0),
        }
    }

    pub fn active_override_count(&self) -> usize {
        let mut count = 0;
        if self.enable_sr_dll_override {
            count += 1;
        }
        if self.enable_fg_dll_override {
            count += 1;
        }
        if matches!(self.sr_preset, Some(p) if p != DlssPreset::Default) {
            count += 1;
        }
        if matches!(self.fg_preset, Some(p) if p != DlssPreset::Default) {
            count += 1;
        }
        if matches!(self.fg_mode, Some(m) if m != FrameGenMode::AppControlled) {
            count += 1;
        }
        if matches!(self.fg_fixed_count, Some(c) if c != FrameGenCount::AppControlled) {
            count += 1;
        }
        if matches!(self.fg_dynamic_target_fps, Some(fps) if fps > 0) {
            count += 1;
        }
        count
    }

    pub fn is_empty(&self) -> bool {
        self.active_override_count() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_letters_map_to_expected_drs_values() {
        assert_eq!(DlssPreset::A.to_value(), 1);
        assert_eq!(DlssPreset::J.to_value(), 0x0A);
        assert_eq!(DlssPreset::K.to_value(), 0x0B);
        assert_eq!(DlssPreset::M.to_value(), 0x0D);
        assert_eq!(DlssPreset::Recommended.to_value(), 0x00FF_FFFF);
        assert_eq!(DlssPreset::Default.to_value(), 0);
    }

    #[test]
    fn frame_gen_mode_uses_verified_fixed_and_dynamic_values() {
        assert_eq!(FrameGenMode::Fixed.to_value(), 2);
        assert_eq!(FrameGenMode::Dynamic.to_value(), 4);
        assert_eq!(FrameGenMode::AppControlled.to_value(), 0);
    }

    #[test]
    fn frame_gen_count_maps_multiplier_to_index() {
        assert_eq!(FrameGenCount::X2.to_value(), 1);
        assert_eq!(FrameGenCount::X3.to_value(), 2);
        assert_eq!(FrameGenCount::X4.to_value(), 3);
        assert_eq!(FrameGenCount::AppControlled.to_value(), 0);
    }

    #[test]
    fn empty_config_emits_no_settings() {
        assert!(DlssOverrideConfig::default().to_drs_settings().is_empty());
    }

    #[test]
    fn dynamic_frame_gen_config_emits_mode_and_target() {
        let config = DlssOverrideConfig {
            enable_fg_dll_override: true,
            fg_mode: Some(FrameGenMode::Dynamic),
            fg_dynamic_target_fps: Some(240),
            ..Default::default()
        };
        let settings = config.to_drs_settings();
        assert!(settings.contains(&DrsSetting {
            id: ids::DLSS_FG_ENABLE_OVERRIDE,
            value: VALUE_ON
        }));
        assert!(settings.contains(&DrsSetting {
            id: ids::DLSS_FG_FORCED_MODE,
            value: FRAME_GEN_MODE_DYNAMIC
        }));
        assert!(settings.contains(&DrsSetting {
            id: ids::DLSS_MFG_TARGET_FRAME_RATE,
            value: 240
        }));
    }

    #[test]
    fn fixed_frame_gen_with_preset_emits_each_set_field_once() {
        let config = DlssOverrideConfig {
            enable_sr_dll_override: true,
            sr_preset: Some(DlssPreset::K),
            fg_mode: Some(FrameGenMode::Fixed),
            fg_fixed_count: Some(FrameGenCount::X4),
            ..Default::default()
        };
        let settings = config.to_drs_settings();
        assert_eq!(settings.len(), 4);
        assert!(settings.contains(&DrsSetting {
            id: ids::DLSS_SR_FORCED_PRESET,
            value: 0x0B
        }));
        assert!(settings.contains(&DrsSetting {
            id: ids::DLSS_MFG_FIXED_COUNT,
            value: 3
        }));
    }

    #[test]
    fn resettable_ids_cover_every_override_setting() {
        assert_eq!(RESETTABLE_IDS.len(), 8);
        assert!(RESETTABLE_IDS.contains(&ids::DLSS_FG_FORCED_MODE));
    }

    #[test]
    fn preset_from_value_inverts_to_value() {
        for &preset in &[
            DlssPreset::Default,
            DlssPreset::A,
            DlssPreset::B,
            DlssPreset::C,
            DlssPreset::D,
            DlssPreset::E,
            DlssPreset::F,
            DlssPreset::G,
            DlssPreset::H,
            DlssPreset::I,
            DlssPreset::J,
            DlssPreset::K,
            DlssPreset::L,
            DlssPreset::M,
            DlssPreset::N,
            DlssPreset::O,
            DlssPreset::Recommended,
        ] {
            assert_eq!(DlssPreset::from_value(preset.to_value()), Some(preset));
        }
    }

    #[test]
    fn preset_from_value_accepts_both_recommended_sentinels() {
        assert_eq!(
            DlssPreset::from_value(0x00FF_FFFE),
            Some(DlssPreset::Recommended)
        );
        assert_eq!(
            DlssPreset::from_value(0x00FF_FFFF),
            Some(DlssPreset::Recommended)
        );
    }

    #[test]
    fn preset_from_value_covers_full_a_to_o_range_and_rejects_garbage() {
        assert_eq!(DlssPreset::from_value(0x09), Some(DlssPreset::I));
        assert_eq!(DlssPreset::from_value(0x0E), Some(DlssPreset::N));
        assert_eq!(DlssPreset::from_value(0x0F), Some(DlssPreset::O));
        assert_eq!(DlssPreset::from_value(0x10), None);
        assert_eq!(DlssPreset::from_value(0xDEAD), None);
    }

    #[test]
    fn frame_gen_mode_and_count_from_value_invert() {
        for &mode in &[
            FrameGenMode::AppControlled,
            FrameGenMode::Fixed,
            FrameGenMode::Dynamic,
        ] {
            assert_eq!(FrameGenMode::from_value(mode.to_value()), Some(mode));
        }
        assert_eq!(FrameGenMode::from_value(3), None);
        for &count in &[
            FrameGenCount::AppControlled,
            FrameGenCount::X2,
            FrameGenCount::X3,
            FrameGenCount::X4,
        ] {
            assert_eq!(FrameGenCount::from_value(count.to_value()), Some(count));
        }
        assert_eq!(FrameGenCount::from_value(9), None);
    }

    fn emulate_read(written: &[DrsSetting]) -> Vec<(u32, Option<u32>)> {
        RESETTABLE_IDS
            .iter()
            .map(|&id| (id, written.iter().find(|s| s.id == id).map(|s| s.value)))
            .collect()
    }

    #[test]
    fn config_round_trips_through_drs_settings() {
        let cfg = DlssOverrideConfig {
            enable_sr_dll_override: true,
            sr_preset: Some(DlssPreset::K),
            enable_fg_dll_override: true,
            fg_preset: Some(DlssPreset::J),
            fg_mode: Some(FrameGenMode::Dynamic),
            fg_fixed_count: Some(FrameGenCount::X4),
            fg_dynamic_target_fps: Some(240),
        };
        let read = emulate_read(&cfg.to_drs_settings());
        assert_eq!(DlssOverrideConfig::from_drs_settings(&read), cfg);
    }

    #[test]
    fn from_drs_settings_decodes_a_globally_set_preset() {
        let read = vec![(ids::DLSS_SR_FORCED_PRESET, Some(DlssPreset::K.to_value()))];
        let cfg = DlssOverrideConfig::from_drs_settings(&read);
        assert_eq!(cfg.sr_preset, Some(DlssPreset::K));
        assert_eq!(cfg.active_override_count(), 1);
        assert!(!cfg.is_empty());
    }

    #[test]
    fn empty_read_reconstructs_the_default_inactive_config() {
        let read: Vec<(u32, Option<u32>)> = RESETTABLE_IDS.iter().map(|&id| (id, None)).collect();
        let cfg = DlssOverrideConfig::from_drs_settings(&read);
        assert_eq!(cfg, DlssOverrideConfig::default());
        assert!(cfg.is_empty());
        assert_eq!(cfg.active_override_count(), 0);
    }

    #[test]
    fn unknown_drs_value_degrades_to_none_not_a_wrong_preset() {
        let read = vec![(ids::DLSS_SR_FORCED_PRESET, Some(0x10))];
        assert_eq!(DlssOverrideConfig::from_drs_settings(&read).sr_preset, None);
    }

    #[test]
    fn zero_target_fps_is_treated_as_unset_both_ways() {
        let cfg = DlssOverrideConfig {
            fg_dynamic_target_fps: Some(0),
            ..Default::default()
        };
        assert!(cfg.is_empty());
        assert!(
            cfg.to_drs_settings().is_empty(),
            "a 0 fps target is not a real override and must not be written"
        );
        let read = emulate_read(&cfg.to_drs_settings());
        assert_eq!(
            DlssOverrideConfig::from_drs_settings(&read).fg_dynamic_target_fps,
            None
        );
    }
}
