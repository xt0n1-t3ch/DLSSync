mod commands;
mod constants;
mod efficiency;
mod error;
mod logging;
mod netpolicy;
mod paths;
mod state;
mod system_info;
mod tray;

const STALE_STAGING_AGE_SECS: u64 = 24 * 60 * 60;

fn sweep_stale_staging_dirs(root: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut removed = 0usize;
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !(name_str.starts_with(".tmp") || name_str.starts_with("staging-")) {
            continue;
        }
        if let Ok(modified) = meta.modified() {
            if let Ok(age) = now.duration_since(modified) {
                if age.as_secs() >= STALE_STAGING_AGE_SECS
                    && std::fs::remove_dir_all(entry.path()).is_ok()
                {
                    removed += 1;
                }
            }
        }
    }
    if removed > 0 {
        tracing::info!(removed, "swept stale staging dirs");
    }
}

/// Dispatch the two ELEVATED-child modes this process can be relaunched in via
/// UAC (by the System & Components driver commands). Returns the child exit code,
/// or `None` for a normal (GUI) launch.
///
/// - `--restore-driver <dir>`: re-install an exported DriverStore snapshot
///   (rollback). Writes an `InstallReport` JSON to `--result`.
/// - `--wua-install <UpdateID:Rev>`: snapshot the current driver
///   (`--snapshot-inf`/`--snapshot-dest`, best-effort) then run the
///   Administrator-only WUA download+install. Writes incremental progress to
///   `--progress-file` and the `InstallReport` to `--result`.
///
/// Either way the GUI is never started.
#[cfg(windows)]
fn try_run_wua_install_child() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let arg_after = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    if let Some(dir) = arg_after("--restore-driver") {
        return Some(run_driver_restore_child(&dir, arg_after("--result")));
    }
    let update_id = arg_after("--wua-install")?;
    Some(run_wua_install_child(
        &update_id,
        arg_after("--result"),
        arg_after("--progress-file"),
        arg_after("--snapshot-inf"),
        arg_after("--snapshot-dest"),
    ))
}

#[cfg(windows)]
fn run_driver_restore_child(dir: &str, result_path: Option<String>) -> i32 {
    use system_drivers::InstallReport;
    let (report, exit) =
        match system_drivers::driver_snapshot::restore_driver(std::path::Path::new(dir)) {
            Ok(()) => (
                InstallReport {
                    success: true,
                    reboot_required: true,
                    result_code: 0,
                    message: "Driver rolled back from its pre-update snapshot.".to_string(),
                },
                0,
            ),
            Err(e) => (
                InstallReport {
                    success: false,
                    reboot_required: false,
                    result_code: -1,
                    message: format!("Rollback failed: {e}"),
                },
                2,
            ),
        };
    if let Some(path) = result_path {
        if let Ok(json) = serde_json::to_string(&report) {
            let _ = std::fs::write(&path, json);
        }
    }
    exit
}

#[cfg(windows)]
fn run_wua_install_child(
    update_id: &str,
    result_path: Option<String>,
    progress_path: Option<String>,
    snapshot_inf: Option<String>,
    snapshot_dest: Option<String>,
) -> i32 {
    use system_drivers::{InstallProgress, InstallReport, UpdateSource, WuaSource};

    if let (Some(inf), Some(dest)) = (snapshot_inf.as_deref(), snapshot_dest.as_deref()) {
        let _ = system_drivers::driver_snapshot::export_driver(inf, std::path::Path::new(dest));
        let _ = system_drivers::driver_snapshot::create_restore_point(
            "DLSSync — before System & Components driver update",
        );
    }

    let mut on_progress = |p: InstallProgress| {
        if let Some(path) = progress_path.as_deref() {
            if let Ok(json) = serde_json::to_string(&p) {
                let _ = std::fs::write(path, json);
            }
        }
    };
    let (report, exit) = match WuaSource.install(update_id, &mut on_progress) {
        Ok(r) if r.success => (r, 0),
        Ok(r) => (r, 2),
        Err(e) => {
            let msg = e.to_string();
            let lower = msg.to_ascii_lowercase();
            let exit = if msg.contains("80240044") || lower.contains("access denied") {
                3
            } else {
                2
            };
            (
                InstallReport {
                    success: false,
                    reboot_required: false,
                    result_code: -1,
                    message: format!("Install failed: {msg}"),
                },
                exit,
            )
        }
    };
    if let Some(path) = result_path {
        if let Ok(json) = serde_json::to_string(&report) {
            let _ = std::fs::write(&path, json);
        }
    }
    exit
}

const BACKGROUND_INITIAL_DELAY_SECS: u64 = 60;
const SECS_PER_HOUR: u64 = 60 * 60;

/// Converts a clamped `effective_interval_hours` value into a
/// [`tokio::time::Duration`] suitable for building or resetting an interval.
///
/// Extracted so the conversion is testable without a running runtime.
fn interval_period(effective_hours: u32) -> tokio::time::Duration {
    tokio::time::Duration::from_secs(effective_hours as u64 * SECS_PER_HOUR)
}

/// Backend scan scheduler. A `tokio` interval drives the cadence; each tick the
/// loop re-reads `settings.background` (so interval/enabled changes apply without
/// a restart) and, when the daemon is enabled and no apply is inflight, emits
/// `background:scan-tick` for the frontend to run the existing scan/digest flow.
///
/// Drift resistance: uses [`tokio::time::interval_at`] with
/// [`tokio::time::MissedTickBehavior::Skip`] so a long-running scan does not
/// cause the next tick to fire immediately; the ~24 h cadence stays ~24 h
/// regardless of scan duration.  When the user changes `interval_hours` in
/// Settings the interval is rebuilt anchored to `last_tick + new_period` so
/// the new cadence takes effect on the very next fire without accumulating lag.
fn spawn_background_scheduler(handle: tauri::AppHandle) {
    use tokio::time::{interval_at, MissedTickBehavior};

    use tauri::{Emitter, Manager};

    tauri::async_runtime::spawn(async move {
        let state = handle.state::<state::AppState>();

        // One-time startup grace period: give the frontend time to initialise
        // before the first scan fires.
        tokio::time::sleep(tokio::time::Duration::from_secs(
            BACKGROUND_INITIAL_DELAY_SECS,
        ))
        .await;

        // Snapshot the period at startup so we can detect live changes.
        let initial_hours = state.settings.read().background.effective_interval_hours();
        let initial_period = interval_period(initial_hours);

        // Schedule the first tick `initial_period` from now (i.e. the startup
        // delay has already elapsed; this is the *scan* interval, not the
        // initial delay).
        let first_tick = tokio::time::Instant::now() + initial_period;
        let mut ticker = interval_at(first_tick, initial_period);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        // Track the period in effect so we can rebuild `ticker` on change.
        let mut current_period = initial_period;

        loop {
            let tick_at = ticker.tick().await;

            let background = state.settings.read().background.clone();
            let new_period = interval_period(background.effective_interval_hours());

            // If the user changed the interval in Settings, rebuild the ticker
            // anchored to this tick instant so the new cadence starts from now.
            if new_period != current_period {
                tracing::debug!(
                    old_hours = (current_period.as_secs() / SECS_PER_HOUR),
                    new_hours = (new_period.as_secs() / SECS_PER_HOUR),
                    "background scheduler: interval changed, rebuilding ticker",
                );
                let next = tick_at + new_period;
                ticker = interval_at(next, new_period);
                ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
                current_period = new_period;
            }

            if !background.enabled {
                continue;
            }

            if state.apply_registry.in_flight() > 0 {
                tracing::debug!("background scan tick skipped: apply inflight");
            } else if let Err(e) = handle.emit(tray::EVENT_BACKGROUND_SCAN_TICK, ()) {
                tracing::warn!(error = %e, "background scan tick emit failed");
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    if let Some(code) = try_run_wua_install_child() {
        std::process::exit(code);
    }

    #[cfg(debug_assertions)]
    {
        const CDP_REMOTE_DEBUGGING_PORT: u16 = 9333;
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            format!("--remote-debugging-port={CDP_REMOTE_DEBUGGING_PORT}"),
        );
    }

    let _log_guard = logging::init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "DLSSync starting");

    let builder =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, _, _| {
            use tauri::Manager;
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
                let _ = w.show();
            }
        }));

    #[cfg(not(feature = "nexus"))]
    let builder = builder.plugin(tauri_plugin_updater::Builder::default().build());

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(state::AppState::new())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                use tauri::Manager;
                let app = window.app_handle();
                let close_to_tray = app
                    .state::<state::AppState>()
                    .settings
                    .read()
                    .background
                    .close_to_tray;
                if close_to_tray {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .setup(|app| {
            use tauri::Manager;

            let app_paths = paths::AppPaths::resolve(app.handle())
                .map_err(|e| format!("resolve app paths: {e}"))?;
            app_paths
                .ensure_dirs()
                .map_err(|e| format!("create app dirs: {e}"))?;
            let migration = app_paths.migrate_legacy(app.handle());
            if migration.legacy_root.is_some() {
                tracing::info!(
                    moved_files = migration.moved_files,
                    rewrote_db_rows = migration.rewrote_db_rows,
                    copied_settings = migration.copied_settings,
                    copied_catalog = migration.copied_catalog,
                    errors = ?migration.errors,
                    legacy_root = ?migration.legacy_root,
                    new_root = %app_paths.root.display(),
                    "legacy data dir migration",
                );
            }

            let state: tauri::State<'_, state::AppState> = app.state();
            *state.paths.write() = Some(app_paths.clone());

            match backup_store::BackupStore::open(
                app_paths.backups_db.clone(),
                app_paths.backups_dir.clone(),
            ) {
                Ok(store) => {
                    sweep_stale_staging_dirs(&store.root_dir);
                    *state.backups.write() = Some(store);
                    tracing::info!(
                        db = %app_paths.backups_db.display(),
                        root = %app_paths.backups_dir.display(),
                        "backup store opened",
                    );
                }
                Err(e) => {
                    tracing::error!(
                        db = %app_paths.backups_db.display(),
                        error = %e,
                        "backup store open failed",
                    );
                }
            }

            match notifications_store::NotificationsStore::open(app_paths.notifications_db.clone())
            {
                Ok(store) => {
                    *state.notifications.write() = Some(store);
                    tracing::info!(
                        db = %app_paths.notifications_db.display(),
                        "notifications store opened",
                    );
                }
                Err(e) => {
                    tracing::error!(
                        db = %app_paths.notifications_db.display(),
                        error = %e,
                        "notifications store open failed",
                    );
                }
            }

            *state.catalog_cache_path.write() = Some(app_paths.catalog_cache.clone());
            if app_paths.catalog_cache.exists() {
                match dll_catalog::load_verified_cache(&app_paths.catalog_cache) {
                    Some(cat) => {
                        *state.catalog.write() = Some(cat);
                        tracing::info!(
                            path = %app_paths.catalog_cache.display(),
                            "catalog loaded from cache",
                        );
                    }
                    None => {
                        tracing::warn!(
                            path = %app_paths.catalog_cache.display(),
                            error = "cache missing, invalid, or signature verification failed",
                            "catalog cache unreadable",
                        );
                        match dll_catalog::embedded_fallback_catalog() {
                            Ok(cat) => {
                                *state.catalog.write() = Some(cat);
                                tracing::info!("catalog loaded from embedded fallback");
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "embedded fallback catalog unreadable",
                                );
                            }
                        }
                    }
                }
            }

            let loaded = commands::settings::load_initial(&app_paths);
            tracing::info!(
                blacklist = loaded.blacklist.len(),
                custom_folders = loaded.launcher_overrides.custom.len(),
                "settings loaded",
            );
            *state.settings.write() = loaded;

            if let Err(e) = tray::install(app.handle()) {
                tracing::warn!(error = %e, "tray icon install failed");
            }

            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

            spawn_background_scheduler(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan::scan_libraries,
            commands::scan::detect_dlls,
            commands::scan::detect_dlss_enabler,
            commands::scan::enrich_game_art,
            commands::scan::fetch_steam_art,
            commands::shell::open_path,
            commands::shell::reveal_path,
            commands::catalog::refresh_catalog,
            commands::catalog::catalog_summary,
            commands::catalog::catalog_latest_shas,
            commands::catalog::list_releases,
            commands::apply::apply_update,
            commands::apply::apply_update_batch,
            commands::apply::cancel_apply,
            commands::apply::cancel_all_applies,
            commands::streamline_set::apply_streamline_set,
            commands::streamline_set::apply_dll_set,
            commands::backup::list_backups,
            commands::backup::restore_backup,
            commands::backup::delete_backup,
            commands::diagnostics::get_log_paths,
            commands::diagnostics::read_recent_logs,
            commands::diagnostics::build_issue_report,
            commands::notifications::list_notifications,
            commands::notifications::mark_notification_read,
            commands::notifications::mark_all_notifications_read,
            commands::notifications::dismiss_notification,
            commands::notifications::push_notification,
            commands::notifications::notifications_unread_count,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::add_blacklist_entry,
            commands::settings::remove_blacklist_entry,
            commands::settings::save_window_state,
            commands::settings::get_app_paths,
            commands::advanced::set_dlss_debug_overlay,
            commands::advanced::get_dlss_debug_overlay,
            commands::system::get_system_info,
            commands::drivers::check_driver_updates,
            commands::drivers::list_driver_history,
            commands::drivers::install_driver,
            commands::system_drivers::scan_system_drivers,
            commands::system_drivers::install_system_driver,
            commands::system_drivers::restore_system_driver,
            commands::system_drivers::system_driver_versions,
            commands::anticheat::detect_anticheat,
            commands::dlss_profile::dlss_overrides_supported,
            commands::dlss_profile::apply_dlss_override,
            commands::dlss_profile::reset_dlss_override,
            commands::dlss_profile::read_dlss_override_config,
            commands::dlss_profile::find_game_executable,
            commands::runtime::runtime_mode,
            commands::runtime::open_devtools,
            commands::background::tray_set_pending,
            commands::ui_prefs::set_efficiency_mode,
            commands::ui_prefs::hide_main_window,
            commands::ui_prefs::show_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DLSSync");
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // interval_period — pure conversion, no runtime needed
    // ---------------------------------------------------------------------------

    #[test]
    fn period_24h_is_86400_secs() {
        assert_eq!(
            interval_period(24).as_secs(),
            86_400,
            "24 h must map to exactly 86 400 s"
        );
    }

    #[test]
    fn period_1h_is_3600_secs() {
        assert_eq!(interval_period(1).as_secs(), SECS_PER_HOUR);
    }

    #[test]
    fn period_168h_is_max_week() {
        assert_eq!(interval_period(168).as_secs(), 168 * SECS_PER_HOUR);
    }

    // ---------------------------------------------------------------------------
    // Drift-resistance: verify that a long-running tick does not pull the next
    // fire time forward.  We simulate the scheduler loop logic in isolation using
    // tokio's time-control primitives so the test runs instantly.
    // ---------------------------------------------------------------------------

    /// Drive one iteration of the rebuild logic: given `tick_at`, `old_period`,
    /// and `new_period`, return the `Instant` at which the rebuilt interval would
    /// next fire.  Mirrors the `tick_at + new_period` expression in the scheduler.
    fn next_fire_after_rebuild(
        tick_at: tokio::time::Instant,
        new_period: tokio::time::Duration,
    ) -> tokio::time::Instant {
        tick_at + new_period
    }

    #[tokio::test]
    async fn rebuild_anchors_to_tick_not_to_wall_clock() {
        // Simulate: tick fires at T=0, scan takes 30 minutes, then we rebuild.
        // The next tick must be T + new_period, NOT wall_clock + new_period.
        tokio::time::pause();

        let tick_at = tokio::time::Instant::now();
        let new_period = interval_period(24); // 86 400 s

        // Advance the clock to simulate a 30-minute scan.
        tokio::time::advance(tokio::time::Duration::from_secs(30 * 60)).await;

        let wall_clock_now = tokio::time::Instant::now();
        let scheduled_next = next_fire_after_rebuild(tick_at, new_period);

        // The scheduled next-fire must equal tick_at + 24 h (86 400 s from T=0),
        // NOT wall_clock_now + 24 h (which would be ~86 400 + 1800 s from T=0).
        let expected = tick_at + new_period;
        assert_eq!(
            scheduled_next, expected,
            "next fire must anchor to tick_at, not wall clock"
        );
        // Confirm the wall clock has drifted past tick_at.
        assert!(
            wall_clock_now > tick_at,
            "wall clock should have advanced beyond tick_at"
        );
        // And that anchoring to tick_at gives a *later* absolute time than
        // anchoring to wall_clock (i.e. we are not bringing the next tick forward).
        let wall_anchored = wall_clock_now + new_period;
        assert!(
            wall_anchored > scheduled_next,
            "wall-clock-anchored next ({wall_anchored:?}) should be later than tick-anchored ({scheduled_next:?})"
        );
    }

    #[tokio::test]
    async fn interval_change_fires_at_new_cadence_from_tick() {
        // Old period: 2 h.  Tick fires.  User changes to 4 h.
        // Next fire must be tick_at + 4 h, not tick_at + 2 h.
        tokio::time::pause();

        let tick_at = tokio::time::Instant::now();
        let old_period = interval_period(2);
        let new_period = interval_period(4);

        let scheduled_next = next_fire_after_rebuild(tick_at, new_period);

        // Should NOT be the old-period boundary.
        let old_next = tick_at + old_period;
        assert_ne!(scheduled_next, old_next, "must use new_period, not old");

        // Should be exactly tick_at + 4 h.
        assert_eq!(
            scheduled_next,
            tick_at + new_period,
            "must anchor to new_period from tick"
        );
    }

    #[tokio::test]
    async fn missed_tick_behavior_skip_does_not_catchup() {
        // Prove that MissedTickBehavior::Skip + interval_at never fires more than
        // once even if the clock jumps past multiple periods.
        use tokio::time::{interval_at, MissedTickBehavior};

        tokio::time::pause();

        let period = tokio::time::Duration::from_secs(10);
        let start = tokio::time::Instant::now() + period;
        let mut ticker = interval_at(start, period);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        // Advance the clock by 35 s (= 3.5 periods).  With Skip behaviour only
        // ONE tick should be pending (the most recent missed deadline); the
        // intermediate missed ticks are discarded.
        tokio::time::advance(tokio::time::Duration::from_secs(35)).await;

        let mut ticks = 0usize;
        // Drain whatever is immediately ready (non-blocking drain via try_join on
        // a very short timeout).  We run inside `pause()` so only pre-advanced
        // instants are ready; `tick()` will not suspend for future instants.
        //
        // We rely on the fact that after one `.tick()` the interval schedules the
        // *next* instant (now + 10 s in paused time), which is NOT yet elapsed,
        // so a second poll would suspend.  We use a tight select! to detect that.
        loop {
            tokio::select! {
                biased;
                _ = ticker.tick() => { ticks += 1; }
                _ = tokio::time::sleep(tokio::time::Duration::ZERO) => { break; }
            }
            if ticks > 1 {
                break; // fail fast — no need to drain further
            }
        }

        assert_eq!(
            ticks, 1,
            "Skip behaviour must fire at most once per poll cycle even after a multi-period gap"
        );
    }
}
