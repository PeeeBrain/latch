mod auth;
mod commands;
mod crypto;
mod password_generator;
mod vault;
mod vault_health;

use std::time::SystemTime;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;
use vault::SESSION_TIMEOUT_SECS;

pub fn spawn_session_timer(
    app_handle: AppHandle,
    state: std::sync::Weak<std::sync::Mutex<vault::coordinator::VaultCoordinator>>,
    session_start: SystemTime,
) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(SESSION_TIMEOUT_SECS)).await;
        let Some(state) = state.upgrade() else {
            return;
        };
        if let Ok(mut coordinator) = state.lock() {
            let expired = coordinator
                .with_vault(|_, workspace| {
                    if workspace.session_start == Some(session_start) {
                        workspace.lock();
                        return Ok(true);
                    }
                    Ok(false)
                })
                .unwrap_or(false);
            if expired {
                let _ = app_handle.emit("vault-locked", ());
            }
        };
    });
}

fn setup_system_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show Latch", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = MenuBuilder::new(app)
        .item(&show_item)
        .separator()
        .item(&quit_item)
        .build()?;

    let tray_icon = app
        .default_window_icon()
        .ok_or("Failed to get window icon")?
        .clone();

    let _tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("Latch Password Manager")
        .icon(tray_icon)
        .on_menu_event(move |app, event| match event.id.0.as_str() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if cfg!(debug_assertions) {
        dotenvy::dotenv().ok();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_google_auth::init())
        .plugin(tauri_plugin_biometry::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let storage =
                vault::storage::VaultStorage::new().expect("Failed to initialize vault storage");
            let workspace = vault::workspace::Workspace::new();
            app.manage(commands::VaultState::new(
                storage,
                workspace,
                app.handle().clone(),
            ));

            let handle = app.handle().clone();
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcut("Ctrl+Space")?
                    .with_handler(move |_app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            if let Some(window) = handle.get_webview_window("main") {
                                let is_visible = window.is_visible().unwrap_or(false);
                                if is_visible {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    })
                    .build(),
            )?;

            if let Err(e) = setup_system_tray(app) {
                eprintln!("Failed to setup system tray: {}", e);
            }

            let window = app
                .get_webview_window("main")
                .ok_or("Failed to get main window")?;
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window_clone.hide();
                    api.prevent_close();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault::init_vault_oauth,
            commands::vault::init_vault_with_key,
            commands::vault::init_vault,
            commands::vault::unlock_vault_oauth,
            commands::vault::unlock_vault_with_key,
            commands::vault::unlock_vault,
            commands::vault::get_vault_auth_method,
            commands::vault::reencrypt_vault,
            commands::vault::reencrypt_vault_to_oauth,
            commands::vault::migrate_to_oauth,
            commands::vault::vault_status,
            commands::session::lock_vault,
            commands::session::get_auth_preferences,
            commands::credential::search_entries,
            commands::credential::request_secret,
            commands::credential::add_entry,
            commands::credential::get_full_entry,
            commands::credential::update_entry,
            commands::credential::delete_entry,
            commands::totp::get_totp_token,
            commands::alias::save_alias_config,
            commands::alias::generate_email_mask,
            commands::generator::generate_password,
            commands::generator::analyze_password_strength,
            commands::health::check_vault_health,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
