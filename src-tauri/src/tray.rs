use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const MENU_SHOW: &str = "tray.show";
pub const MENU_HIDE: &str = "tray.hide";
pub const MENU_CHECK_UPDATE: &str = "tray.check_update";
pub const MENU_QUIT: &str = "tray.quit";

#[derive(Default)]
pub struct TrayPrefs {
    pub close_to_tray: Arc<AtomicBool>,
}

impl TrayPrefs {
    pub fn new(default_close_to_tray: bool) -> Self {
        Self {
            close_to_tray: Arc::new(AtomicBool::new(default_close_to_tray)),
        }
    }
    pub fn close_to_tray(&self) -> bool {
        self.close_to_tray.load(Ordering::Relaxed)
    }
    pub fn set_close_to_tray(&self, value: bool) {
        self.close_to_tray.store(value, Ordering::Relaxed);
    }
}

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, MENU_SHOW, "Show DLSSync", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, MENU_HIDE, "Hide to tray", true, None::<&str>)?;
    let check = MenuItem::with_id(
        app,
        MENU_CHECK_UPDATE,
        "Check for updates",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &check, &quit])?;

    let _tray = TrayIconBuilder::with_id("main")
        .tooltip("DLSSync")
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
            MENU_CHECK_UPDATE => {
                show_main_window(app);
                let _ = app.emit("tray://check-update", ());
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

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
