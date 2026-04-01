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
        let script_path = home_dir.join("_pty_launch.bat");

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

        // Build PATH additions
        let mut extra_paths = Vec::new();
        if let Some(ref res) = resources {
            let git_cmd = res.join("git").join("cmd");
            let git_usr_bin = res.join("git").join("usr").join("bin");
            let node_dir = res.join("node");
            if git_cmd.exists() { extra_paths.push(git_cmd.to_string_lossy().to_string()); }
            if git_usr_bin.exists() { extra_paths.push(git_usr_bin.to_string_lossy().to_string()); }
            if node_dir.exists() { extra_paths.push(node_dir.to_string_lossy().to_string()); }
        }
        let path_prefix = if extra_paths.is_empty() { String::new() } else { format!("set PATH={};%PATH%\n", extra_paths.join(";")) };

        // Find bash.exe
        let mut bash_line = String::new();
        if let Some(ref res) = resources {
            let candidates = [
                res.join("git").join("bin").join("bash.exe"),
                res.join("git").join("usr").join("bin").join("bash.exe"),
            ];
            for c in &candidates {
                if c.exists() {
                    bash_line = format!("set CLAUDE_CODE_GIT_BASH_PATH={}\n", c.to_string_lossy());
                    break;
                }
            }
        }

        let bat = format!(
            "@echo off\r\nset HOME={}\r\nset USERPROFILE={}\r\nset ANTHROPIC_API_KEY={}\r\nset ANTHROPIC_BASE_URL={}\r\nset FORCE_COLOR=1\r\nset TERM=xterm-256color\r\n{}{}\r\ncd /d \"{}\"\r\n{}\r\n",
            vhome, vhome, cfg.api_key, cfg.base_url,
            path_prefix.replace('\n', "\r\n"),
            bash_line.replace('\n', "\r\n"),
            working_dir,
            claude_cmd,
        );
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

    let mut cmd = if cfg!(target_os = "windows") {
        // On Windows: build one big inline command with set && set && ... && node cli.js
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

        // Build PATH with bundled git and node
        let mut path_parts: Vec<String> = Vec::new();
        if let Some(ref res) = resources {
            for sub in &["git\\cmd", "git\\usr\\bin", "git\\bin", "node"] {
                let p = res.join(sub);
                if p.exists() { path_parts.push(p.to_string_lossy().to_string()); }
            }
        }
        let sys_path = std::env::var("PATH").unwrap_or_default();
        path_parts.push(sys_path);
        let full_path = path_parts.join(";");

        // Find bash.exe
        let mut bash_set = String::new();
        if let Some(ref res) = resources {
            for sub in &["git\\bin\\bash.exe", "git\\usr\\bin\\bash.exe"] {
                let p = res.join(sub);
                if p.exists() {
                    bash_set = format!("set CLAUDE_CODE_GIT_BASH_PATH={} && ", p.to_string_lossy());
                    break;
                }
            }
        }

        let inline = format!(
            "set HOME={} && set USERPROFILE={} && set ANTHROPIC_API_KEY={} && set ANTHROPIC_BASE_URL={} && set PATH={} && {}set FORCE_COLOR=1 && set TERM=xterm-256color && cd /d \"{}\" && {}",
            vhome_str, vhome_str, cfg.api_key, cfg.base_url, full_path, bash_set, working_dir, claude_cmd,
        );

        let mut c = CommandBuilder::new("cmd");
        c.args(["/c", &inline]);
        c
    } else {
        // On macOS/Linux: use wrapper script
        let script_path = build_launch_script(&cfg, &vhome_str, &working_dir, &resources)?;
        let mut c = CommandBuilder::new("bash");
        c.arg(&script_path);
        c
    };
    cmd.cwd(&working_dir);

    // Also set env vars directly (belt and suspenders)
    cmd.env("HOME", &vhome_str);
    cmd.env("ANTHROPIC_API_KEY", &cfg.api_key);
    cmd.env("ANTHROPIC_BASE_URL", &cfg.base_url);
    cmd.env("FORCE_COLOR", "1");
    cmd.env("TERM", "xterm-256color");

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
