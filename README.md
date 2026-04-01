# Claude Code Launcher

A lightweight desktop app that makes Claude Code easy to use. No terminal knowledge needed — just configure and click.

## Features

- **Embedded terminal** — Claude Code runs inside the app with a built-in terminal (xterm.js + PTY)
- **Zero dependencies** — Windows installer bundles Node.js, Git, and Claude Code. Nothing else to install
- **Isolated environment** — Uses a separate config directory (`~/.claude-launcher/home/`), won't interfere with your existing Claude Code setup
- **Auto-skip onboarding** — Automatically creates `.claude.json` with `hasCompletedOnboarding: true` so Claude Code uses your API key directly
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

| Component | Size | Purpose |
|-----------|------|---------|
| Node.js v22 | ~30MB | Runtime for Claude Code |
| PortableGit | ~50MB | Git operations + bash.exe |
| Claude Code | ~20MB | The CLI itself |

Total installer size: ~100MB. No internet needed after install (except for API calls).

## How it works

```
┌─────────────────────────────────┐
│  Tauri App (Rust + React)       │
│  ┌───────────────────────────┐  │
│  │  Settings Page            │  │
│  │  API Key + Base URL       │  │
│  └───────────────────────────┘  │
│  ┌───────────────────────────┐  │
│  │  Embedded Terminal        │  │
│  │  xterm.js + PTY           │  │
│  │  ┌─────────────────────┐  │  │
│  │  │  _wrapper.js        │  │  │
│  │  │  sets process.env   │  │  │
│  │  │  spawns Claude Code │  │  │
│  │  └─────────────────────┘  │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

**Environment isolation:**
- Config stored in `~/.config/claude-launcher/config.json` (or `%APPDATA%` on Windows)
- Claude Code runs with `HOME`/`USERPROFILE` set to `~/.claude-launcher/home/`
- `.claude.json` auto-created with `hasCompletedOnboarding: true` to skip login flow
- Your real `~/.claude/` is never touched

**Windows env var workaround:**
portable-pty cannot pass environment variables on Windows ConPTY. The app generates a `_wrapper.js` that uses `child_process.spawn()` with an explicit `env` block to launch Claude Code with the correct `ANTHROPIC_API_KEY` and `ANTHROPIC_BASE_URL`.

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
git tag v0.3.7
git push origin v0.3.7
```

The CI downloads Node.js + PortableGit + Claude Code, bundles everything into a Windows `.exe` installer, and uploads to GitHub Releases.

### Build locally (macOS)

```bash
bash scripts/prepare-resources.sh
npx tauri build
```

## Troubleshooting

**Claude Code says "Unable to connect to Anthropic services"**
- Make sure you entered the correct API Base URL in Settings
- The app auto-creates `.claude.json` with `hasCompletedOnboarding: true`. If it's missing, Claude Code ignores your Base URL and tries its own login flow

**Terminal display issues on resize**
- The terminal uses debounced resize. Slight delay is normal when resizing the window

**Windows: "CLAUDE_CODE_GIT_BASH_PATH" errors**
- The app bundles PortableGit and adds `git/bin` to PATH. If you still see this error, uninstall, delete `%LOCALAPPDATA%\Claude Code Launcher`, and reinstall the latest version

## License

MIT
