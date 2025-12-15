mod api;
mod core;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    menu::MenuBuilder, tray::TrayIconBuilder, ActivationPolicy, AppHandle, Manager, RunEvent,
    WindowEvent,
};
#[cfg(desktop)]
use tauri_plugin_global_shortcut::{Builder as ShortcutBuilder, ShortcutState};

use api::types::AppState;

const MAIN_WINDOW_LABEL: &str = "main";
const MAIN_TRAY_ID: &str = "main-tray";
const TRAY_MENU_SHOW: &str = "tray-show";
const TRAY_MENU_QUIT: &str = "tray-quit";
const GLOBAL_HOTKEY_TOGGLE: &str = "ctrl+space";

static ALLOW_APP_EXIT: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default());

    #[cfg(target_os = "macos")]
    {
        builder = builder.enable_macos_default_menu(false);
    }

    let app = builder
        .setup(|app| {
            setup_tray(app)?;
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(ActivationPolicy::Accessory);
                app.set_dock_visibility(false);
            }
            #[cfg(desktop)]
            if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                if let Some(monitor) = app.primary_monitor()? {
                    let screen = monitor.size();
                    let target_width = screen.width / 2;
                    let target_height = screen.height / 2;

                    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
                        target_width,
                        target_height,
                    )));

                    let pos_x =
                        i32::try_from((screen.width.saturating_sub(target_width)) / 2).unwrap_or(0);
                    let pos_y = i32::try_from((screen.height.saturating_sub(target_height)) / 2)
                        .unwrap_or(0);
                    let _ = window.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition::new(pos_x, pos_y),
                    ));
                }
            }
            #[cfg(desktop)]
            {
                let handle = app.handle();
                handle.plugin(
                    ShortcutBuilder::new()
                        .with_shortcut(GLOBAL_HOTKEY_TOGGLE)?
                        .with_handler(|app, _shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                toggle_main_window(app);
                            }
                        })
                        .build(),
                )?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }

            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let _ = window.hide();
                    api.prevent_close();
                    update_tray_menu(window.app_handle(), false);
                }
                WindowEvent::Focused(false) => {
                    let _ = window.hide();
                    update_tray_menu(window.app_handle(), false);
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            api::commands::get_state_snapshot,
            api::commands::start_app,
            api::commands::set_query,
            api::commands::set_selection_delta,
            api::commands::invoke_action,
            api::commands::handle_escape,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            if !ALLOW_APP_EXIT.swap(false, Ordering::Relaxed) {
                api.prevent_exit();
            }
        }
    });
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let tray_menu = MenuBuilder::new(app)
        .text(TRAY_MENU_SHOW, "Show")
        .text(TRAY_MENU_QUIT, "Quit")
        .build()?;

    let mut tray_builder = TrayIconBuilder::with_id(MAIN_TRAY_ID)
        .menu(&tray_menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            TRAY_MENU_SHOW => toggle_main_window(app),
            TRAY_MENU_QUIT => {
                ALLOW_APP_EXIT.store(true, Ordering::Relaxed);
                app.exit(0);
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    tray_builder.build(app)?;

    Ok(())
}

fn update_tray_menu(app: &AppHandle, is_visible: bool) {
    if let Some(tray) = app.tray_by_id(MAIN_TRAY_ID) {
        let show_text = if is_visible { "Hide" } else { "Show" };
        if let Ok(menu) = MenuBuilder::new(app)
            .text(TRAY_MENU_SHOW, show_text)
            .text(TRAY_MENU_QUIT, "Quit")
            .build()
        {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        match window.is_visible() {
            Ok(true) => {
                let _ = window.hide();
                update_tray_menu(app, false);
            }
            _ => {
                let _ = window.show();
                let _ = window.set_focus();
                update_tray_menu(app, true);
            }
        }
    }
}
