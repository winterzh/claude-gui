# Claude Code Launcher

A lightweight desktop app that makes Claude Code easy to use. No terminal knowledge needed — just configure and click.

## Features

- **Embedded terminal** — Claude Code runs inside the app with a built-in terminal (xterm.js)
- **Zero dependencies** — Windows installer bundles Node.js, Git, and Claude Code. Nothing else to install
- **Isolated environment** — Uses a separate config directory (`~/.claude-launcher/home/`), won't interfere with your existing Claude Code setup
- **Cross-platform** — Windows (installer) + macOS (dev)
- **Dark / Light theme**
- **Chinese / English**

## Download

Go to the [Releases](https://github.com/winterzh/claude-gui/releases) page and download the latest `.exe` installer for Windows.

## Usage

1. Install and open the app
2. Enter your **API Key** and **API Base URL** (proxy/relay endpoint)
3. Choose a **working directory**
4. Click **Launch Claude Code**
5. Claude Code runs in the embedded terminal — type and interact directly

## What's bundled (Windows)

| Component | Purpose |
|-----------|---------|
| Node.js v22 | Runtime for Claude Code |
| MinGit | Git operations (diff, status, etc.) |
| Claude Code | The CLI itself |

Total installer size: ~100MB. No internet needed after install (except for API calls).

## Build from source

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) >= 1.70

### Development

```bash
npm install
npx tauri dev
```

### Build for Windows

Push a version tag to trigger GitHub Actions CI:

```bash
git tag v0.2.1
git push origin v0.2.1
```

The CI downloads Node.js + MinGit + Claude Code, bundles everything into a Windows `.exe` installer, and uploads to GitHub Releases.

### Build locally (macOS)

```bash
bash scripts/prepare-resources.sh
npx tauri build
```

## Architecture

```
┌─────────────────────────────┐
│  Tauri App (Rust + React)   │
│  ┌───────────────────────┐  │
│  │  Settings Page        │  │
│  │  API Key + Base URL   │  │
│  └───────────────────────┘  │
│  ┌───────────────────────┐  │
│  │  Embedded Terminal    │  │
│  │  xterm.js + PTY      │  │
│  │  ┌─────────────────┐  │  │
│  │  │  Claude Code    │  │  │
│  │  │  (bundled)      │  │  │
│  │  └─────────────────┘  │  │
│  └───────────────────────┘  │
└─────────────────────────────┘
         │
         ▼ Isolated
  ~/.claude-launcher/home/.claude/
  (separate from ~/.claude/)
```

- Config stored in `~/.config/claude-launcher/config.json`
- Claude Code runs with `HOME` set to `~/.claude-launcher/home/`
- Your real `~/.claude/` is never touched

## License

MIT
