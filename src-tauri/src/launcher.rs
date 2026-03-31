use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::config;

fn isolated_home() -> Result<PathBuf, String> {
    let dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".claude-launcher")
        .join("home");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join(".claude")).ok();
    Ok(dir)
}

/// Find bundled resources directory
fn find_resources() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))?;

    let candidates = [
        exe_dir.join("resources"),
        exe_dir.join("../Resources/resources"), // macOS .app bundle
        exe_dir.join("../../resources"),         // dev: target/debug/
        exe_dir.join("../../../src-tauri/resources"), // dev
        PathBuf::from("src-tauri/resources"),    // CWD
    ];

    for c in &candidates {
        if c.join("node").exists() && c.join("claude-code").exists() {
            return Some(c.clone());
        }
    }
    None
}

#[tauri::command]
pub fn launch_claude_code() -> Result<(), String> {
    let cfg = config::load_config()?.ok_or("No config found. Please configure first.")?;

    let working_dir = if cfg.working_dir.is_empty() {
        dirs::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    } else {
        cfg.working_dir.clone()
    };

    let vhome = isolated_home()?;
    let vhome_str = vhome.to_string_lossy().to_string();
    let resources = find_resources();

    // Build the claude command depending on whether we have bundled resources
    let claude_cmd = if let Some(ref res) = resources {
        if cfg!(target_os = "windows") {
            let node = res.join("node").join("node.exe");
            let cli = res.join("claude-code").join("node_modules").join("@anthropic-ai").join("claude-code").join("cli.js");
            format!("\"{}\" \"{}\"", node.to_string_lossy(), cli.to_string_lossy())
        } else {
            let node = res.join("node").join("bin").join("node");
            let cli = res.join("claude-code").join("node_modules").join("@anthropic-ai").join("claude-code").join("cli.js");
            format!("'{}' '{}'", node.to_string_lossy(), cli.to_string_lossy())
        }
    } else {
        // Fallback: use system npx
        "npx @anthropic-ai/claude-code".to_string()
    };

    if cfg!(target_os = "macos") {
        let script_path = vhome.join("_launch.sh");
        let script = format!(
            "#!/bin/bash\nexport HOME='{}'\nexport ANTHROPIC_API_KEY='{}'\nexport ANTHROPIC_BASE_URL='{}'\nexport TERM=xterm-256color\nexport TERMINFO=/usr/share/terminfo\ncd '{}'\nclear\n{}\n",
            vhome_str.replace("'", "'\\''"),
            cfg.api_key.replace("'", "'\\''"),
            cfg.base_url.replace("'", "'\\''"),
            working_dir.replace("'", "'\\''"),
            claude_cmd,
        );
        fs::write(&script_path, &script).map_err(|e| e.to_string())?;
        Command::new("chmod").args(["+x", &script_path.to_string_lossy()]).output().ok();
        Command::new("open")
            .args(["-a", "Terminal", &script_path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Failed to open Terminal: {}", e))?;
    } else if cfg!(target_os = "windows") {
        let script_path = vhome.join("_launch.sh");
        let script = format!(
            "#!/bin/bash\nexport HOME='{}'\nexport USERPROFILE='{}'\nexport ANTHROPIC_API_KEY='{}'\nexport ANTHROPIC_BASE_URL='{}'\nexport TERM=xterm-256color\nif [ -d '/usr/share/terminfo' ]; then export TERMINFO=/usr/share/terminfo; fi\ncd '{}'\nclear\n{}\n",
            vhome_str.replace('\\', "/"),
            vhome_str.replace('\\', "/"),
            cfg.api_key,
            cfg.base_url,
            working_dir.replace('\\', "/"),
            claude_cmd.replace('\\', "/"),
        );
        fs::write(&script_path, &script).map_err(|e| e.to_string())?;

        // Find bash.exe from Git for Windows
        let bash_paths = [
            r"C:\Program Files\Git\bin\bash.exe",
            r"C:\Program Files (x86)\Git\bin\bash.exe",
            r"C:\Git\bin\bash.exe",
        ];
        let bash = bash_paths.iter().find(|p| std::path::Path::new(p).exists());

        if let Some(bash_exe) = bash {
            // Open a new cmd window that runs bash with our script
            let script_unix = script_path.to_string_lossy().replace('\\', "/");
            Command::new("cmd")
                .args(["/c", "start", "", bash_exe, "--login", &script_unix])
                .spawn()
                .map_err(|e| format!("Failed to open Git Bash: {}", e))?;
        } else {
            return Err("Git Bash not found. Please install Git for Windows: https://git-scm.com/download/win".to_string());
        }
    } else {
        let script_path = vhome.join("_launch.sh");
        let script = format!(
            "#!/bin/bash\nexport HOME='{}'\nexport ANTHROPIC_API_KEY='{}'\nexport ANTHROPIC_BASE_URL='{}'\nexport TERM=xterm-256color\nexport TERMINFO=/usr/share/terminfo\ncd '{}'\n{}\n",
            vhome_str.replace("'", "'\\''"),
            cfg.api_key.replace("'", "'\\''"),
            cfg.base_url.replace("'", "'\\''"),
            working_dir.replace("'", "'\\''"),
            claude_cmd,
        );
        fs::write(&script_path, &script).map_err(|e| e.to_string())?;
        Command::new("chmod").args(["+x", &script_path.to_string_lossy()]).output().ok();

        let terminals = ["x-terminal-emulator", "gnome-terminal", "konsole", "xterm"];
        let mut launched = false;
        for term in &terminals {
            if Command::new(term).args(["-e", &script_path.to_string_lossy()]).spawn().is_ok() {
                launched = true;
                break;
            }
        }
        if !launched {
            return Err("No terminal emulator found".to_string());
        }
    }

    Ok(())
}
