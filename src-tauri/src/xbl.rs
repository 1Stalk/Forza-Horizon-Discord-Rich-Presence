use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE};
use serde_json::Value;

pub async fn poll_xbl_presence(api_key: &str) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("Disconnected".to_string());
    }

    let mut headers = HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(api_key) {
        headers.insert("X-Authorization", val);
    } else {
        return Err("Error: Invalid API Key format".to_string());
    }
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US"));

    let client = reqwest::Client::new();
    let res = client.get("https://xbl.io/api/v2/presence")
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if res.status() == 401 {
        return Err("Error: Invalid API Key".to_string());
    }
    if res.status() == 429 {
        return Err("Error: API Rate Limit".to_string());
    }
    if !res.status().is_success() {
        return Err(format!("API error: {}", res.status()));
    }

    let json: Value = res.json().await.map_err(|_| "Failed to parse API response".to_string())?;
    
    // Extract rich presence from the API response
    if let Some(devices) = json["content"]["devices"].as_array() {
        for device in devices {
            if let Some(titles) = device["titles"].as_array() {
                for title in titles {
                    if let Some(activity) = title.get("activity") {
                        if let Some(rp) = activity.get("richPresence") {
                            if let Some(rp_str) = rp.as_str() {
                                if !rp_str.is_empty() {
                                    return Ok(rp_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If no rich presence found, return a generic but positive state
    Ok("Connected (No Activity)".to_string())
}
