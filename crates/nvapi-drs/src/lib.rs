pub mod settings;

#[cfg(windows)]
pub mod ffi;

pub use settings::{
    DlssOverrideConfig, DlssPreset, DrsSetting, FrameGenCount, FrameGenMode, OverrideScope,
};
