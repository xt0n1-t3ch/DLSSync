use tracing_subscriber::EnvFilter;

mod commands;
mod constants;
mod efficiency;
mod error;
mod paths;
mod state;
mod system_info;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dlssync=info,launcher_scan=info,dll_catalog=info".into()),
        )
        .with_target(true)
        .with_thread_ids(false)
        .compact()
        .try_init()
        .ok();

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
            commands::scan::enrich_game_art,
            commands::scan::fetch_steam_art,
            commands::shell::open_path,
            commands::shell::reveal_path,
            commands::catalog::refresh_catalog,
            commands::catalog::catalog_summary,
            commands::catalog::catalog_latest_shas,
            commands::catalog::list_releases,
            commands::apply::apply_update,
            commands::backup::list_backups,
            commands::backup::restore_backup,
            commands::backup::delete_backup,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::add_blacklist_entry,
            commands::settings::remove_blacklist_entry,
            commands::settings::save_window_state,
            commands::settings::get_app_paths,
            commands::advanced::set_dlss_debug_overlay,
            commands::advanced::get_dlss_debug_overlay,
            commands::system::get_system_info,
            commands::runtime::runtime_mode,
            commands::ui_prefs::set_close_to_tray,
            commands::ui_prefs::get_close_to_tray,
            commands::ui_prefs::set_efficiency_mode,
            commands::ui_prefs::hide_main_window,
            commands::ui_prefs::show_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DLSSync");
}
