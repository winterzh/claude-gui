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
        .join(env!("ISOLATION_DIR"))
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
    // We spawn the bundled binary directly — Claude Code's self-update has
    // nothing to manage for us. Removing installMethod silences the
    // "method is native, but claude command not found" warning.
    obj.remove("installMethod");

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

    let mut config: serde_json::Value = fs::read_to_string(active_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let obj = config.as_object_mut().unwrap();

    if is_minimax {
        let servers = obj.entry("mcpServers").or_insert(serde_json::json!({}));

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
            Some(ref path) => (
                path.as_str(),
                vec!["tool", "run", "minimax-coding-plan-mcp"],
            ),
            None => ("uvx", vec!["minimax-coding-plan-mcp"]),
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

    // Claude Code 2.1+ ships as a native single-file binary at
    // claude-code/node_modules/@anthropic-ai/claude-code/bin/claude.exe (the
    // .exe name is preserved on every OS). We spawn it directly with env vars.
    let res = resources.as_ref().ok_or_else(|| {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        format!("find_resources returned None. exe_dir={:?}", exe_dir)
    })?;
    let claude_bin = res.join(crate::launcher::claude_binary_rel());
    if !claude_bin.exists() {
        return Err(format!(
            "Claude Code binary missing at {:?}. Please reinstall the app.",
            claude_bin
        ));
    }

    let mut cmd = CommandBuilder::new(&claude_bin);
    if skip_perms {
        cmd.arg("--dangerously-skip-permissions");
    }

    // Resolve active profile (if any) — its fields override top-level config.
    let active_profile = if cfg.active_profile.is_empty() {
        None
    } else {
        cfg.profiles.iter().find(|p| p.name == cfg.active_profile)
    };

    // Set exactly ONE auth env var to avoid Claude Code's "Auth conflict"
    // warning. Profile-defined auth_env wins; default ANTHROPIC_API_KEY.
    let auth_env_name = active_profile
        .map(|p| p.auth_env.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("ANTHROPIC_API_KEY");
    cmd.env(auth_env_name, &cfg.api_key);

    cmd.env("ANTHROPIC_BASE_URL", &cfg.base_url);

    // Model: profile.model wins over legacy top-level cfg.model.
    let model_to_use = active_profile
        .map(|p| p.model.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| cfg.model.trim());
    if !model_to_use.is_empty() {
        cmd.env("ANTHROPIC_MODEL", model_to_use);
    }

    cmd.env("FORCE_COLOR", "1");
    cmd.env("TERM", "xterm-256color");

    // Apply the active profile's extra_env (preset env bundle).
    // Skip auth-related keys — those are already managed above from api_key.
    if let Some(active) = active_profile {
        for (k, v) in &active.extra_env {
            if k.is_empty() { continue; }
            if matches!(k.as_str(), "ANTHROPIC_API_KEY" | "ANTHROPIC_AUTH_TOKEN") {
                continue;
            }
            cmd.env(k, v);
        }
    }

    if cfg!(target_os = "windows") {
        cmd.env("HOME", &vhome_str);
        cmd.env("USERPROFILE", &vhome_str);
        // Build PATH with bundled git/node/uv alongside system PATH
        let mut extra: Vec<String> = Vec::new();
        for sub in &["git\\cmd", "git\\usr\\bin", "git\\bin", "node", "uv"] {
            let p = res.join(sub);
            if p.exists() {
                extra.push(p.to_string_lossy().to_string());
            }
        }
        let sys_path = std::env::var("PATH").unwrap_or_default();
        extra.push(sys_path);
        cmd.env("PATH", extra.join(";"));
    } else {
        cmd.env("HOME", &vhome_str);
        let node_bin_dir = res.join("node").join("bin");
        let uv_bin_dir = res.join("uv").join("bin");
        let sys_path = std::env::var("PATH").unwrap_or_default();
        cmd.env(
            "PATH",
            format!(
                "{}:{}:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:{}",
                node_bin_dir.to_string_lossy(),
                uv_bin_dir.to_string_lossy(),
                sys_path
            ),
        );
        cmd.env("SSL_CERT_FILE", "/etc/ssl/cert.pem");
        cmd.env("NODE_EXTRA_CA_CERTS", "/etc/ssl/cert.pem");
    }
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
