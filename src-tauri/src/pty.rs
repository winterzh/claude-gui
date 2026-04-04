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

/// Write .claude.json with onboarding complete, workspace trust, and API key approval
fn write_claude_config(home_dir: &PathBuf, working_dir: &str, api_key: &str) {
    let claude_json = home_dir.join(".claude.json");

    // Read existing config or start fresh
    let mut config: serde_json::Value = if claude_json.exists() {
        fs::read_to_string(&claude_json)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let obj = config.as_object_mut().unwrap();

    // Skip onboarding
    obj.insert("hasCompletedOnboarding".into(), serde_json::json!(true));
    obj.insert("theme".into(), serde_json::json!("dark"));

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
        &claude_json,
        serde_json::to_string_pretty(&config).unwrap_or_default(),
    )
    .ok();
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
    // Kill existing
    {
        let mut pty = state.lock().map_err(|e| e.to_string())?;
        if let Some(mut child) = pty.child.take() {
            child.kill().ok();
        }
        pty.writer = None;
        pty.master = None;
    }

    let cfg = config::load_config()?.ok_or("No config found")?;
    let skip_perms = cfg.skip_permissions;
    let vhome = isolated_home();
    let vhome_str = vhome.to_string_lossy().to_string();

    let working_dir = if cfg.working_dir.is_empty() {
        dirs::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    } else {
        cfg.working_dir.clone()
    };

    // Validate working directory exists
    if !std::path::Path::new(&working_dir).exists() {
        return Err(
            "Working directory does not exist. Please choose a valid directory in Settings."
                .to_string(),
        );
    }

    // Pre-configure .claude.json: skip onboarding, trust workspace, approve API key
    write_claude_config(&vhome, &working_dir, &cfg.api_key);
    sync_user_settings(&vhome);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let resources = find_resources();

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

        // Build PATH with bundled git/node
        let mut extra_paths: Vec<String> = Vec::new();
        for sub in &["git\\cmd", "git\\usr\\bin", "git\\bin", "node"] {
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

        // Ensure bundled node is in PATH alongside system tools
        if let Some(ref res) = resources {
            let node_bin_dir = res.join("node").join("bin");
            let sys_path = std::env::var("PATH").unwrap_or_default();
            // Add common macOS tool paths that GUI apps might miss
            c.env(
                "PATH",
                format!(
                    "{}:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:{}",
                    node_bin_dir.to_string_lossy(),
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

#[tauri::command]
pub fn kill_claude(state: tauri::State<'_, SharedPtyState>) -> Result<(), String> {
    let mut pty = state.lock().map_err(|e| e.to_string())?;
    if let Some(mut child) = pty.child.take() {
        child.kill().ok();
    }
    pty.writer = None;
    pty.master = None;
    Ok(())
}
