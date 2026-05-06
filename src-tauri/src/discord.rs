use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::Mutex;
use crate::telemetry::TelemetryData;
use crate::database::CarDatabase;
use crate::modules::GameModule;

pub struct DiscordService {
    client: Mutex<Option<DiscordIpcClient>>,
    client_id: String,
}

impl DiscordService {
    pub fn new(client_id: &str) -> Self {
        Self {
            client: Mutex::new(None),
            client_id: client_id.to_string(),
        }
    }

    pub fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut client = DiscordIpcClient::new(&self.client_id)?;
        client.connect()?;
        
        let mut lock = self.client.lock().unwrap();
        *lock = Some(client);
        
        Ok(())
    }

    pub fn disconnect(&self) {
        let mut lock = self.client.lock().unwrap();
        if let Some(mut client) = lock.take() {
            let _ = client.clear_activity();
            let _ = client.close();
        }
    }

    pub fn update_presence(&self, data: &TelemetryData, db: &CarDatabase, module: &dyn GameModule) {
        let mut lock = self.client.lock().unwrap();
        if let Some(client) = lock.as_mut() {
            if data.is_race_on == 0 {
                // In menus
                let activity = activity::Activity::new()
                    .state("In Menus")
                    .assets(activity::Assets::new().large_image("menu_icon"));
                let _ = client.set_activity(activity);
                return;
            }

            let car_name = db.get_car_name(data.car_ordinal);
            let class_str = module.format_class(data.car_class);
            
            let details = format!("{}", car_name);
            let state = format!("{:.0} km/h | Class {} ({})", data.speed_kmh.abs(), class_str, data.car_pi);

            let payload = activity::Activity::new()
                .details(&details)
                .state(&state)
                .assets(activity::Assets::new()
                    .large_image("car_default") // In a real app, you could map car IDs to asset keys
                    .large_text(&car_name));

            let _ = client.set_activity(payload);
        }
    }
}
