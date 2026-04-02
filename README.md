# Claude Code Launcher

A lightweight desktop app that makes Claude Code easy to use on Windows. No terminal knowledge needed — just configure and click.

![Version](https://img.shields.io/github/v/release/winterzh/claude-gui)
![Downloads](https://img.shields.io/github/downloads/winterzh/claude-gui/total)

## Features

- **Embedded terminal** — Claude Code runs inside the app (xterm.js + PTY)
- **Zero dependencies** — Installer bundles Node.js, Git, and Claude Code. Nothing else to install
- **Multi-profile** — Save and switch between multiple API configurations
- **Connection test** — Verify your API key and endpoint before launching
- **Presets** — One-click setup for Anthropic, Pincc.ai, MiniMaxi
- **Connection status** — Real-time indicator on the home page
- **Isolated environment** — Separate config directory, won't touch your existing Claude Code setup
- **Dark / Light theme, Chinese / English**

## Quick Start

### 1. Download

Go to [Releases](https://github.com/winterzh/claude-gui/releases) and download the latest `.exe` installer.

### 2. Install

Run the installer. Everything is bundled — no need to install Node.js, Git, or anything else.

### 3. Configure

Open the app. You'll see the Settings page:

1. Choose a **preset** (Pincc.ai / MiniMaxi / Anthropic) or enter a custom Base URL
2. Enter your **API Key**
3. Click **Test Connection** to verify
4. Click **Save & Launch**

### 4. Launch

1. Choose a **working directory** (the project you want Claude Code to work on)
2. Click **Launch Claude Code**
3. Start coding!

## Presets

| Provider | Base URL | Notes |
|----------|----------|-------|
| Anthropic (Direct) | `https://api.anthropic.com` | Requires direct API access |
| Pincc.ai | `https://v2.pincc.ai` | China-friendly relay |
| MiniMaxi | `https://api.minimaxi.com/anthropic` | China-friendly relay |

You can also save custom configurations as profiles and switch between them.

## What's bundled

| Component | Purpose |
|-----------|---------|
| Node.js v22 | Runtime for Claude Code |
| PortableGit | Git operations + bash.exe |
| Claude Code | The CLI itself |

Total installer size: ~100MB.

## How it works

```
 User clicks "Launch Claude Code"
              |
              v
 ┌─────────────────────────────┐
 │  _wrapper.js                │
 │  Sets process.env:          │
 │    ANTHROPIC_API_KEY        │
 │    ANTHROPIC_BASE_URL       │
 │    HOME (isolated)          │
 │    PATH (bundled git+node)  │
 │  Spawns Claude Code CLI     │
 └──────────┬──────────────────┘
            |
            v
 ┌─────────────────────────────┐
 │  Embedded Terminal          │
 │  xterm.js <-> PTY <-> node │
 └─────────────────────────────┘
```

**Environment isolation:**
- Claude Code config: `~/.claude-launcher/home/.claude/` (not `~/.claude/`)
- `.claude.json` auto-created with `hasCompletedOnboarding: true`
- App config: `%APPDATA%/claude-launcher/config.json`

## Build from source

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) >= 1.70

### Development

```bash
npm install
npx tauri dev
```

### Release build

Push a version tag to trigger GitHub Actions:

```bash
git tag v0.4.0
git push origin v0.4.0
```

CI downloads Node.js + PortableGit + Claude Code, bundles into `.exe`, uploads to Releases.

## Troubleshooting

### "Unable to connect to Anthropic services"

The app auto-creates `.claude.json` with `hasCompletedOnboarding: true`. Without this file, Claude Code ignores your Base URL. If you see this error:

1. Uninstall the app
2. Delete `%LOCALAPPDATA%\Claude Code Launcher`
3. Delete `%USERPROFILE%\.claude-launcher`
4. Reinstall the latest version

### Terminal display issues

Slight delay when resizing is normal (debounced at 50ms).

### Connection test works but Claude Code doesn't connect

Make sure you're on the latest version. Older versions had issues with environment variable propagation on Windows.

## License

MIT
