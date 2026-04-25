use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    /// Per-profile model selection — overrides the legacy top-level `model`.
    #[serde(default)]
    pub model: String,
    /// Which env var the api_key should land in. Empty defaults to
    /// `ANTHROPIC_API_KEY` (x-api-key style — Anthropic, most relays).
    /// Set to `ANTHROPIC_AUTH_TOKEN` for Bearer-style proxies (DeepSeek, etc.).
    /// Setting only one prevents Claude Code's "Auth conflict" warning.
    #[serde(default)]
    pub auth_env: String,
    /// Extra environment variables injected into Claude Code at spawn time.
    /// Lets a preset (e.g. DeepSeek v4) ship a bundle of env defaults so the
    /// user only has to enter the API key.
    #[serde(default)]
    pub extra_env: HashMap<String, String>,
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
    #[serde(default)]
    pub skip_permissions: bool,
}

fn default_config() -> AppConfig {
    AppConfig {
        api_key: String::new(),
        base_url: String::new(),
        working_dir: String::new(),
        model: String::new(),
        profiles: Vec::new(),
        active_profile: String::new(),
        skip_permissions: false,
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(env!("CONFIG_DIR_NAME"));
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
pub fn save_skip_permissions(skip: bool) -> Result<(), String> {
    let mut config = load_config()?.unwrap_or_else(default_config);
    config.skip_permissions = skip;
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
pub async fn fetch_models(api_key: String, base_url: String) -> Result<Vec<String>, String> {
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{}/models", base)
    } else {
        format!("{}/v1/models", base)
    };

    let output = tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new("curl");
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd.args([
            "-s",
            "-m",
            "15",
            &url,
            "-H",
            &format!("x-api-key: {}", api_key),
            "-H",
            &format!("Authorization: Bearer {}", api_key),
            "-H",
            "anthropic-version: 2023-06-01",
        ]);
        cmd.output()
    })
    .await
    .map_err(|_| "Internal error".to_string())?
    .map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "curl not found. Please install curl or check your PATH.".to_string()
        } else {
            format!("Failed to fetch models: {}", e)
        }
    })?;

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    if body.trim().is_empty() {
        return Err("Empty response from server.".to_string());
    }

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| "Server did not return JSON. The endpoint may not support /v1/models.".to_string())?;

    // Try common schemas: { data: [{id}] } (OpenAI/Anthropic style) or { models: [{id|name}] }
    let pick_id = |v: &serde_json::Value| -> Option<String> {
        v.get("id")
            .and_then(|x| x.as_str())
            .or_else(|| v.get("name").and_then(|x| x.as_str()))
            .or_else(|| v.get("model").and_then(|x| x.as_str()))
            .map(|s| s.to_string())
    };

    let arr = json
        .get("data")
        .and_then(|d| d.as_array())
        .or_else(|| json.get("models").and_then(|d| d.as_array()))
        .or_else(|| json.as_array());

    let mut ids: Vec<String> = match arr {
        Some(a) => a.iter().filter_map(pick_id).collect(),
        None => {
            // Surface API-level error message when present
            let err = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unexpected response shape from /v1/models");
            return Err(err.to_string());
        }
    };

    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Err("Model list is empty.".to_string());
    }
    Ok(ids)
}

#[tauri::command]
pub async fn test_connection(api_key: String, base_url: String) -> Result<String, String> {
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{}/messages", base)
    } else {
        format!("{}/v1/messages", base)
    };

    let body = r#"{"model":"claude-sonnet-4-20250514","max_tokens":10,"messages":[{"role":"user","content":"hi"}]}"#;

    // Use curl for maximum compatibility (reqwest has TLS issues in Tauri on some platforms)
    let output = tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new("curl");
        // Hide console window on Windows
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        cmd.args([
            "-s",
            "-w",
            "\n%{http_code}",
            "-m",
            "15",
            "-X",
            "POST",
            &url,
            "-H",
            &format!("x-api-key: {}", api_key),
            "-H",
            "anthropic-version: 2023-06-01",
            "-H",
            "content-type: application/json",
            "-d",
            body,
        ]);
        cmd.output()
    })
    .await
    .map_err(|_| "Internal error".to_string())?
    .map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "curl not found. Please install curl or check your PATH.".to_string()
        } else {
            format!("Failed to test connection: {}", e)
        }
    })?;

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let lines: Vec<&str> = raw.trim().rsplitn(2, '\n').collect();

    let (status_str, body_text) = if lines.len() == 2 {
        (lines[0].trim(), lines[1])
    } else {
        (raw.trim(), "")
    };

    let status: u16 = status_str.parse().unwrap_or(0);

    if status == 0 {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.contains("Could not resolve host") || stderr.contains("getaddrinfo") {
            return Err("Unable to connect. Please check the URL and your network.".to_string());
        }
        if stderr.contains("timed out") || stderr.contains("Timeout") {
            return Err("Connection timed out. The server may be slow or unreachable.".to_string());
        }
        if stderr.contains("SSL") || stderr.contains("certificate") {
            return Err("SSL/TLS error. The server's certificate may be invalid.".to_string());
        }
        return Err(
            "Connection failed. Please check your Base URL and network connection.".to_string(),
        );
    }

    if (200..300).contains(&status) {
        Ok("Connected successfully!".to_string())
    } else if status == 401 {
        Err("API Key is invalid or expired. Please check your key.".to_string())
    } else if status == 403 {
        Err("Access denied. Your API key may not have permission for this endpoint.".to_string())
    } else if status == 404 {
        Err("Endpoint not found. Please check the Base URL.".to_string())
    } else if status == 429 {
        Err("Rate limited. Too many requests — please wait and try again.".to_string())
    } else if status >= 500 {
        let msg = if body_text.contains("overloaded") || body_text.contains("负载") {
            "Server is overloaded. Please try again later."
        } else {
            "Server error. The API provider may be experiencing issues."
        };
        Err(msg.to_string())
    } else {
        Err(format!(
            "Unexpected response (HTTP {}). Please check your configuration.",
            status
        ))
    }
}
