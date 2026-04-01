use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs;
use std::path::PathBuf;
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
        Self { writer: None, master: None, child: None }
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

/// Build a wrapper script that sets env vars then launches Claude Code.
/// This is more reliable than CommandBuilder::env() on Windows.
fn build_launch_script(
    cfg: &config::AppConfig,
    vhome: &str,
    working_dir: &str,
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
            let cli = res.join("claude-code").join("node_modules")
                .join("@anthropic-ai").join("claude-code").join("cli.js");
            if node.exists() && cli.exists() {
                format!("\"{}\" \"{}\"", node.to_string_lossy(), cli.to_string_lossy())
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
                if p.exists() { extra.push(p.to_string_lossy().to_string()); }
            }
            if !extra.is_empty() {
                lines.push(format!("set \"PATH={};%PATH%\"", extra.join(";")));
            }
            // Find bash.exe
            for sub in &["git\\bin\\bash.exe", "git\\usr\\bin\\bash.exe"] {
                let p = res.join(sub);
                if p.exists() {
                    lines.push(format!("set \"CLAUDE_CODE_GIT_BASH_PATH={}\"", p.to_string_lossy()));
                    break;
                }
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
            let cli = res.join("claude-code").join("node_modules")
                .join("@anthropic-ai").join("claude-code").join("cli.js");
            if node.exists() && cli.exists() {
                format!("'{}' '{}'", node.to_string_lossy(), cli.to_string_lossy())
            } else {
                "npx @anthropic-ai/claude-code".to_string()
            }
        } else {
            "npx @anthropic-ai/claude-code".to_string()
        };

        let sh = format!(
            "#!/bin/bash\nexport HOME='{}'\nexport ANTHROPIC_API_KEY='{}'\nexport ANTHROPIC_BASE_URL='{}'\nexport FORCE_COLOR=1\nexport TERM=xterm-256color\ncd '{}'\n{}\n",
            vhome.replace("'", "'\\''"),
            cfg.api_key.replace("'", "'\\''"),
            cfg.base_url.replace("'", "'\\''"),
            working_dir.replace("'", "'\\''"),
            claude_cmd,
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
        if let Some(mut child) = pty.child.take() { child.kill().ok(); }
        pty.writer = None;
        pty.master = None;
    }

    let cfg = config::load_config()?.ok_or("No config found")?;
    let vhome = isolated_home();
    let vhome_str = vhome.to_string_lossy().to_string();

    let working_dir = if cfg.working_dir.is_empty() {
        dirs::home_dir().unwrap_or_default().to_string_lossy().to_string()
    } else {
        cfg.working_dir.clone()
    };

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    let resources = find_resources();

    // Build the command - on Windows spawn node.exe directly, on Unix use wrapper script
    let mut cmd = if cfg!(target_os = "windows") {
        let (node_exe, cli_js) = if let Some(ref res) = resources {
            let n = res.join("node").join("node.exe");
            let c = res.join("claude-code").join("node_modules")
                .join("@anthropic-ai").join("claude-code").join("cli.js");
            if n.exists() && c.exists() { (n, c) } else {
                return Err("Bundled node.exe or claude-code not found in resources".to_string());
            }
        } else {
            return Err("Resources directory not found".to_string());
        };

        let mut c = CommandBuilder::new(&node_exe);
        c.arg(&cli_js);

        // Set ALL env vars directly on the process
        c.env("HOME", &vhome_str);
        c.env("USERPROFILE", &vhome_str);
        c.env("ANTHROPIC_API_KEY", &cfg.api_key);
        c.env("ANTHROPIC_BASE_URL", &cfg.base_url);
        c.env("FORCE_COLOR", "1");
        c.env("TERM", "xterm-256color");

        // Add bundled git and node to PATH
        if let Some(ref res) = resources {
            let mut extra: Vec<String> = Vec::new();
            for sub in &["git\\cmd", "git\\usr\\bin", "git\\bin", "node"] {
                let p = res.join(sub);
                if p.exists() { extra.push(p.to_string_lossy().to_string()); }
            }
            let sys_path = std::env::var("PATH").unwrap_or_default();
            extra.push(sys_path);
            c.env("PATH", extra.join(";"));

            // bash.exe for Claude Code
            for sub in &["git\\bin\\bash.exe", "git\\usr\\bin\\bash.exe"] {
                let p = res.join(sub);
                if p.exists() {
                    c.env("CLAUDE_CODE_GIT_BASH_PATH", p.to_string_lossy().to_string());
                    break;
                }
            }
        }
        c
    } else {
        let script_path = build_launch_script(&cfg, &vhome_str, &working_dir, &resources)?;
        let mut c = CommandBuilder::new("bash");
        c.arg(&script_path);
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
        let code = pty.child.as_mut()
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
pub fn pty_resize(cols: u16, rows: u16, state: tauri::State<'_, SharedPtyState>) -> Result<(), String> {
    let pty = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref master) = pty.master {
        master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn kill_claude(state: tauri::State<'_, SharedPtyState>) -> Result<(), String> {
    let mut pty = state.lock().map_err(|e| e.to_string())?;
    if let Some(mut child) = pty.child.take() { child.kill().ok(); }
    pty.writer = None;
    pty.master = None;
    Ok(())
}
