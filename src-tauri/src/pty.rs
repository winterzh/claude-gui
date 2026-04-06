use log::info;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

use crate::config;
use crate::launcher::find_resources;

pub struct PtyState {
    writer: Option<Box<dyn Write + Send>>,
    master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn portable_pty::Child + Send>>,
}

impl PtyState {
    pub fn new() -> Self {
        Self {
            writer: None,
            master: None,
            child: None,
        }
    }
}

pub type SharedPtyState = Arc<Mutex<PtyState>>;

fn isolated_home() -> PathBuf {
    let dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude-launcher")
        .join("home");
    fs::create_dir_all(&dir).ok();
    fs::create_dir_all(dir.join(".claude")).ok();
    dir
}

/// Mirrors Claude Code's VX() function: replace all non-alphanumeric chars with "-".
fn path_to_slug(path: &str) -> String {
    path.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Windows: canonicalize a working directory path to a stable form.
/// - Uses fs::canonicalize for a real path
/// - Lowercases drive letter (C:\ -> c:\)
/// - Normalizes to forward slashes
#[cfg(target_os = "windows")]
fn canonicalize_windows_path(path: &str) -> String {
    let resolved = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string());

    let normalized = resolved.replace('\\', "/");

    // Lowercase drive letter: "C:/..." -> "c:/..."
    let mut chars: Vec<char> = normalized.chars().collect();
    if chars.len() >= 2 && chars[1] == ':' && chars[0].is_ascii_uppercase() {
        chars[0] = chars[0].to_ascii_lowercase();
    }
    chars.into_iter().collect()
}

/// Write config with onboarding complete, workspace trust, and API key approval.
/// Claude Code prefers .claude/.config.json if it exists, otherwise .claude.json.
fn write_claude_config(home_dir: &PathBuf, working_dir: &str, api_key: &str) {
    let config_json = home_dir.join(".claude").join(".config.json");
    let claude_json = home_dir.join(".claude.json");
    let active_path = if config_json.exists() {
        &config_json
    } else {
        &claude_json
    };

    // Read existing config or start fresh
    let mut config: serde_json::Value = fs::read_to_string(active_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let obj = config.as_object_mut().unwrap();

    // Skip onboarding + simulate proper install
    obj.insert("hasCompletedOnboarding".into(), serde_json::json!(true));
    obj.insert("theme".into(), serde_json::json!("dark"));
    if !obj.contains_key("installMethod") {
        obj.insert("installMethod".into(), serde_json::json!("native"));
    }

    // Pre-approve API key (hash first 20 chars like Claude Code does)
    if !api_key.is_empty() {
        let key_hash = if api_key.len() >= 20 {
            &api_key[api_key.len() - 20..]
        } else {
            api_key
        };
        let approved = obj
            .entry("customApiKeyResponses")
            .or_insert(serde_json::json!({"approved":[],"rejected":[]}));
        if let Some(arr) = approved.get_mut("approved").and_then(|a| a.as_array_mut()) {
            if !arr.iter().any(|v| v.as_str() == Some(key_hash)) {
                arr.push(serde_json::json!(key_hash));
            }
        }
    }

    // Pre-trust working directory (both slash variants for Windows)
    if !working_dir.is_empty() {
        let projects = obj.entry("projects").or_insert(serde_json::json!({}));
        let mut paths_to_trust = vec![working_dir.replace('\\', "/")];
        if working_dir.contains('\\') {
            paths_to_trust.push(working_dir.to_string());
        }
        if let Some(proj_map) = projects.as_object_mut() {
            for wd in &paths_to_trust {
                let project = proj_map.entry(wd).or_insert(serde_json::json!({}));
                if let Some(p) = project.as_object_mut() {
                    p.insert("hasTrustDialogAccepted".into(), serde_json::json!(true));
                }
            }
        }
    }

    fs::write(
        active_path,
        serde_json::to_string_pretty(&config).unwrap_or_default(),
    )
    .ok();
}

/// When base_url contains "minimax" (case-insensitive), write MiniMax MCP
/// server config so Claude Code can use web_search and understand_image tools.
/// When base_url does NOT contain "minimax", remove the MiniMax MCP entry.
///
/// Claude Code picks its config from whichever file exists first:
///   1. $HOME/.claude/.config.json  (preferred if present)
///   2. $HOME/.claude.json          (fallback)
/// We must update the ACTIVE file and clean up ALL possible locations.
fn configure_minimax_mcp(
    home_dir: &PathBuf,
    base_url: &str,
    api_key: &str,
    resources: &Option<PathBuf>,
) {
    let config_json = home_dir.join(".claude").join(".config.json");
    let claude_json = home_dir.join(".claude.json");

    // Claude Code prefers .claude/.config.json if it exists
    let active_path = if config_json.exists() {
        &config_json
    } else {
        &claude_json
    };

    let is_minimax = base_url.to_lowercase().contains("minimax");

    // Debug log for troubleshooting MCP activation issues
    let debug_path = home_dir.join(".mcp-debug.log");
    let debug_info = format!(
        "=== MCP Debug ===\nbase_url: {}\nis_minimax: {}\nconfig_json exists: {}\nclaude_json exists: {}\nactive_path: {:?}\n\n",
        base_url,
        is_minimax,
        config_json.exists(),
        claude_json.exists(),
        active_path,
    );
    {
        use std::io::Write as _;
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&debug_path)
        {
            let _ = f.write_all(debug_info.as_bytes());
        }
    }

    let mut config: serde_json::Value = fs::read_to_string(active_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let obj = config.as_object_mut().unwrap();

    if is_minimax {
        let servers = obj.entry("mcpServers").or_insert(serde_json::json!({}));

        // Determine the uv executable path — prefer bundled, fall back to system uvx
        let uv_exe = if cfg!(target_os = "windows") {
            resources
                .as_ref()
                .map(|res| res.join("uv").join("uv.exe"))
                .filter(|p| p.exists())
                .map(|p| p.to_string_lossy().to_string().replace('\\', "/"))
        } else {
            resources
                .as_ref()
                .map(|res| res.join("uv").join("bin").join("uv"))
                .filter(|p| p.exists())
                .map(|p| p.to_string_lossy().to_string())
        };

        let (cmd, args) = match uv_exe {
            Some(path) => (path, vec!["tool", "run", "minimax-coding-plan-mcp"]),
            None => ("uvx".to_string(), vec!["minimax-coding-plan-mcp"]),
        };

        let mcp_entry = serde_json::json!({
            "command": cmd,
            "args": args,
            "env": {
                "MINIMAX_API_KEY": api_key,
                "MINIMAX_API_HOST": "https://api.minimaxi.com"
            }
        });

        if let Some(server_map) = servers.as_object_mut() {
            server_map.insert("MiniMax".to_string(), mcp_entry);
        }
        info!("[mcp] MiniMax MCP enabled in {:?}", active_path);
    } else {
        // Remove MiniMax MCP from the active config
        if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
            servers.remove("MiniMax");
        }
        // Remove the mcpServers key entirely if it's empty
        if obj
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .map(|m| m.is_empty())
            .unwrap_or(false)
        {
            obj.remove("mcpServers");
        }
        info!("[mcp] MiniMax MCP removed from {:?}", active_path);
    }

    fs::write(
        active_path,
        serde_json::to_string_pretty(&config).unwrap_or_default(),
    )
    .ok();

    // Also clean up MiniMax MCP from ALL other config files to prevent stale entries
    let other_paths = [
        &config_json,
        &claude_json,
        &home_dir.join(".claude").join("settings.json"),
    ];
    for path in &other_paths {
        if *path == active_path || !path.exists() {
            continue;
        }
        if let Ok(text) = fs::read_to_string(path) {
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&text) {
                let mut changed = false;
                if let Some(obj) = val.as_object_mut() {
                    if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut())
                    {
                        if servers.remove("MiniMax").is_some() {
                            changed = true;
                        }
                    }
                    // Remove empty mcpServers
                    if obj
                        .get("mcpServers")
                        .and_then(|v| v.as_object())
                        .map(|m| m.is_empty())
                        .unwrap_or(false)
                    {
                        obj.remove("mcpServers");
                        changed = true;
                    }
                }
                if changed {
                    fs::write(path, serde_json::to_string_pretty(&val).unwrap_or_default()).ok();
                    info!("[mcp] cleaned up MiniMax MCP from {:?}", path);
                }
            }
        }
    }
}

fn merge_json(dst: &mut serde_json::Value, src: &serde_json::Value) {
    match (dst, src) {
        (serde_json::Value::Object(dst_obj), serde_json::Value::Object(src_obj)) => {
            for (key, value) in src_obj {
                match dst_obj.get_mut(key) {
                    Some(existing) => merge_json(existing, value),
                    None => {
                        dst_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (dst_value, src_value) => *dst_value = src_value.clone(),
    }
}

/// Windows-only: sync recent-activity history from real ~/.claude into the
/// isolated home so the "Recent activity" panel is populated.
/// Focuses on ensuring the current working directory's project slug is present.
/// macOS must NOT call this — it risks triggering system permission popups.
#[cfg(target_os = "windows")]
fn sync_user_history_for_windows(home_dir: &PathBuf, working_dir: &str) {
    let Some(real_home) = dirs::home_dir() else {
        return;
    };
    let src_claude = real_home.join(".claude");
    if !src_claude.exists() {
        return;
    }
    let dst_claude = home_dir.join(".claude");
    let src_projects = src_claude.join("projects");
    let dst_projects = dst_claude.join("projects");
    fs::create_dir_all(&dst_projects).ok();

    let target_slug = path_to_slug(working_dir);
    info!(
        "[history-sync] raw working_dir={}, target_slug={}",
        working_dir, target_slug
    );

    // Step 1: Copy all project directories from real ~/.claude/projects (shallow)
    if src_projects.is_dir() {
        if let Ok(entries) = fs::read_dir(&src_projects) {
            for entry in entries.flatten() {
                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let dst_entry = dst_projects.join(entry.file_name());
                if dst_entry.exists() {
                    continue;
                }
                // Copy directory contents (one level deep — session .jsonl files)
                fs::create_dir_all(&dst_entry).ok();
                if let Ok(files) = fs::read_dir(entry.path()) {
                    for f in files.flatten() {
                        if f.file_type().map(|t| t.is_file()).unwrap_or(false) {
                            fs::copy(f.path(), dst_entry.join(f.file_name())).ok();
                        }
                    }
                }
            }
        }
    }

    // Step 2: Ensure target_slug directory exists — find case-insensitive or
    // drive-letter alias candidates if exact match is missing
    let target_dir = dst_projects.join(&target_slug);
    if !target_dir.exists() {
        info!("[history-sync] target_slug dir missing, searching candidates...");
        let target_lower = target_slug.to_lowercase();

        // Build a drive-letter alias: c-... <-> C-...
        let alias_slug = {
            let chars: Vec<char> = target_slug.chars().collect();
            if chars.len() >= 2 && chars[1] == '-' && chars[0].is_ascii_alphabetic() {
                let flipped = if chars[0].is_ascii_lowercase() {
                    chars[0].to_ascii_uppercase()
                } else {
                    chars[0].to_ascii_lowercase()
                };
                let mut s = String::with_capacity(target_slug.len());
                s.push(flipped);
                s.push_str(&target_slug[1..]);
                Some(s)
            } else {
                None
            }
        };

        // Search both src and dst projects directories for a candidate
        let search_dirs = [&src_projects, &dst_projects];
        let mut found_candidate: Option<PathBuf> = None;

        for search_dir in &search_dirs {
            if !search_dir.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(search_dir) {
                for entry in entries.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Exact alias match (drive letter case flip)
                    if let Some(ref alias) = alias_slug {
                        if name == *alias {
                            info!("[history-sync] found alias candidate: {}", name);
                            found_candidate = Some(entry.path());
                            break;
                        }
                    }
                    // Case-insensitive match
                    if name.to_lowercase() == target_lower {
                        info!("[history-sync] found case-insensitive candidate: {}", name);
                        found_candidate = Some(entry.path());
                        break;
                    }
                }
            }
            if found_candidate.is_some() {
                break;
            }
        }

        // Copy candidate contents into the exact target_slug directory
        if let Some(candidate) = found_candidate {
            fs::create_dir_all(&target_dir).ok();
            if let Ok(files) = fs::read_dir(&candidate) {
                for f in files.flatten() {
                    if f.file_type().map(|t| t.is_file()).unwrap_or(false) {
                        let dst_file = target_dir.join(f.file_name());
                        if !dst_file.exists() {
                            fs::copy(f.path(), &dst_file).ok();
                        }
                    }
                }
            }
            info!(
                "[history-sync] populated target dir from candidate: {}",
                candidate.display()
            );
        } else {
            info!("[history-sync] no candidate found — recent activity will be empty for this project");
        }
    } else {
        info!("[history-sync] target_slug dir already exists");
    }

    info!(
        "[history-sync] dst projects dirs: {:?}",
        fs::read_dir(&dst_projects)
            .map(|rd| rd
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>())
            .unwrap_or_default()
    );
}

/// Keep the launcher's isolated HOME, but import the user's Claude settings
/// so proxy/certificate/network env stays available inside packaged builds.
fn sync_user_settings(home_dir: &PathBuf) {
    let Some(real_home) = dirs::home_dir() else {
        return;
    };
    let src = real_home.join(".claude").join("settings.json");
    if !src.exists() {
        return;
    }

    let Ok(source_text) = fs::read_to_string(&src) else {
        return;
    };
    let Ok(source_json) = serde_json::from_str::<serde_json::Value>(&source_text) else {
        return;
    };

    let dst_dir = home_dir.join(".claude");
    fs::create_dir_all(&dst_dir).ok();
    let dst = dst_dir.join("settings.json");

    let mut merged = if dst.exists() {
        fs::read_to_string(&dst)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    merge_json(&mut merged, &source_json);

    if let Some(env) = merged.get_mut("env").and_then(|v| v.as_object_mut()) {
        for key in [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_AUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
        ] {
            env.remove(key);
        }
    }

    fs::write(
        &dst,
        serde_json::to_string_pretty(&merged).unwrap_or_default(),
    )
    .ok();
}

/// Build a wrapper script that sets env vars then launches Claude Code.
/// This is more reliable than CommandBuilder::env() on Windows.
fn build_launch_script(
    cfg: &config::AppConfig,
    vhome: &str,
    working_dir: &str,
    skip_perms: bool,
    resources: &Option<PathBuf>,
) -> Result<PathBuf, String> {
    let home_dir = isolated_home();

    if cfg!(target_os = "windows") {
        // Use TEMP dir for the bat file to avoid path issues
        let temp_dir = std::env::var("TEMP")
            .or_else(|_| std::env::var("TMP"))
            .unwrap_or_else(|_| home_dir.to_string_lossy().to_string());
        let script_path = PathBuf::from(temp_dir).join("claude_launch.bat");

        // Determine claude command
        let claude_cmd = if let Some(ref res) = resources {
            let node = res.join("node").join("node.exe");
            let cli = res
                .join("claude-code")
                .join("node_modules")
                .join("@anthropic-ai")
                .join("claude-code")
                .join("cli.js");
            if node.exists() && cli.exists() {
                format!(
                    "\"{}\" \"{}\"",
                    node.to_string_lossy(),
                    cli.to_string_lossy()
                )
            } else {
                "npx @anthropic-ai/claude-code".to_string()
            }
        } else {
            "npx @anthropic-ai/claude-code".to_string()
        };

        // Build bat lines
        let mut lines: Vec<String> = vec!["@echo off".to_string()];
        lines.push(format!("set \"HOME={}\"", vhome));
        lines.push(format!("set \"USERPROFILE={}\"", vhome));
        lines.push(format!("set \"ANTHROPIC_API_KEY={}\"", cfg.api_key));
        lines.push(format!("set \"ANTHROPIC_BASE_URL={}\"", cfg.base_url));
        lines.push("set \"FORCE_COLOR=1\"".to_string());
        lines.push("set \"TERM=xterm-256color\"".to_string());

        // PATH with bundled git and node
        if let Some(ref res) = resources {
            let mut extra: Vec<String> = Vec::new();
            for sub in &["git\\cmd", "git\\usr\\bin", "git\\bin", "node"] {
                let p = res.join(sub);
                if p.exists() {
                    extra.push(p.to_string_lossy().to_string());
                }
            }
            if !extra.is_empty() {
                lines.push(format!("set \"PATH={};%PATH%\"", extra.join(";")));
            }
        }

        lines.push(format!("cd /d \"{}\"", working_dir));
        lines.push(claude_cmd);

        let bat = lines.join("\r\n") + "\r\n";
        fs::write(&script_path, &bat).map_err(|e| e.to_string())?;
        Ok(script_path)
    } else {
        let script_path = home_dir.join("_pty_launch.sh");

        let claude_cmd = if let Some(ref res) = resources {
            let node = res.join("node").join("bin").join("node");
            let cli = res
                .join("claude-code")
                .join("node_modules")
                .join("@anthropic-ai")
                .join("claude-code")
                .join("cli.js");
            if node.exists() && cli.exists() {
                format!("'{}' '{}'", node.to_string_lossy(), cli.to_string_lossy())
            } else {
                "npx @anthropic-ai/claude-code".to_string()
            }
        } else {
            "npx @anthropic-ai/claude-code".to_string()
        };

        let skip_flag = if skip_perms {
            " --dangerously-skip-permissions"
        } else {
            ""
        };

        // Save real HOME for reference, then override
        let real_home = dirs::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Collect bundled resource paths for PATH
        let mut extra_paths = Vec::new();
        if let Some(ref res) = resources {
            let node_bin = res.join("node").join("bin");
            if node_bin.exists() {
                extra_paths.push(node_bin.to_string_lossy().to_string());
            }
        }

        let sh = format!(
            r#"#!/bin/bash
# === Isolated Claude Code Environment ===
# Override HOME for Claude Code config isolation
export REAL_HOME='{real_home}'
export HOME='{vhome}'

# API configuration
export ANTHROPIC_API_KEY='{api_key}'
export ANTHROPIC_BASE_URL='{base_url}'

# Terminal
export FORCE_COLOR=1
export TERM=xterm-256color

# Ensure system tools are accessible (macOS GUI apps have minimal PATH)
export PATH="{extra_path}/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"

# Preserve system SSL certificates
export SSL_CERT_FILE=/etc/ssl/cert.pem
export NODE_EXTRA_CA_CERTS=/etc/ssl/cert.pem

# Preserve system temp directory
export TMPDIR="${{TMPDIR:-/tmp}}"

# Copy essential dotfiles to isolated home if missing
[ -f "$REAL_HOME/.gitconfig" ] && [ ! -f "$HOME/.gitconfig" ] && cp "$REAL_HOME/.gitconfig" "$HOME/.gitconfig" 2>/dev/null

cd '{working_dir}'
{claude_cmd}{skip_flag}
"#,
            real_home = real_home.replace("'", "'\\''"),
            vhome = vhome.replace("'", "'\\''"),
            api_key = cfg.api_key.replace("'", "'\\''"),
            base_url = cfg.base_url.replace("'", "'\\''"),
            extra_path = if extra_paths.is_empty() {
                String::new()
            } else {
                format!("{}:", extra_paths.join(":"))
            },
            working_dir = working_dir.replace("'", "'\\''"),
            claude_cmd = claude_cmd,
            skip_flag = skip_flag,
        );
        fs::write(&script_path, &sh).map_err(|e| e.to_string())?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).ok();
        }

        Ok(script_path)
    }
}

#[tauri::command]
pub fn spawn_claude(app: AppHandle, state: tauri::State<'_, SharedPtyState>) -> Result<(), String> {
    // Gracefully stop any existing session before starting a new one
    {
        let mut pty = state.lock().map_err(|e| e.to_string())?;
        graceful_shutdown(&mut pty);
    }

    let cfg = config::load_config()?.ok_or("No config found")?;
    let skip_perms = cfg.skip_permissions;
    let vhome = isolated_home();
    let vhome_str = vhome.to_string_lossy().to_string();

    let raw_working_dir = if cfg.working_dir.is_empty() {
        dirs::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    } else {
        cfg.working_dir.clone()
    };

    // Validate working directory exists
    if !std::path::Path::new(&raw_working_dir).exists() {
        return Err(
            "Working directory does not exist. Please choose a valid directory in Settings."
                .to_string(),
        );
    }

    // On Windows, canonicalize to stabilize drive letter case and slash direction.
    // This ensures the project slug matches what Claude Code will compute from cwd.
    #[cfg(target_os = "windows")]
    let working_dir = canonicalize_windows_path(&raw_working_dir);
    #[cfg(not(target_os = "windows"))]
    let working_dir = raw_working_dir;

    info!(
        "[spawn] working_dir={}, slug={}",
        working_dir,
        path_to_slug(&working_dir)
    );

    // Windows: sync recent-activity history from real ~/.claude so the
    // "Recent activity" panel is populated.  macOS skips this entirely.
    #[cfg(target_os = "windows")]
    sync_user_history_for_windows(&vhome, &working_dir);

    // Pre-configure .claude.json: skip onboarding, trust workspace, approve API key
    write_claude_config(&vhome, &working_dir, &cfg.api_key);

    let resources = find_resources();

    // Auto-configure MiniMax MCP when using MiniMaxi relay
    configure_minimax_mcp(&vhome, &cfg.base_url, &cfg.api_key, &resources);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    // On Windows: write a Node.js wrapper that sets process.env then runs Claude Code.
    // This bypasses all OS/PTY env var issues entirely.
    let mut cmd = if cfg!(target_os = "windows") {
        let res = resources.as_ref().ok_or("Resources directory not found")?;
        let node = res.join("node").join("node.exe");
        let cli = res
            .join("claude-code")
            .join("node_modules")
            .join("@anthropic-ai")
            .join("claude-code")
            .join("cli.js");

        if !node.exists() {
            return Err("Claude Code resources are missing. Please reinstall the app.".to_string());
        }
        if !cli.exists() {
            return Err(
                "Claude Code is not properly installed. Please reinstall the app.".to_string(),
            );
        }

        // Build PATH with bundled git/node/uv
        let mut extra_paths: Vec<String> = Vec::new();
        for sub in &["git\\cmd", "git\\usr\\bin", "git\\bin", "node", "uv"] {
            let p = res.join(sub);
            if p.exists() {
                extra_paths.push(p.to_string_lossy().to_string().replace('\\', "/"));
            }
        }
        let sys_path = std::env::var("PATH").unwrap_or_default().replace('\\', "/");
        extra_paths.push(sys_path);
        let full_path = extra_paths.join(";");

        // Write a JS wrapper that sets env then requires Claude Code directly
        // No spawn — runs in the same process, avoids all path issues
        let wrapper_path = vhome.join("_wrapper.js");
        let cli_escaped = cli.to_string_lossy().replace('\\', "\\\\");

        // Preserve essential system vars for network/SSL/DNS
        let sys_root = std::env::var("SystemRoot")
            .unwrap_or_else(|_| "C:\\Windows".to_string())
            .replace('\\', "/");
        let sys_temp = std::env::var("TEMP")
            .unwrap_or_else(|_| format!("{}\\Temp", sys_root))
            .replace('\\', "/");

        let wrapper_js = format!(
            r#"// === Isolated Claude Code Environment ===
// Override HOME/USERPROFILE for config isolation
process.env.HOME = {home};
process.env.USERPROFILE = {home};

// API configuration
process.env.ANTHROPIC_API_KEY = {key};
process.env.ANTHROPIC_BASE_URL = {url};

// Terminal
process.env.FORCE_COLOR = "1";
process.env.TERM = "xterm-256color";

// System paths (bundled git/node + system PATH)
process.env.PATH = {path};
delete process.env.CLAUDE_CODE_GIT_BASH_PATH;

// Preserve system essentials for network/SSL/DNS
process.env.SystemRoot = process.env.SystemRoot || {sys_root};
process.env.TEMP = process.env.TEMP || {sys_temp};
process.env.TMP = process.env.TMP || {sys_temp};

try {{ process.chdir({cwd}); }} catch(e) {{}}
{skip_args}
require("{cli}");
"#,
            home = serde_json::to_string(&vhome_str.replace('\\', "/")).unwrap(),
            key = serde_json::to_string(&cfg.api_key).unwrap(),
            url = serde_json::to_string(&cfg.base_url).unwrap(),
            path = serde_json::to_string(&full_path).unwrap(),
            sys_root = serde_json::to_string(&sys_root).unwrap(),
            sys_temp = serde_json::to_string(&sys_temp).unwrap(),
            cwd = serde_json::to_string(&working_dir.replace('\\', "/")).unwrap(),
            skip_args = if skip_perms {
                "process.argv.push('--dangerously-skip-permissions');"
            } else {
                ""
            },
            cli = cli_escaped,
        );
        fs::write(&wrapper_path, &wrapper_js).map_err(|e| e.to_string())?;

        let mut c = CommandBuilder::new(&node);
        c.arg(&wrapper_path);
        c
    } else {
        // macOS/Linux: spawn node directly, set env vars via CommandBuilder::env()
        // This preserves the system environment (SSL, DNS, etc.) while only overriding
        // what Claude Code needs. Unlike bash script approach, this doesn't break
        // network by changing HOME at the shell level.
        let (node_bin, cli_js) = if let Some(ref res) = resources {
            let n = res.join("node").join("bin").join("node");
            let c = res
                .join("claude-code")
                .join("node_modules")
                .join("@anthropic-ai")
                .join("claude-code")
                .join("cli.js");
            if n.exists() && c.exists() {
                (n, c)
            } else {
                return Err(
                    "Claude Code resources are missing. Please reinstall the app.".to_string(),
                );
            }
        } else {
            return Err("Resources directory not found. Please reinstall the app.".to_string());
        };

        let mut c = CommandBuilder::new(&node_bin);
        c.arg(&cli_js);
        if skip_perms {
            c.arg("--dangerously-skip-permissions");
        }

        // Set only what Claude Code needs — system env (SSL, DNS, PATH) stays intact
        c.env("HOME", &vhome_str);
        c.env("ANTHROPIC_API_KEY", &cfg.api_key);
        c.env("ANTHROPIC_BASE_URL", &cfg.base_url);
        c.env("FORCE_COLOR", "1");
        c.env("TERM", "xterm-256color");

        // Ensure bundled node + uv are in PATH alongside system tools
        if let Some(ref res) = resources {
            let node_bin_dir = res.join("node").join("bin");
            let uv_bin_dir = res.join("uv").join("bin");
            let sys_path = std::env::var("PATH").unwrap_or_default();
            // Add common macOS tool paths that GUI apps might miss
            c.env(
                "PATH",
                format!(
                    "{}:{}:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:{}",
                    node_bin_dir.to_string_lossy(),
                    uv_bin_dir.to_string_lossy(),
                    sys_path
                ),
            );
        }

        // Ensure SSL works with system certificates
        c.env("SSL_CERT_FILE", "/etc/ssl/cert.pem");
        c.env("NODE_EXTRA_CA_CERTS", "/etc/ssl/cert.pem");

        c
    };
    cmd.cwd(&working_dir);

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave);

    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

    {
        let mut pty = state.lock().map_err(|e| e.to_string())?;
        pty.writer = Some(writer);
        pty.master = Some(pair.master);
        pty.child = Some(child);
    }

    let app_clone = app.clone();
    let state_clone = Arc::clone(&state.inner());
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    app_clone.emit("pty-output", text).ok();
                }
            }
        }
        let mut pty = state_clone.lock().unwrap();
        let code = pty
            .child
            .as_mut()
            .and_then(|c| c.try_wait().ok().flatten())
            .map(|s| s.exit_code())
            .unwrap_or(0);
        app_clone.emit("pty-exit", code).ok();
    });

    Ok(())
}

#[tauri::command]
pub fn pty_write(data: String, state: tauri::State<'_, SharedPtyState>) -> Result<(), String> {
    let mut pty = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut w) = pty.writer {
        w.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
        w.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn pty_resize(
    cols: u16,
    rows: u16,
    state: tauri::State<'_, SharedPtyState>,
) -> Result<(), String> {
    let pty = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref master) = pty.master {
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Try graceful shutdown first (send "exit\n" + short wait), then force-kill.
fn graceful_shutdown(pty: &mut PtyState) {
    // Try sending "exit\n" through the PTY writer
    if let Some(ref mut w) = pty.writer {
        let _ = w.write_all(b"\nexit\n");
        let _ = w.flush();
    }

    // Give the process a moment to exit on its own
    if let Some(ref mut child) = pty.child {
        for _ in 0..20 {
            // 20 × 50ms = 1s max
            match child.try_wait() {
                Ok(Some(_)) => break, // exited
                Ok(None) => {}        // still running
                Err(_) => break,      // can't query — bail
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    // Force-kill if still alive, then clean up handles
    if let Some(mut child) = pty.child.take() {
        child.kill().ok();
    }
    pty.writer = None;
    pty.master = None;
}

#[tauri::command]
pub fn kill_claude(state: tauri::State<'_, SharedPtyState>) -> Result<(), String> {
    let mut pty = state.lock().map_err(|e| e.to_string())?;
    graceful_shutdown(&mut pty);
    Ok(())
}
