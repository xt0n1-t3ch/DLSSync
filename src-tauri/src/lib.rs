mod commands;
mod constants;
mod efficiency;
mod error;
mod logging;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(windows)]
    if let Some(code) = try_run_wua_install_child() {
        std::process::exit(code);
    }

    let _log_guard = logging::init();
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "DLSSync starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            use tauri::Manager;
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
                let _ = w.show();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::default().build())
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
        .manage(tray::TrayPrefs::new(true))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                use tauri::Manager;
                let app = window.app_handle();
                let prefs = app.state::<tray::TrayPrefs>();
                if prefs.close_to_tray() {
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
                match std::fs::read(&app_paths.catalog_cache)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| {
                        serde_json::from_slice::<dll_catalog::Catalog>(&bytes)
                            .map_err(|e| e.to_string())
                    }) {
                    Ok(cat) => {
                        *state.catalog.write() = Some(cat);
                        tracing::info!(
                            path = %app_paths.catalog_cache.display(),
                            "catalog loaded from cache",
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %app_paths.catalog_cache.display(),
                            error = %e,
                            "catalog cache unreadable",
                        );
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
            commands::ui_prefs::set_close_to_tray,
            commands::ui_prefs::get_close_to_tray,
            commands::ui_prefs::set_efficiency_mode,
            commands::ui_prefs::hide_main_window,
            commands::ui_prefs::show_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DLSSync");
}
