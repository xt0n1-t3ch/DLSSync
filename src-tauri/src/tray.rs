use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const TRAY_ID: &str = "main";

pub const MENU_SHOW: &str = "tray.show";
pub const MENU_HIDE: &str = "tray.hide";
pub const MENU_PROGRESS: &str = "tray.progress";
pub const MENU_CHECK_UPDATE: &str = "tray.check_update";
pub const MENU_CHECK_NOW: &str = "tray.check_now";
pub const MENU_APPLY_ALL: &str = "tray.apply_all";
pub const MENU_QUIT: &str = "tray.quit";

pub const TRAY_EVENT_CHECK_UPDATE: &str = "tray://check-update";
pub const TRAY_EVENT_SHOW_PROGRESS: &str = "tray://show-progress";

pub const EVENT_BACKGROUND_SCAN_TICK: &str = "background:scan-tick";
pub const EVENT_BACKGROUND_APPLY_ALL: &str = "background:apply-all";

pub const TRAY_TOOLTIP_IDLE: &str = "DLSSync";

pub fn pending_tooltip(n: u32) -> String {
    if n == 0 {
        TRAY_TOOLTIP_IDLE.to_string()
    } else {
        format!("DLSSync — {n} games have updates")
    }
}

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, MENU_SHOW, "Show DLSSync", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, MENU_HIDE, "Hide to tray", true, None::<&str>)?;
    let progress = MenuItem::with_id(app, MENU_PROGRESS, "Show progress", true, None::<&str>)?;
    let check = MenuItem::with_id(
        app,
        MENU_CHECK_UPDATE,
        "Check for updates",
        true,
        None::<&str>,
    )?;
    let check_now = MenuItem::with_id(app, MENU_CHECK_NOW, "Check now", true, None::<&str>)?;
    let apply_all =
        MenuItem::with_id(app, MENU_APPLY_ALL, "Apply all updates", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show, &hide, &progress, &check, &check_now, &apply_all, &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(TRAY_TOOLTIP_IDLE)
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or_else(|| tauri::Error::AssetNotFound("default window icon missing".into()))?,
        )
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_SHOW => {
                show_main_window(app);
            }
            MENU_HIDE => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            MENU_PROGRESS => {
                show_main_window(app);
                let _ = app.emit(TRAY_EVENT_SHOW_PROGRESS, ());
            }
            MENU_CHECK_UPDATE => {
                show_main_window(app);
                let _ = app.emit(TRAY_EVENT_CHECK_UPDATE, ());
            }
            MENU_CHECK_NOW => {
                let _ = app.emit(EVENT_BACKGROUND_SCAN_TICK, ());
            }
            MENU_APPLY_ALL => {
                let _ = app.emit(EVENT_BACKGROUND_APPLY_ALL, ());
            }
            MENU_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_inflight<R: Runtime>(app: &AppHandle<R>, in_flight: usize) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let tooltip = if in_flight == 0 {
        TRAY_TOOLTIP_IDLE.to_string()
    } else {
        format!("DLSSync — {in_flight} in flight",)
    };
    let _ = tray.set_tooltip(Some(tooltip));
}

pub fn update_pending_count<R: Runtime>(app: &AppHandle<R>, n: u32) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let _ = tray.set_tooltip(Some(pending_tooltip(n)));
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_tooltip_idle_when_zero() {
        assert_eq!(pending_tooltip(0), TRAY_TOOLTIP_IDLE);
    }

    #[test]
    fn pending_tooltip_reports_count() {
        assert_eq!(pending_tooltip(1), "DLSSync — 1 games have updates");
        assert_eq!(pending_tooltip(7), "DLSSync — 7 games have updates");
    }
}
