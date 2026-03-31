# Claude Code Launcher

A lightweight desktop app that makes Claude Code easy to use. No terminal knowledge needed — just configure and click.

## Features

- **One-click launch** — Configure API Key and Base URL, choose a working directory, click to start
- **Bundled Node.js** — Windows installer includes everything, no extra installation required
- **Isolated environment** — Uses a separate config directory (`~/.claude-launcher/home/`), won't interfere with your existing Claude Code setup
- **Cross-platform** — macOS (dev) + Windows (installer)
- **Dark / Light theme**
- **Chinese / English**

## Download

Go to the [Releases](https://github.com/winterzh/claude-gui/releases) page and download the latest `.exe` installer for Windows.

## Usage

1. Install and open the app
2. Enter your **API Key** and **API Base URL** (proxy/relay endpoint)
3. Choose a **working directory**
4. Click **Launch Claude Code**
5. A terminal window opens with Claude Code running — fully configured

## Build from source

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) >= 1.70

### Development

```bash
# Install dependencies
npm install

# Run in dev mode (macOS/Linux)
npx tauri dev
```

### Build for Windows

Push a version tag to trigger the GitHub Actions CI build:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The CI will:
1. Download portable Node.js + Claude Code
2. Bundle them into the app
3. Build a Windows `.exe` installer
4. Upload to GitHub Releases

### Build locally (macOS)

```bash
# Prepare bundled resources
bash scripts/prepare-resources.sh

# Build
npx tauri build
```

## How it works

The launcher is a thin Tauri (Rust + React) wrapper that:

1. Stores your API config in `~/.config/claude-launcher/config.json`
2. Creates an isolated home directory at `~/.claude-launcher/home/`
3. Launches Claude Code in a terminal with the right environment variables set (`ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL`, `HOME`)
4. Your real `~/.claude/` is never touched

## License

MIT
