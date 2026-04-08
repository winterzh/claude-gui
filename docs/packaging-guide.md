# Claude Code Launcher 打包手册

> 本手册以 **Antigravity** 品牌为例，说明如何基于 v0.9.7 源码进行自定义打包。

---

## 目录

1. [环境准备](#1-环境准备)
2. [克隆源码](#2-克隆源码)
3. [生成品牌素材](#3-生成品牌素材nano-banana)
4. [编写配置文件](#4-编写配置文件)
5. [执行打包](#5-执行打包)
6. [GitHub Actions 自动构建（Windows）](#6-github-actions-自动构建windows)
7. [配置字段说明](#7-配置字段说明)
8. [常见问题](#8-常见问题)

---

## 1. 环境准备

### macOS

| 工具 | 版本要求 | 安装方式 |
|------|---------|---------|
| Xcode Command Line Tools | 最新 | `xcode-select --install` |
| Rust | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | v18+ | `brew install node` 或 [nodejs.org](https://nodejs.org) |
| npm | 随 Node.js | 自带 |

### Windows

| 工具 | 版本要求 | 安装方式 |
|------|---------|---------|
| Visual Studio Build Tools | 2022+ | [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/) |
| Rust | stable | [rustup.rs](https://rustup.rs) |
| Node.js | v18+ | [nodejs.org](https://nodejs.org) |

---

## 2. 克隆源码

```bash
git clone --branch v0.9.7 https://github.com/winterzh/claude-gui.git
cd claude-gui
npm install
```

---

## 3. 生成品牌素材（Nano Banana）

使用 Nano Banana 生成两张图片：

### 3.1 应用图标（Icon）

**提示词参考：**
> A modern app icon for "Antigravity", minimalist style, floating geometric shapes defying gravity, gradient purple-to-blue tones, rounded square shape, clean and professional, suitable for macOS/Windows app icon

**要求：**
- 尺寸：**1024 x 1024 px**（必须正方形）
- 格式：**PNG**
- 背景：不透明（图标不支持透明背景在所有平台上表现一致）

生成后保存为：
```
config-packaging/icon-1024.png
```

### 3.2 启动画面（Splash）

**提示词参考：**
> A splash screen for "Antigravity" desktop app, centered logo with app name below, clean minimal design, dark background (#0f0f23), light text, professional SaaS product feel

**要求：**
- 尺寸：建议 **800 x 600 px** 或 **1024 x 768 px**
- 格式：**PNG**
- 背景色：建议与应用暗色主题一致（`#0f0f23`）或亮色（`#ffffff`）

生成后保存为：
```
config-packaging/splash.png
```

---

## 4. 编写配置文件

创建 `config-packaging/config.json`：

```json
{
  "enabled": true,

  "appName": "Antigravity",
  "appSlug": "antigravity",
  "identifier": "com.antigravity.app",
  "version": "1.0.0",

  "company": {
    "name": "Antigravity Team",
    "authors": ["Antigravity"],
    "copyright": "Copyright 2025 Antigravity",
    "website": "https://antigravity.example.com"
  },

  "icon": "config-packaging/icon-1024.png",
  "splash": "config-packaging/splash.png",

  "activationCodes": {
    "你的激活码1": {
      "name": "profile-name",
      "api_key": "sk-你的API密钥",
      "base_url": "https://你的API地址",
      "msg_zh": "已添加配置",
      "msg_en": "Added profile"
    }
  },

  "defaults": {
    "language": "zh",
    "showSkipPermissions": true,
    "isolationDir": ".antigravity"
  },

  "features": {
    "showActivationCode": true,
    "showUpdateButton": true
  },

  "build": {
    "nodeVersion": "v22.14.0",
    "gitVersion": "2.49.0"
  }
}
```

### 关键字段说明

| 字段 | 必须改 | 说明 |
|------|--------|------|
| `enabled` | **是** | 必须设为 `true`，否则配置不生效 |
| `appName` | 是 | 应用显示名称，会出现在窗口标题、首页 |
| `appSlug` | 是 | 技术名称（无空格、小写），用于包名和配置目录 |
| `identifier` | 是 | macOS Bundle ID，格式 `com.xxx.app` |
| `version` | 是 | 你的版本号 |
| `activationCodes` | 是 | 用户输入激活码后自动填入的 API 配置 |
| `defaults.isolationDir` | 建议改 | 用户目录下的隔离文件夹名 |
| `icon` / `splash` | 可选 | 不填则使用默认图标/启动画面 |

### 激活码工作原理

用户在初次设置页面输入激活码（如 `mycode01`），应用会查找 `activationCodes` 中对应的 key，自动填入 `api_key` 和 `base_url`，用户无需手动输入。

---

## 5. 执行打包

### 5.1 macOS 打包

```bash
# 1. 注入配置（修改 tauri.conf.json、Cargo.toml、package.json、生成图标、复制 splash）
node scripts/apply-packaging-config.mjs

# 2. 下载并打包运行时资源（Node.js universal binary + uv + Claude Code）
bash scripts/prepare-resources.sh

# 3. 构建应用
npx tauri build
```

构建产物位于：
```
src-tauri/target/release/bundle/
├── macos/
│   └── Antigravity.app          # macOS 应用包
└── dmg/
    └── Antigravity_1.0.0_universal.dmg  # 安装镜像（可能失败，见下方手动方法）
```

#### DMG 手动创建（如果自动 DMG 打包失败）

```bash
APP_NAME="Antigravity"
VERSION="1.0.0"
APP_PATH="src-tauri/target/release/bundle/macos/${APP_NAME}.app"
DMG_PATH="${APP_NAME}_${VERSION}_universal.dmg"
TEMP_DIR=$(mktemp -d)

# 准备 DMG 内容
cp -R "$APP_PATH" "$TEMP_DIR/"
ln -s /Applications "$TEMP_DIR/Applications"

# Ad-hoc 签名（无开发者证书时使用）
codesign --force --deep --sign - "$TEMP_DIR/${APP_NAME}.app"

# 创建 DMG
hdiutil create -volname "$APP_NAME" -srcfolder "$TEMP_DIR" \
  -ov -format UDBZ "$DMG_PATH"

rm -rf "$TEMP_DIR"
echo "DMG 已创建: $DMG_PATH"
```

### 5.2 Windows 打包

```powershell
# 1. 注入配置
node scripts/apply-packaging-config.mjs

# 2. 下载运行时资源（Node.js + PortableGit + uv + Claude Code）
pwsh scripts/prepare-resources-win.ps1

# 3. 构建应用
npx tauri build
```

构建产物位于：
```
src-tauri/target/release/bundle/nsis/
└── Antigravity_1.0.0_x64-setup.exe    # Windows 安装程序
```

---

## 6. GitHub Actions 自动构建（Windows）

如果使用 GitHub Actions 自动构建 Windows 版本：

### 6.1 设置 Repository Secret

进入你的 GitHub 仓库 → Settings → Secrets and variables → Actions → New repository secret

- **Name:** `PACKAGING_CONFIG`
- **Value:** 将 `config-packaging/config.json` 的完整内容粘贴进去

### 6.2 触发构建

三种触发方式：
- **推送 Tag：** `git tag v1.0.0 && git push origin v1.0.0` → 自动构建并创建 Release
- **手动触发：** 仓库 Actions 页面 → "Build Windows Installer" → Run workflow
- **定时构建：** 每周一 8:00 UTC 自动执行

构建完成后，.exe 安装包会自动上传到 GitHub Releases。

---

## 7. 配置字段说明

### 完整字段参考

```
config-packaging/
├── config.json        # 打包配置（含密钥，不要提交 git）
├── icon-1024.png      # 自定义图标（不要提交 git）
└── splash.png         # 自定义启动画面（不要提交 git）
```

> `config.json` 和 `*.png` 已在 `.gitignore` 中，不会被意外提交。

### `activationCodes` 详细说明

```json
"activationCodes": {
  "激活码": {                    // ← 用户输入的激活码字符串
    "name": "配置名称",          // 存储在本地配置中的名称
    "api_key": "sk-xxx",        // API 密钥
    "base_url": "https://...",  // API 地址
    "msg_zh": "中文提示",       // 激活成功后的中文提示
    "msg_en": "English message" // 激活成功后的英文提示
  }
}
```

可以配置多个激活码，每个对应不同的 API 提供商或配置。

### `features` 开关

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `showActivationCode` | `true` | 是否在设置页显示激活码输入框 |
| `showUpdateButton` | `true` | 是否在主页显示"更新 Claude Code"按钮 |

---

## 8. 常见问题

### Q: 不想自定义图标/启动画面怎么办？

删除 `config.json` 中的 `icon` 和 `splash` 字段即可，会使用默认图标和启动画面。

### Q: `enabled: false` 和没有 config.json 有什么区别？

没有区别。两种情况下应用行为完全一致，使用源码中的所有默认值。

### Q: macOS DMG 打包失败？

Tauri 的 DMG bundler 在某些环境下可能失败，但 `.app` 已经成功生成。使用上方的手动 DMG 创建命令即可。

### Q: 如何分发给没有开发者证书签名的用户？

macOS 用户首次打开时会提示"无法验证开发者"，需要：
1. 右键点击应用 → 打开
2. 或者：系统设置 → 隐私与安全性 → 仍要打开

### Q: 构建时 `npx tauri icon` 报错？

确保图标是 **1024x1024 的 PNG 文件**。如果 icon 生成失败，脚本会跳过图标替换（使用默认图标），不影响其他构建步骤。

### Q: 想同时维护多个品牌版本怎么办？

准备多份 `config.json`，构建前替换即可：
```bash
cp configs/antigravity.json config-packaging/config.json
node scripts/apply-packaging-config.mjs
# ...后续构建步骤
```

---

## 快速参考：一键打包流程

```bash
# macOS 完整流程
git clone --branch v0.9.7 https://github.com/winterzh/claude-gui.git
cd claude-gui
npm install

# 把 Nano Banana 生成的图片放到 config-packaging/
# 编辑 config-packaging/config.json（确保 enabled: true）

node scripts/apply-packaging-config.mjs    # 注入配置
bash scripts/prepare-resources.sh           # 下载资源（约 5 分钟）
npx tauri build                             # 构建（约 10 分钟）

# 产物在 src-tauri/target/release/bundle/macos/
```
