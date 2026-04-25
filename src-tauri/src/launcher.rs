use std::path::PathBuf;
use std::process::Command;

/// Path to the bundled Claude Code executable (relative to a resources dir).
/// Since 2.1.x, `@anthropic-ai/claude-code` ships as a native binary; the
/// postinstall script copies the platform binary over `bin/claude.exe`. The
/// `.exe` name is preserved on every OS so the path stays cross-platform.
pub fn claude_binary_rel() -> PathBuf {
    PathBuf::from("claude-code")
        .join("node_modules")
        .join("@anthropic-ai")
        .join("claude-code")
        .join("bin")
        .join("claude.exe")
}

/// Find bundled resources directory.
/// Validates that the Claude Code native binary actually exists — not just
/// the directory shell. This prevents stale empty `target/debug/resources/`
/// from masking the real source dir.
pub fn find_resources() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))?;

    let candidates = [
        exe_dir.join("resources"),
        exe_dir.join("../Resources/resources"), // macOS .app bundle
        exe_dir.join("../../resources"),        // dev: target/debug/
        exe_dir.join("../../../src-tauri/resources"), // dev
        PathBuf::from("src-tauri/resources"),   // CWD
    ];

    let bin_rel = claude_binary_rel();

    for c in &candidates {
        if c.join(&bin_rel).exists() {
            return Some(c.clone());
        }
    }
    None
}

#[tauri::command]
pub async fn update_claude_code() -> Result<String, String> {
    let res = find_resources().ok_or("Resources directory not found")?;

    let node = if cfg!(target_os = "windows") {
        res.join("node").join("node.exe")
    } else {
        res.join("node").join("bin").join("node")
    };

    // npm shim ships as `bin/npm` on macOS but the require path inside it is
    // broken when bundled (it's a regular file, not the symlink the official
    // distro uses). Run npm-cli.js directly on every platform.
    let npm = if cfg!(target_os = "windows") {
        res.join("node")
            .join("node_modules")
            .join("npm")
            .join("bin")
            .join("npm-cli.js")
    } else {
        res.join("node")
            .join("lib")
            .join("node_modules")
            .join("npm")
            .join("bin")
            .join("npm-cli.js")
    };

    let claude_dir = res.join("claude-code");

    if !node.exists() {
        return Err("Claude Code resources are missing. Please reinstall the app.".to_string());
    }

    let node_clone = node.clone();
    let npm_clone = npm.clone();
    let claude_dir_clone = claude_dir.clone();

    // Force-install the latest version. `npm update` can refuse to bump past
    // semver ranges; `install ...@latest` always pulls the newest published.
    let (stdout, stderr, success) = tokio::task::spawn_blocking(move || {
        let output = Command::new(&node_clone)
            .args([
                npm_clone.to_string_lossy().as_ref(),
                "install",
                "@anthropic-ai/claude-code@latest",
                "--prefix",
                claude_dir_clone.to_string_lossy().as_ref(),
                "--no-audit",
                "--no-fund",
            ])
            .output();
        match output {
            Ok(o) => (
                String::from_utf8_lossy(&o.stdout).to_string(),
                String::from_utf8_lossy(&o.stderr).to_string(),
                o.status.success(),
            ),
            Err(e) => (String::new(), e.to_string(), false),
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    if !success {
        // Include the whole npm error output so we can actually see what broke.
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            "no output".to_string()
        };
        // Cap to ~800 chars so the dialog stays readable but actionable.
        let detail = if detail.chars().count() > 800 {
            format!("{}...", detail.chars().take(800).collect::<String>())
        } else {
            detail
        };
        return Err(format!("Update failed.\n{}", detail));
    }

    // Verify the new native binary actually runs. If postinstall skipped,
    // hit a network error, or the binary is corrupt, --version will fail
    // and we surface it instead of silently leaving a broken bundle.
    let claude_bin = res.join(claude_binary_rel());
    if !claude_bin.exists() {
        return Err(format!(
            "Update completed, but the Claude Code binary is missing at {:?}. \
             Postinstall (install.cjs) likely failed to copy the platform binary. \
             Check your network and try again.",
            claude_bin
        ));
    }

    let claude_bin_clone = claude_bin.clone();
    let verify = tokio::task::spawn_blocking(move || {
        let mut c = Command::new(&claude_bin_clone);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        c.arg("--version").output()
    })
    .await
    .map_err(|e| format!("Update completed, but verification thread failed: {}", e))?;

    let verify = match verify {
        Ok(o) => o,
        Err(e) => {
            return Err(format!(
                "Update completed, but the new binary failed to execute: {}. \
                 The bundle may be incomplete — try updating again.",
                e
            ));
        }
    };

    if !verify.status.success() {
        let err = String::from_utf8_lossy(&verify.stderr).trim().to_string();
        return Err(format!(
            "Update completed, but `claude --version` exited with {}. {}",
            verify.status.code().unwrap_or(-1),
            err
        ));
    }

    let version = String::from_utf8_lossy(&verify.stdout).trim().to_string();
    if version.is_empty() {
        Ok(format!("Updated. {}", stdout.trim()))
    } else {
        // `claude --version` prints something like "2.1.119 (Claude Code)"
        Ok(format!("Updated to {}", version))
    }
}
