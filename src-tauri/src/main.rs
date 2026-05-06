// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod discord;
mod modules;
mod telemetry;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::os::windows::process::CommandExt;
use sysinfo::System;
use tauri::{Manager, Emitter};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
use tauri::menu::{Menu, MenuItem};
use tokio::sync::broadcast;

use database::CarDatabase;
use discord::DiscordService;
use modules::{fh4::FH4Module, fh5::FH5Module, GameModule};
use telemetry::{TelemetryServer, TelemetryData};

struct AppState {
    modules: Vec<Arc<dyn GameModule>>,
}

#[tauri::command]
async fn fix_uwp_isolation(state: tauri::State<'_, AppState>) -> Result<String, String> {
    for module in &state.modules {
        let package_name = module.uwp_package_name();
        if package_name.is_empty() { continue; }
        
        // Command to exempt UWP app from loopback isolation.
        let status = std::process::Command::new("powershell")
            .args(&[
                "-Command",
                &format!("Start-Process -FilePath 'CheckNetIsolation.exe' -ArgumentList 'LoopbackExempt -a -n={}' -Verb RunAs -WindowStyle Hidden", package_name)
            ])
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err(format!("Failed to fix isolation for {}", module.game_name()));
        }
    }
    
    Ok("Isolation fixed for all supported games".into())
}

#[tauri::command]
async fn check_db_updates(app: tauri::AppHandle) -> Result<String, String> {
    CarDatabase::check_for_updates(app).await
}

#[tauri::command]
async fn check_uwp_status(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let output = std::process::Command::new("CheckNetIsolation")
        .arg("LoopbackExempt")
        .arg("-s")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e: std::io::Error| e.to_string())?;

    let output_str = String::from_utf8_lossy(&output.stdout).to_lowercase();
    
    // Check if ALL packages are exempt
    for module in &state.modules {
        let package_name = module.uwp_package_name().to_lowercase();
        if package_name.is_empty() { continue; }
        
        if !output_str.contains(&package_name) {
            return Ok(false); // At least one game needs fixing
        }
    }
    
    Ok(true)
}

#[tauri::command]
fn ui_ready() {
    // Frontend is ready to receive status updates
}

#[tauri::command]
fn toggle_autostart(enable: bool) -> Result<String, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_path_str = exe_path.to_str().unwrap_or_default();
    
    let key_path = r#"HKCU\Software\Microsoft\Windows\CurrentVersion\Run"#;
    let app_name = "forzarichpresence";

    if enable {
        let status = std::process::Command::new("reg")
            .args(&["add", key_path, "/v", app_name, "/t", "REG_SZ", "/d", &format!("\"{}\"", exe_path_str), "/f"])
            .creation_flags(0x08000000)
            .status()
            .map_err(|e| e.to_string())?;
        
        if status.success() { Ok("Enabled".into()) } else { Err("Failed".into()) }
    } else {
        let status = std::process::Command::new("reg")
            .args(&["delete", key_path, "/v", app_name, "/f"])
            .creation_flags(0x08000000)
            .status()
            .map_err(|e| e.to_string())?;
            
        if status.success() { Ok("Disabled".into()) } else { Err("Failed".into()) }
    }
}

#[tauri::command]
fn is_autostart_enabled() -> Result<bool, String> {
    let key_path = r#"HKCU\Software\Microsoft\Windows\CurrentVersion\Run"#;
    let app_name = "forzarichpresence";
    
    let output = std::process::Command::new("reg")
        .args(&["query", key_path, "/v", app_name])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e: std::io::Error| e.to_string())?;
        
    Ok(output.status.success())
}

#[tauri::command]
fn hide_window(window: tauri::Window) {
    let _ = window.hide();
}

#[tauri::command]
fn show_window(window: tauri::Window) {
    let _ = window.show();
    let _ = window.set_focus();
}

fn create_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "Settings", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                std::process::exit(0);
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .icon(app.default_window_icon().unwrap().clone())
        .build(app)?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            create_tray(app)?;
            
            let app_handle = app.handle().clone();
            
            let db = Arc::new(Mutex::new(CarDatabase::new(&app_handle)));
            let modules: Vec<Arc<dyn GameModule>> = vec![
                Arc::new(FH4Module),
                Arc::new(FH5Module),
            ];

            app.manage(AppState {
                modules: modules.clone(),
            });

            // Start background monitor task
            let app_handle_clone = app_handle.clone();
            
            tauri::async_runtime::spawn(async move {
                let mut sys = System::new();
                let server = Arc::new(TelemetryServer::new());
                
                let mut _telemetry_tx: Option<broadcast::Sender<TelemetryData>> = None;
                let mut is_game_running = false;
                let mut active_discord: Option<Arc<DiscordService>> = None;

                loop {
                    sys.refresh_processes_specifics(sysinfo::ProcessRefreshKind::new());
                    
                    let mut active_module: Option<Arc<dyn GameModule>> = None;
                    for module in &modules {
                        let process_name = module.target_process_name();
                        for process in sys.processes().values() {
                            if process.name() == process_name {
                                active_module = Some(module.clone());
                                break;
                            }
                        }
                        if active_module.is_some() { break; }
                    }

                    if let Some(module) = active_module {
                        if !is_game_running {
                            // Game started
                            is_game_running = true;
                            println!("Game started: {}", module.game_name());
                            
                            let (tx, _) = broadcast::channel::<TelemetryData>(16);
                            let discord_service = Arc::new(DiscordService::new(module.discord_client_id()));
                            let _ = discord_service.connect();
                            active_discord = Some(discord_service.clone());
                            
                            server.start(9909, tx.clone());
                            _telemetry_tx = Some(tx.clone());

                            let _ = app_handle_clone.emit("status_update", serde_json::json!({
                                "status": "connected",
                                "game": module.game_name(),
                                "details": "Broadcasting presence..."
                            }));
                            
                            // Spawn Discord updater loop
                            let db_clone = db.clone();
                            let module_clone = module.clone();
                            let mut rx_clone = tx.subscribe();
                            
                            tauri::async_runtime::spawn(async move {
                                let mut last_update = tokio::time::Instant::now();
                                while let Ok(data) = rx_clone.recv().await {
                                    if last_update.elapsed() >= Duration::from_millis(2000) {
                                        let db_lock = db_clone.lock().unwrap();
                                        discord_service.update_presence(&data, &db_lock, module_clone.as_ref());
                                        last_update = tokio::time::Instant::now();
                                    }
                                }
                            });
                        }
                    } else if is_game_running {
                        // Game stopped
                        is_game_running = false;
                        println!("Game stopped.");
                        
                        server.stop();
                        _telemetry_tx = None; // Drops the sender, terminating the updater loop
                        if let Some(discord) = active_discord.take() {
                            discord.disconnect();
                        }

                        let _ = app_handle_clone.emit("status_update", serde_json::json!({
                            "status": "disconnected",
                            "game": "",
                            "details": "Waiting for game..."
                        }));
                    }

                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            fix_uwp_isolation,
            check_uwp_status,
            check_db_updates,
            toggle_autostart,
            is_autostart_enabled,
            hide_window,
            show_window,
            ui_ready
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
