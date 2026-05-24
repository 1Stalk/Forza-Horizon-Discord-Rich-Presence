use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::Mutex;
use crate::telemetry::TelemetryData;
use crate::database::CarDatabase;
use crate::modules::GameModule;

#[cfg(target_os = "linux")]
fn get_active_sockets() -> Vec<std::path::PathBuf> {
    use std::fs;
    let mut sockets = Vec::new();
    
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        if let Ok(entries) = fs::read_dir("/run/user") {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        let name = entry.file_name();
                        if let Some(name_str) = name.to_str() {
                            if name_str.chars().all(|c| c.is_ascii_digit()) {
                                return format!("/run/user/{}", name_str);
                            }
                        }
                    }
                }
            }
        }
        "/run/user/1000".to_string()
    });

    let base_paths = vec![
        std::path::PathBuf::from(&xdg_runtime_dir),
        std::path::PathBuf::from("/tmp"),
    ];

    // Subpaths to search in base_paths
    let subpaths = vec![
        "".to_string(),
        "app/com.discordapp.Discord".to_string(),
        "app/dev.vencord.Vesktop".to_string(),
        ".flatpak/com.discordapp.Discord/xdg-run".to_string(),
        ".flatpak/dev.vencord.Vesktop/xdg-run".to_string(),
        "snap.discord-canary".to_string(),
        "snap.discord".to_string(),
    ];

    for base in &base_paths {
        for sub in &subpaths {
            let path = if sub.is_empty() {
                base.clone()
            } else {
                base.join(sub)
            };

            if path.exists() && path.is_dir() {
                if let Ok(entries) = fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        let file_name = entry.file_name();
                        let file_name_str = file_name.to_string_lossy();
                        if file_name_str.starts_with("discord-ipc-") {
                            let socket_path = entry.path();
                            if socket_path.exists() {
                                sockets.push(socket_path);
                            }
                        }
                    }
                }
            }
        }
    }

    sockets.sort();
    sockets.dedup();
    sockets
}

pub struct DiscordService {
    clients: Mutex<Vec<DiscordIpcClient>>,
    client_id: String,
    start_time: i64,
    last_car_data: Mutex<Option<TelemetryData>>,
    last_xbl_state: Mutex<Option<String>>,
}

impl DiscordService {
    pub fn new(client_id: &str) -> Self {
        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            clients: Mutex::new(Vec::new()),
            client_id: client_id.to_string(),
            start_time,
            last_car_data: Mutex::new(None),
            last_xbl_state: Mutex::new(None),
        }
    }

    pub fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut clients = Vec::new();

        #[cfg(target_os = "linux")]
        {
            let sockets = get_active_sockets();
            println!("Found active Discord IPC sockets: {:?}", sockets);
            
            if sockets.is_empty() {
                let mut client = DiscordIpcClient::new(&self.client_id)?;
                if let Err(e) = client.connect() {
                    println!("Default fallback connection failed: {:?}", e);
                } else {
                    println!("Successfully connected via default fallback client.");
                    clients.push(client);
                }
            } else {
                for (idx, socket_path) in sockets.iter().enumerate() {
                    let temp_dir = std::path::PathBuf::from(format!("/tmp/forza_rpc_socket_link_{}", idx));
                    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
                        println!("Failed to create temp dir {:?}: {:?}", temp_dir, e);
                        continue;
                    }
                    
                    let link_path = temp_dir.join("discord-ipc-0");
                    let _ = std::fs::remove_file(&link_path);
                    
                    if let Err(e) = std::os::unix::fs::symlink(socket_path, &link_path) {
                        println!("Failed to create symlink from {:?} to {:?}: {:?}", socket_path, link_path, e);
                        continue;
                    }

                    // Save original environment
                    let env_keys = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"];
                    let mut original_env = Vec::new();
                    for key in &env_keys {
                        original_env.push((*key, std::env::var(key).ok()));
                    }

                    // Override env to direct client to our temp symlink
                    for key in &env_keys {
                        std::env::set_var(key, &temp_dir);
                    }

                    // Connect client
                    let mut client = DiscordIpcClient::new(&self.client_id)?;
                    let res = client.connect();

                    // Restore env
                    for (key, val) in original_env {
                        if let Some(v) = val {
                            std::env::set_var(key, v);
                        } else {
                            std::env::remove_var(key);
                        }
                    }

                    // Clean up symlink/temp dir
                    let _ = std::fs::remove_file(&link_path);
                    let _ = std::fs::remove_dir(&temp_dir);

                    match res {
                        Ok(_) => {
                            println!("Successfully connected client to socket: {:?}", socket_path);
                            clients.push(client);
                        }
                        Err(e) => {
                            println!("Failed to connect client to socket {:?}: {:?}", socket_path, e);
                        }
                    }
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let mut client = DiscordIpcClient::new(&self.client_id)?;
            client.connect()?;
            clients.push(client);
        }

        let mut lock = self.clients.lock().unwrap();
        *lock = clients;
        
        Ok(())
    }

    pub fn disconnect(&self) {
        let mut lock = self.clients.lock().unwrap();
        for mut client in lock.drain(..) {
            let _ = client.clear_activity();
            let _ = client.close();
        }
    }

    pub fn update_presence(&self, data_opt: Option<&TelemetryData>, db: &CarDatabase, module: &dyn GameModule, xbl_state: Option<&str>) {
        let mut lock = self.clients.lock().unwrap();
        if lock.is_empty() {
            return;
        }

        let valid_xbl_state = xbl_state.and_then(|s| {
            let s = s.trim();
            let s_lower = s.to_lowercase();
            if s_lower.starts_with("error:") 
                || s_lower.starts_with("api error")
                || s_lower.starts_with("network error")
                || s == "Connected (No Activity)"
                || s == "Connecting..."
                || s == "Disconnected"
                || s == "Waiting for game..." {
                None
            } else {
                Some(s)
            }
        });

        // Handle XBL state caching
        let effective_xbl_state = {
            let mut last_xbl_lock = self.last_xbl_state.lock().unwrap();
            if let Some(xbl) = valid_xbl_state {
                *last_xbl_lock = Some(xbl.to_string());
                Some(xbl.to_string())
            } else {
                last_xbl_lock.clone()
            }
        };
        let effective_xbl_ref = effective_xbl_state.as_deref();

        // Handle car data caching
        let effective_data = {
            let mut last_lock = self.last_car_data.lock().unwrap();
            if let Some(data) = data_opt {
                if data.car_ordinal != 0 {
                    *last_lock = Some(data.clone());
                    Some(data.clone())
                } else {
                    last_lock.clone()
                }
            } else {
                last_lock.clone()
            }
        };

        // Use "Exploring [Country]" fallback if OpenXBL is not connected but we have telemetry
        let effective_xbl_or_fallback = effective_xbl_ref.or_else(|| {
            if effective_data.is_some() {
                let fallback = match module.game_name() {
                    "Forza Horizon 4" => "Exploring Great Britain",
                    "Forza Horizon 5" => "Exploring Mexico",
                    _ => "Exploring Japan",
                };
                Some(fallback)
            } else {
                None
            }
        });

        let mut details_str = String::new(); // Top line
        let mut state_str = String::new();   // Bottom line
        let mut payload = activity::Activity::new()
            .timestamps(activity::Timestamps::new().start(self.start_time));
        let mut assets = activity::Assets::new();
        let mut class_key = None;
        let mut hover_text = None;

        if let Some(data) = &effective_data {
            let car_name_opt = db.get_car_name_opt(data.car_ordinal);
            let is_unknown = car_name_opt.is_none();
            let car_name = car_name_opt.unwrap_or_else(|| format!("Unknown Car ({})", data.car_ordinal));
            
            let display_name = if car_name.chars().count() > 25 {
                let truncated: String = car_name.chars().take(22).collect();
                format!("{}...", truncated)
            } else {
                car_name.clone()
            };

            let class_str = module.format_class(data.car_class);
            let telemetry_str = format!("{} | {} ({})", display_name, class_str, data.car_pi);

            // Car display info is now always shown if effective_data exists
            // BUT skip if the car is unknown (user request)
            if let Some(xbl) = effective_xbl_or_fallback {
                details_str = xbl.to_string();
                if !is_unknown {
                    state_str = telemetry_str;
                }
            } else {
                if !is_unknown {
                    details_str = car_name.clone();
                    state_str = format!("{} ({})", class_str, data.car_pi);
                }
            }

            class_key = Some(format!("class_{}", class_str.to_lowercase()));
            hover_text = Some(format!("{} | {} ({})", car_name, class_str, data.car_pi));

            assets = assets.large_image(module.logo_asset_key());
            
            if !is_unknown {
                if let (Some(ref key), Some(ref text)) = (&class_key, &hover_text) {
                    assets = assets.small_image(key).small_text(text);
                }
            }
            
            if let Some(xbl) = effective_xbl_or_fallback {
                assets = assets.large_text(xbl);
            }
        } else {
            // No telemetry data EVER seen yet
            if let Some(xbl) = effective_xbl_or_fallback {
                details_str = xbl.to_string();
                assets = assets.large_image(module.logo_asset_key()).large_text(xbl);
            } else {
                assets = assets.large_image(module.logo_asset_key());
            }
        }

        if !details_str.is_empty() {
            payload = payload.details(&details_str);
        }
        if !state_str.is_empty() {
            payload = payload.state(&state_str);
        }
        payload = payload.assets(assets);

        // Broadcast to all active clients
        for client in lock.iter_mut() {
            let _ = client.set_activity(payload.clone());
        }
    }
}
