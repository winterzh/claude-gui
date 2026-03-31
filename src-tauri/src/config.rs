use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub api_key: String,
    pub base_url: String,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub model: String,
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
    // Preserve existing working_dir and model if config already exists
    let existing = load_config().ok().flatten();
    let config = AppConfig {
        api_key,
        base_url,
        working_dir: existing.as_ref().map(|c| c.working_dir.clone()).unwrap_or_default(),
        model: existing.as_ref().map(|c| c.model.clone()).unwrap_or_default(),
    };
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_working_dir(dir: String) -> Result<(), String> {
    let mut config = load_config()?.unwrap_or(AppConfig {
        api_key: String::new(),
        base_url: String::new(),
        working_dir: String::new(),
        model: String::new(),
    });
    config.working_dir = dir;
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_model_pref(model: String) -> Result<(), String> {
    let mut config = load_config()?.unwrap_or(AppConfig {
        api_key: String::new(),
        base_url: String::new(),
        working_dir: String::new(),
        model: String::new(),
    });
    config.model = model;
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
