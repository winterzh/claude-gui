use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
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
        Self { writer: None, master: None, child: None }
    }
}

pub type SharedPtyState = Arc<Mutex<PtyState>>;

fn isolated_home() -> String {
    let dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude-launcher")
        .join("home");
    std::fs::create_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join(".claude")).ok();
    dir.to_string_lossy().to_string()
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

    // Build command
    let mut cmd = if let Some(ref res) = resources {
        let node = if cfg!(target_os = "windows") {
            res.join("node").join("node.exe")
        } else {
            res.join("node").join("bin").join("node")
        };
        let cli = res.join("claude-code").join("node_modules")
            .join("@anthropic-ai").join("claude-code").join("cli.js");
        if node.exists() && cli.exists() {
            let mut c = CommandBuilder::new(&node);
            c.arg(&cli);
            c
        } else {
            let mut c = CommandBuilder::new(if cfg!(target_os = "windows") { "cmd" } else { "bash" });
            if cfg!(target_os = "windows") { c.args(["/c", "npx", "@anthropic-ai/claude-code"]); }
            else { c.args(["-c", "npx @anthropic-ai/claude-code"]); }
            c
        }
    } else {
        let mut c = CommandBuilder::new(if cfg!(target_os = "windows") { "cmd" } else { "bash" });
        if cfg!(target_os = "windows") { c.args(["/c", "npx", "@anthropic-ai/claude-code"]); }
        else { c.args(["-c", "npx @anthropic-ai/claude-code"]); }
        c
    };

    cmd.env("HOME", &vhome);
    cmd.env("ANTHROPIC_API_KEY", &cfg.api_key);
    cmd.env("ANTHROPIC_BASE_URL", &cfg.base_url);
    cmd.env("FORCE_COLOR", "1");
    cmd.env("TERM", "xterm-256color");
    cmd.cwd(&working_dir);

    if cfg!(target_os = "windows") {
        cmd.env("USERPROFILE", &vhome);
    }

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
