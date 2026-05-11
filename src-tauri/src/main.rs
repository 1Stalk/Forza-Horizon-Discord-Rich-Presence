// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod discord;
mod modules;
mod telemetry;
mod xbl;

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
    active_game: Arc<Mutex<Option<String>>>,
    xbl_api_key: Arc<Mutex<String>>,
    telemetry_port: Arc<Mutex<u16>>,
}

#[tauri::command]
fn fix_uwp_isolation(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let mut needs_uac = false;

    // First try direct execution (works if app is already running as Admin)
    for module in &state.modules {
        let package_name = module.uwp_package_name();
        if package_name.is_empty() { continue; }

        let cmd_str = format!("CheckNetIsolation LoopbackExempt -a -n={}", package_name);
        let status = std::process::Command::new("cmd")
            .args(&["/c", &cmd_str])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if let Ok(exit_status) = status {
            if !exit_status.success() {
                needs_uac = true;
                break;
            }
        } else {
            needs_uac = true;
            break;
        }
    }

    if !needs_uac {
        return Ok("Isolation fixed directly".into());
    }

    // Fallback to PowerShell UAC (Silent)
    let mut script = String::new();
    for module in &state.modules {
        let package_name = module.uwp_package_name();
        if package_name.is_empty() { continue; }
        if !script.is_empty() {
            script.push_str(" & ");
        }
        script.push_str(&format!("CheckNetIsolation LoopbackExempt -a -n={}", package_name));
    }

    let status = std::process::Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle", "Hidden",
            "-Command",
            &format!("Start-Process -FilePath 'cmd.exe' -ArgumentList '/c {}' -Verb RunAs -Wait -WindowStyle Hidden", script)
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("Failed to fix isolation (UAC denied or command failed)".into());
    }
    
    Ok("Isolation fixed via UAC".into())
}

#[tauri::command]
async fn check_db_updates(app: tauri::AppHandle) -> Result<String, String> {
    CarDatabase::check_for_updates(app).await
}

#[tauri::command]
fn check_uwp_status(state: tauri::State<'_, AppState>) -> Result<bool, String> {
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
fn ui_ready(app: tauri::AppHandle, state: tauri::State<'_, AppState>) {
    let game = state.active_game.lock().unwrap().clone();
    if let Some(game_name) = game {
        let _ = app.emit("status_update", serde_json::json!({
            "status": "connected",
            "game": game_name,
            "details": "Broadcasting presence..."
        }));
    } else {
        let _ = app.emit("status_update", serde_json::json!({
            "status": "disconnected",
            "game": "",
            "details": "Launch game to broadcast"
        }));
    }
}

#[tauri::command]
fn update_xbl_settings(api_key: String, state: tauri::State<'_, AppState>) {
    *state.xbl_api_key.lock().unwrap() = api_key;
}

#[tauri::command]
fn update_telemetry_port(port: u16, state: tauri::State<'_, AppState>) {
    *state.telemetry_port.lock().unwrap() = port;
}

#[tauri::command]
fn open_url(url: String) {
    let _ = std::process::Command::new("cmd")
        .args(&["/c", "start", "", &url])
        .creation_flags(0x08000000)
        .spawn();
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

            let active_game = Arc::new(Mutex::new(None));
            let xbl_api_key = Arc::new(Mutex::new(String::new()));
            let telemetry_port = Arc::new(Mutex::new(9909));
            
            app.manage(AppState {
                modules: modules.clone(),
                active_game: active_game.clone(),
                xbl_api_key: xbl_api_key.clone(),
                telemetry_port: telemetry_port.clone(),
            });

            // Start background monitor task
            let app_handle_clone = app_handle.clone();
            let xbl_api_key_clone = xbl_api_key.clone();
            let telemetry_port_clone = telemetry_port.clone();
            
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
                            *active_game.lock().unwrap() = Some(module.game_name().to_string());
                            println!("Game started: {}", module.game_name());
                            
                            let (tx, _) = broadcast::channel::<TelemetryData>(16);
                            let discord_service = Arc::new(DiscordService::new(module.discord_client_id()));
                            let _ = discord_service.connect();
                            active_discord = Some(discord_service.clone());
                            
                            let port = *telemetry_port_clone.lock().unwrap();
                            server.start(port, tx.clone());
                            _telemetry_tx = Some(tx.clone());

                            let _ = app_handle_clone.emit("status_update", serde_json::json!({
                                "status": "connected",
                                "game": module.game_name(),
                                "details": "Broadcasting presence...",
                                "xbl_status": "Connecting..."
                            }));
                            
                            let xbl_status = Arc::new(Mutex::new(None::<String>));
                            let xbl_status_clone_loop = xbl_status.clone();
                            let key_clone = xbl_api_key_clone.clone();
                            let app_handle_clone2 = app_handle_clone.clone();
                            let module_name = module.game_name().to_string();
                            
                            let (xbl_stop_tx, mut xbl_stop_rx) = tokio::sync::mpsc::channel::<()>(1);
                            
                            tauri::async_runtime::spawn(async move {
                                let mut last_poll = tokio::time::Instant::now().checked_sub(Duration::from_secs(26)).unwrap();
                                loop {
                                    tokio::select! {
                                        _ = xbl_stop_rx.recv() => break,
                                        _ = tokio::time::sleep(Duration::from_millis(1000)) => {}
                                    }
                                    
                                    if last_poll.elapsed() >= Duration::from_secs(26) {
                                        let key = key_clone.lock().unwrap().clone();
                                        if !key.is_empty() {
                                            match xbl::poll_xbl_presence(&key).await {
                                                Ok(status) => {
                                                    *xbl_status_clone_loop.lock().unwrap() = Some(status.clone());
                                                    let _ = app_handle_clone2.emit("status_update", serde_json::json!({
                                                        "status": "connected",
                                                        "game": module_name,
                                                        "details": "Broadcasting presence...",
                                                        "xbl_status": status
                                                    }));
                                                },
                                                Err(err) => {
                                                    if err != "Disconnected" {
                                                        *xbl_status_clone_loop.lock().unwrap() = Some(err.clone());
                                                        let _ = app_handle_clone2.emit("status_update", serde_json::json!({
                                                            "status": "connected",
                                                            "game": module_name,
                                                            "details": "Broadcasting presence...",
                                                            "xbl_status": err
                                                        }));
                                                    }
                                                }
                                            }
                                        } else {
                                            *xbl_status_clone_loop.lock().unwrap() = None;
                                            let _ = app_handle_clone2.emit("status_update", serde_json::json!({
                                                "status": "connected",
                                                "game": module_name,
                                                "details": "Broadcasting presence...",
                                                "xbl_status": "Disconnected"
                                            }));
                                        }
                                        last_poll = tokio::time::Instant::now();
                                    }
                                }
                            });
                            
                            // Spawn Discord updater loop
                            let db_clone = db.clone();
                            let module_clone = module.clone();
                            let mut rx_clone = tx.subscribe();
                            
                            tauri::async_runtime::spawn(async move {
                                let mut last_update = tokio::time::Instant::now();
                                // Bind the xbl_stop_tx to this loop's lifetime so it drops when this breaks
                                let _stop_tx = xbl_stop_tx; 
                                loop {
                                    match rx_clone.recv().await {
                                        Ok(data) => {
                                            if last_update.elapsed() >= Duration::from_millis(1500) {
                                                let db_lock = db_clone.lock().unwrap();
                                                let xbl_lock = xbl_status.lock().unwrap();
                                                discord_service.update_presence(&data, &db_lock, module_clone.as_ref(), xbl_lock.as_deref());
                                                last_update = tokio::time::Instant::now();
                                            }
                                        }
                                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                                    }
                                }
                            });
                        }
                    } else if is_game_running {
                        // Game stopped
                        is_game_running = false;
                        *active_game.lock().unwrap() = None;
                        println!("Game stopped.");
                        
                        server.stop();
                        _telemetry_tx = None; // Drops the sender, terminating the updater loop
                        if let Some(discord) = active_discord.take() {
                            discord.disconnect();
                        }

                        let _ = app_handle_clone.emit("status_update", serde_json::json!({
                            "status": "disconnected",
                            "game": "",
                            "details": "Launch game to broadcast"
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
            ui_ready,
            update_xbl_settings,
            open_url,
            update_telemetry_port
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
