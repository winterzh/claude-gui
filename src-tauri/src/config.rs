use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub api_key: String,
    pub base_url: String,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub active_profile: String,
}

fn default_config() -> AppConfig {
    AppConfig {
        api_key: String::new(),
        base_url: String::new(),
        working_dir: String::new(),
        model: String::new(),
        profiles: Vec::new(),
        active_profile: String::new(),
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claude-launcher");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.json")
}

#[tauri::command]
pub fn save_config(api_key: String, base_url: String) -> Result<(), String> {
    let existing = load_config().ok().flatten();
    let mut config = existing.unwrap_or_else(default_config);
    config.api_key = api_key;
    config.base_url = base_url;
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_working_dir(dir: String) -> Result<(), String> {
    let mut config = load_config()?.unwrap_or_else(default_config);
    config.working_dir = dir;
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_model_pref(model: String) -> Result<(), String> {
    let mut config = load_config()?.unwrap_or_else(default_config);
    config.model = model;
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_profiles(profiles: Vec<Profile>, active_profile: String) -> Result<(), String> {
    let mut config = load_config()?.unwrap_or_else(default_config);
    config.profiles = profiles;
    config.active_profile = active_profile;
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn load_config() -> Result<Option<AppConfig>, String> {
    let path = config_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config: AppConfig = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(Some(config))
}

#[tauri::command]
pub async fn test_connection(api_key: String, base_url: String) -> Result<String, String> {
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{}/messages", base)
    } else {
        format!("{}/v1/messages", base)
    };

    let body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 10,
        "messages": [{"role": "user", "content": "hi"}]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let status = resp.status();
    if status.is_success() {
        Ok("Connected successfully!".to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("HTTP {}: {}", status, &text[..text.len().min(200)]))
    }
}
