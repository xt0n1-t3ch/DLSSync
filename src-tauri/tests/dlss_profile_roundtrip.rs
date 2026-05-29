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

    let denied = apply_overrides(&scope, &settings)
        .expect("apply_overrides should not abort against the live NVIDIA driver");

    let written_ids: Vec<u32> = settings.iter().map(|s| s.id).collect();
    let read_back = read_overrides(&scope, &written_ids).expect("read_overrides should succeed");

    for s in &settings {
        if denied.contains(&s.id) {
            continue;
        }
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

    let reconstructed = DlssOverrideConfig::from_drs_settings(
        &read_overrides(&scope, RESETTABLE_IDS).expect("read_overrides should succeed"),
    );
    if denied.is_empty() {
        assert_eq!(
            reconstructed, cfg,
            "elevated: the read path must reconstruct the exact applied config (forum bug #1)"
        );
    } else {
        assert_eq!(
            reconstructed.sr_preset, cfg.sr_preset,
            "SR preset must apply + reconstruct even when FG writes are privilege-denied"
        );
        assert!(reconstructed.enable_sr_dll_override);
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

    assert!(
        DlssOverrideConfig::from_drs_settings(
            &read_overrides(&scope, RESETTABLE_IDS).expect("read after reset should succeed")
        )
        .is_empty(),
        "a reset profile must read back as an empty (inactive) config"
    );
}

#[test]
#[ignore = "destructive: writes a synthetic per-game NVIDIA profile (reset after); run with --ignored on a Windows host with an NVIDIA driver"]
fn recommended_sentinel_round_trips_on_a_synthetic_per_game_profile() {
    let exe = std::env::temp_dir().join("dlssync_e2e_selftest.exe");
    let scope = OverrideScope::PerGame {
        executable_path: exe.to_string_lossy().into_owned(),
    };
    let _guard = ResetGuard {
        scope: scope.clone(),
    };

    let cfg = DlssOverrideConfig {
        enable_sr_dll_override: true,
        sr_preset: Some(DlssPreset::Recommended),
        enable_fg_dll_override: false,
        fg_preset: None,
        fg_mode: None,
        fg_fixed_count: None,
        fg_dynamic_target_fps: None,
    };
    let denied = apply_overrides(&scope, &cfg.to_drs_settings())
        .expect("apply_overrides should succeed against the live NVIDIA driver");
    assert!(
        denied.is_empty(),
        "SR-only overrides must apply without elevation; denied = {denied:?}"
    );

    let reconstructed = DlssOverrideConfig::from_drs_settings(
        &read_overrides(&scope, RESETTABLE_IDS).expect("read_overrides should succeed"),
    );
    assert_eq!(
        reconstructed, cfg,
        "Recommended must round-trip as Recommended via 0x00FF_FFFF on the live driver"
    );

    reset_overrides(&scope, RESETTABLE_IDS).expect("reset_overrides should succeed");
    assert!(
        DlssOverrideConfig::from_drs_settings(
            &read_overrides(&scope, RESETTABLE_IDS).expect("read after reset should succeed")
        )
        .is_empty(),
        "a reset profile must read back as empty"
    );
}

#[test]
#[ignore = "destructive: writes a synthetic per-game NVIDIA profile (reset after); run with --ignored on a Windows host with an NVIDIA driver"]
fn apply_is_resilient_to_privilege_gated_frame_gen_settings() {
    let fg_mode_id = nvapi_drs::settings::ids::DLSS_FG_FORCED_MODE;
    let exe = std::env::temp_dir().join("dlssync_e2e_selftest.exe");
    let scope = OverrideScope::PerGame {
        executable_path: exe.to_string_lossy().into_owned(),
    };
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
    let denied = apply_overrides(&scope, &cfg.to_drs_settings())
        .expect("apply must not abort even when a setting is privilege-denied");

    let read = DlssOverrideConfig::from_drs_settings(
        &read_overrides(&scope, RESETTABLE_IDS).expect("read_overrides should succeed"),
    );
    assert_eq!(read.sr_preset, Some(DlssPreset::K));
    assert!(read.enable_sr_dll_override);
    if denied.is_empty() {
        assert_eq!(read.fg_mode, Some(FrameGenMode::Fixed));
    } else {
        assert!(denied.contains(&fg_mode_id));
        assert_eq!(
            read.fg_mode, None,
            "a privilege-denied FG setting must not be written"
        );
    }
}
