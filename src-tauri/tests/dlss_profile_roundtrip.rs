#![cfg(target_os = "windows")]

use nvapi_drs::ffi::{apply_overrides, read_overrides, reset_overrides};
use nvapi_drs::settings::RESETTABLE_IDS;
use nvapi_drs::{
    DlssOverrideConfig, DlssPreset, DrsSetting, FrameGenCount, FrameGenMode, OverrideScope,
};

struct ResetGuard {
    scope: OverrideScope,
}

impl Drop for ResetGuard {
    fn drop(&mut self) {
        let _ = reset_overrides(&self.scope, RESETTABLE_IDS);
    }
}

#[test]
#[ignore = "destructive: mutates the NVIDIA driver base profile; run with --ignored on a Windows host with an NVIDIA driver"]
fn dlss_overrides_apply_read_reset_round_trip_on_global_profile() {
    let scope = OverrideScope::Global;
    let _guard = ResetGuard {
        scope: scope.clone(),
    };

    let cfg = DlssOverrideConfig {
        enable_sr_dll_override: true,
        sr_preset: Some(DlssPreset::K),
        enable_fg_dll_override: false,
        fg_preset: None,
        fg_mode: Some(FrameGenMode::Fixed),
        fg_fixed_count: Some(FrameGenCount::X2),
        fg_dynamic_target_fps: None,
    };
    let settings: Vec<DrsSetting> = cfg.to_drs_settings();
    assert!(
        !settings.is_empty(),
        "config should yield at least one DRS write"
    );

    apply_overrides(&scope, &settings)
        .expect("apply_overrides should succeed against the live NVIDIA driver");

    let written_ids: Vec<u32> = settings.iter().map(|s| s.id).collect();
    let read_back = read_overrides(&scope, &written_ids).expect("read_overrides should succeed");

    for s in &settings {
        let (_, value) = read_back
            .iter()
            .find(|(id, _)| *id == s.id)
            .copied()
            .unwrap_or_else(|| panic!("read should return entry for DRS id {:#x}", s.id));
        assert_eq!(
            value,
            Some(s.value),
            "DRS DWORD {:#x} should read back as {}",
            s.id,
            s.value,
        );
    }

    reset_overrides(&scope, RESETTABLE_IDS).expect("reset_overrides should succeed");

    let after_reset =
        read_overrides(&scope, &written_ids).expect("read after reset should succeed");
    for (id, value) in after_reset {
        assert!(
            value.is_none() || value == Some(0),
            "DRS DWORD {:#x} should be cleared after reset, got {:?}",
            id,
            value,
        );
    }
}
