# Claude Code Launcher

A lightweight desktop app that makes Claude Code easy to use on Windows. No terminal knowledge needed — just configure and click.

Windows 上开箱即用的 Claude Code 桌面客户端。无需命令行知识，配置即用。

![Version](https://img.shields.io/github/v/release/winterzh/claude-gui)
![Downloads](https://img.shields.io/github/downloads/winterzh/claude-gui/total)

---

## Features / 功能

- **Embedded terminal / 内嵌终端** — Claude Code runs inside the app (xterm.js + PTY)
- **Zero dependencies / 零依赖** — Installer bundles Node.js, Git, and Claude Code
- **Multi-profile / 多配置** — Save and switch between multiple API configurations
- **Connection test / 连接测试** — Verify your API key and endpoint before launching
- **Presets / 预设** — One-click setup for Anthropic, Pincc.ai, MiniMaxi
- **Connection status / 连接状态** — Real-time indicator on the home page
- **Isolated environment / 环境隔离** — Separate config directory, won't touch your existing Claude Code setup
- **Dark / Light theme, Chinese / English / 深浅主题，中英双语**

---

## Quick Start / 快速上手

### 1. Download / 下载

Go to [Releases](https://github.com/winterzh/claude-gui/releases) and download the latest `.exe` installer.

前往 [Releases](https://github.com/winterzh/claude-gui/releases) 页面下载最新的 `.exe` 安装包。

### 2. Install / 安装

Run the installer. Everything is bundled — no need to install Node.js, Git, or anything else.

运行安装包即可。Node.js、Git、Claude Code 全部内置，不需要额外安装任何东西。

### 3. Configure / 配置

Open the app and create a new profile:

打开应用，新建一个配置：

1. Click **+ New Profile / 新建配置**
2. Choose a **preset** (Pincc.ai / MiniMaxi / Anthropic) or enter a custom Base URL / 选择预设或输入自定义地址
3. Enter your **API Key** / 输入 API Key
4. Click **Test / 测试连接** to verify / 验证连接
5. Click **Save / 保存**

### 4. Launch / 启动

1. Choose a **working directory** / 选择工作目录
2. Click **Launch Claude Code / 启动 Claude Code**
3. Start coding! / 开始编码！

---

## Presets / 预设中转站

| Provider / 服务商 | Base URL | Notes / 备注 |
|----------|----------|-------|
| Anthropic (Direct) | `https://api.anthropic.com` | Requires direct API access / 需直连 |
| Pincc.ai | `https://v2.pincc.ai` | China-friendly relay / 国内可用 |
| MiniMaxi | `https://api.minimaxi.com/anthropic` | China-friendly relay / 国内可用 |

You can also save custom configurations as profiles and switch between them.

也可以保存自定义配置，随时切换。

---

## What's bundled / 打包内容

| Component / 组件 | Purpose / 用途 |
|-----------|---------|
| Node.js v22 | Runtime for Claude Code / Claude Code 运行时 |
| PortableGit | Git operations + bash.exe / Git 操作 |
| Claude Code | The CLI itself / CLI 本体 |

Total installer size: ~100MB. / 安装包约 100MB。

---

## How it works / 工作原理

```
 User clicks "Launch Claude Code"
 用户点击"启动 Claude Code"
              |
              v
 ┌─────────────────────────────┐
 │  _wrapper.js                │
 │  Sets process.env:          │
 │    ANTHROPIC_API_KEY        │
 │    ANTHROPIC_BASE_URL       │
 │    HOME (isolated)          │
 │    PATH (bundled git+node)  │
 │  require() Claude Code CLI  │
 └──────────┬──────────────────┘
            |
            v
 ┌─────────────────────────────┐
 │  Embedded Terminal          │
 │  xterm.js <-> PTY <-> node │
 └─────────────────────────────┘
```

**Environment isolation / 环境隔离:**
- Claude Code config / 配置目录: `~/.claude-launcher/home/.claude/` (not `~/.claude/`)
- `.claude.json` auto-created with `hasCompletedOnboarding: true` / 自动创建跳过引导
- App config / 应用配置: `%APPDATA%/claude-launcher/config.json`

---

## Build from source / 从源码构建

### Prerequisites / 前置条件

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) >= 1.70

### Development / 开发

```bash
npm install
npx tauri dev
```

### Release build / 发布构建

Push a version tag to trigger GitHub Actions:

```bash
git tag v0.4.5
git push origin v0.4.5
```

CI downloads Node.js + PortableGit + Claude Code, bundles into `.exe`, uploads to Releases.

CI 自动下载 Node.js + Git + Claude Code，打包成 `.exe`，上传到 Releases。

---

## Troubleshooting / 故障排查

### "Unable to connect to Anthropic services" / 无法连接

The app auto-creates `.claude.json` with `hasCompletedOnboarding: true`. Without this file, Claude Code ignores your Base URL. If you see this error:

应用会自动创建 `.claude.json`。如果仍然出现此错误：

1. Uninstall the app / 卸载应用
2. Delete / 删除 `%LOCALAPPDATA%\Claude Code Launcher`
3. Delete / 删除 `%USERPROFILE%\.claude-launcher`
4. Reinstall the latest version / 重新安装最新版

### Connection test works but Claude Code doesn't connect / 测试通过但启动后连不上

Make sure you're on the latest version. / 确保使用最新版本。

### API key prompt / API Key 确认提示

The app auto-selects "Yes" when Claude Code asks "Do you want to use this API key?". If it doesn't work, manually press Up arrow then Enter.

应用会自动选择 "Yes"。如果没有自动选择，手动按上箭头再按回车。

---

## License

MIT
