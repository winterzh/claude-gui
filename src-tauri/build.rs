use std::fs;
use std::path::Path;

const DEFAULT_ISOLATION_DIR: &str = ".claude-launcher";
const DEFAULT_CONFIG_DIR_NAME: &str = "claude-launcher";

fn main() {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("config-packaging")
        .join("config.json");

    let (isolation_dir, config_dir_name) = config_path
        .exists()
        .then(|| fs::read_to_string(&config_path).ok())
        .flatten()
        .and_then(|data| serde_json::from_str::<serde_json::Value>(&data).ok())
        .filter(|v| v.get("enabled") == Some(&serde_json::Value::Bool(true)))
        .map(|v| {
            let iso = v
                .pointer("/defaults/isolationDir")
                .and_then(|v| v.as_str())
                .unwrap_or(DEFAULT_ISOLATION_DIR)
                .to_string();
            let name = v
                .get("appSlug")
                .and_then(|v| v.as_str())
                .unwrap_or(DEFAULT_CONFIG_DIR_NAME)
                .to_string();
            (iso, name)
        })
        .unwrap_or_else(|| {
            (
                DEFAULT_ISOLATION_DIR.to_string(),
                DEFAULT_CONFIG_DIR_NAME.to_string(),
            )
        });

    println!("cargo:rustc-env=ISOLATION_DIR={}", isolation_dir);
    println!("cargo:rustc-env=CONFIG_DIR_NAME={}", config_dir_name);
    tauri_build::build()
}
