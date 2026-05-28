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
const PRESET_RECOMMENDED: u32 = 0x00FF_FFFE;
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
    J,
    K,
    L,
    M,
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
            DlssPreset::J => 0x0A,
            DlssPreset::K => 0x0B,
            DlssPreset::L => 0x0C,
            DlssPreset::M => 0x0D,
            DlssPreset::Recommended => PRESET_RECOMMENDED,
        }
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
        if let Some(target_fps) = self.fg_dynamic_target_fps {
            settings.push(DrsSetting {
                id: ids::DLSS_MFG_TARGET_FRAME_RATE,
                value: target_fps,
            });
        }
        settings
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
        assert_eq!(DlssPreset::Recommended.to_value(), 0x00FF_FFFE);
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
}
