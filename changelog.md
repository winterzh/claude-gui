# 更新日志

## v0.9.8

- **适配 Claude Code 2.1.x native binary**: Anthropic 已将 npm 包从 JS 改为单文件原生二进制,启动器同步切换为直接 spawn `bin/claude.exe`,不再依赖 node 运行时
- **修复内置升级**: macOS 上 `bin/npm` 软链不正确导致 npm 启动失败,改为直接执行 `npm-cli.js`;升级后会校验 `claude --version` 实际可执行,失败回报具体错误
- **修复 "Auth conflict" 警告**: 不再同时设置 `ANTHROPIC_API_KEY` 与 `ANTHROPIC_AUTH_TOKEN`;Profile 携带 `auth_env` 字段,DeepSeek 等 Bearer 风格代理走 `ANTHROPIC_AUTH_TOKEN`,Anthropic 兼容代理走默认 `ANTHROPIC_API_KEY`
- **Model 跟随 Profile**: 模型字段不再全局共享,每个 Profile 独立保存;切换 Profile 自动加载对应 Model
- **一键配置 DeepSeek v4**: 新增按钮,自动创建 `deepseek v4` Profile,预填 base_url、模型、超时、effort level、subagent 模型等 7 项 env;用户只需填 API Key
- **Profile 类型分级**: 激活码 Profile 完全隐藏配置项;DeepSeek 等 curated Profile 仅显示 API Key 输入框;自定义 Profile 全部可编辑
- **模型获取按钮**: 设置页可一键拉取 `${baseUrl}/v1/models` 列表,可视化选择
- **xterm 中文/Box-drawing 渲染修复**: 加载 Unicode 11 width 表 + 启用 `customGlyphs`,避免 CJK 字符与 ASCII 错位重叠
- **Shift+Enter 换行 / Ctrl+V 粘贴**: 在 xterm 上自定义键盘处理器,Shift+Enter 注入 Alt+Enter 等价转义;Windows/Linux Ctrl+V、macOS Cmd+V 通过 clipboard API 写入 PTY
- **新建 Profile 修复**: 替换 `window.prompt`(Tauri 2 webview 不支持)为内联输入框,Enter 确认 Esc 取消
- **macOS 图标重做**: 原图无透明背景导致 Dock 显示一圈白方块,改为透明背景 + 80% 内边距(符合 Apple HIG)
- **死代码清理**: 删除 `launch_claude_code` / `build_launch_script` / `sync_user_settings` / `merge_json` 等未使用函数

## v0.9.6

- 当 Base URL 包含 "minimax" 时，自动配置 MiniMax MCP（网页搜索 + 图片理解）
- 内置 uv（Python 包运行器），MCP 服务器开箱即用，无需手动安装
- 切换到非 MiniMaxi 配置时自动移除 MiniMax MCP
- 兼容 `.claude.json` 和 `.claude/.config.json` 两种配置路径
- 全面清理所有可能残留 MCP 配置的文件位置
- 设置页面：激活码配置完全隐藏 Key 和 URL
- 设置页面：自定义配置明文显示 URL，Key 默认遮掩并提供显示/隐藏按钮
- 修复 Windows 首页 Recent Activity 为空的问题（盘符大小写导致项目目录 slug 不匹配）
- Windows 工作目录路径规范化（统一盘符小写 + 正斜杠）
- 项目目录同步支持大小写模糊匹配和盘符别名匹配
- 优雅退出：先发送 `exit` 命令等待 1 秒，再强制终止进程
- Windows 和 macOS 的 PATH 中均加入 uv 路径

## v0.9.2

- `.claude.json` 中写入 `installMethod: "native"`，支持 Recent Activity 显示
- 首页显示版本号

## v0.9.1

- 回退到 v0.8.6 基线，移除 `sync_user_settings`（导致 macOS 权限弹窗）

## v0.9.0

- MiniMaxi MCP 网页搜索集成（首次尝试，后在 v0.9.6 中重做）
- 内置 uv
- 移除 MSI 安装包输出

## v0.8.7

- 同步完整 `~/.claude/` 目录以支持网页搜索和 OAuth
- （v0.9.1 中回退，因触发 macOS 权限弹窗）

## v0.8.6

- 修复 macOS 修改 HOME 后网络不可用的问题
- macOS 改用 `CommandBuilder::env()` 直接启动 node（保留系统 SSL/DNS 环境）
- 从真实 `~/.claude/settings.json` 同步用户设置到隔离环境

## v0.8.5

- 修复 Windows 上 curl 测试连接时闪过黑色窗口的问题

## v0.8.4

- Windows 上隐藏所有子进程控制台窗口（`CREATE_NO_WINDOW`）

## v0.8.3

- 所有错误消息脱敏：绝不暴露 API Key 或 Base URL

## v0.8.2

- 友好的错误提示（覆盖 401、403、404、429、5xx 等状态码）
- 连接测试从 reqwest 切换为 curl（reqwest 在 Tauri 中有 TLS 兼容问题）

## v0.8.1

- 重新生成 256x256 应用图标，带抗锯齿

## v0.8.0

- 全新应用图标
- 包含 Windows 所需的所有图标尺寸

## v0.7.2

- 更新图标

## v0.7.1

- 代码审查清理和简化

## v0.7.0

- 设置中增加"跳过所有权限确认"选项（带安全警告确认弹窗）
- 激活码功能（cclxy01、cclxy02 预设配置）
- 自动信任工作目录

## v0.6.2

- 激活码预设 API 配置
- 首次启动默认空配置
- 预信任工作目录，跳过 Claude Code 信任对话框

## v0.6.1

- 预写 `.claude.json` 中的 `hasCompletedOnboarding` 跳过引导流程
- 替代之前的自动按键方案

## v0.6.0

- 自动应答 Claude Code 的 API Key 确认和引导提示
- 增加 `--dangerously-skip-permissions` 选项

## v0.5.5

- 应用内 Claude Code 更新（npm update）
- GitHub Actions 每周自动构建

## v0.5.4

- 每个配置独立的 API Key 和 Base URL
- 敏感字段完全遮掩
- 统一的配置管理界面

## v0.5.3

- 修复 Windows ENOENT 错误：wrapper 改用 `require()` 替代 `spawn()`
- 增加资源路径校验

## v0.5.2

- 产品名从 "Claude Code Launcher" 改为 "Claude-Code-Launcher"（去除空格）
- 修复 Windows 路径含空格导致启动失败

## v0.5.1

- 配置列表内联创建/删除按钮

## v0.4.0

- 连接测试按钮
- 多配置支持
- 连接状态指示器
- 新图标

## v0.3.7

- 高分辨率抗锯齿图标

## v0.3.6

- 每次启动强制写入 `.claude.json`
- 修复连接失败根本原因

## v0.3.5

- 自动创建 `.claude.json` 跳过 Claude Code 引导流程

## v0.3.4

- 使用 `child_process.spawn` 并显式传递环境变量

## v0.3.3

- 移除 `CLAUDE_CODE_GIT_BASH_PATH` 变通方案

## v0.3.2

- 不再设置 `CLAUDE_CODE_GIT_BASH_PATH`，依赖 PATH 查找

## v0.3.1

- 引入 Node.js `_wrapper.js` 模式绕过 Windows ConPTY 环境变量问题
- 在同一进程中 `process.env.*` 然后 `require("cli.js")`

## v0.3.0

- 修复 Windows 环境变量：在 spawn 前设置父进程环境

## v0.2.9

- Windows 直接 spawn `node.exe`，不再经过 `cmd.exe`

## v0.2.8

- 尝试通过 PTY stdin 输入命令修复 Windows 环境变量

## v0.2.7

- 修复 Windows bat 脚本引号问题，使用 TEMP 目录

## v0.2.6

- Windows 改用内联 `set` 命令替代 .bat 文件

## v0.2.5

- 从 MinGit 切换到 PortableGit（包含 `bash.exe`）

## v0.2.4

- 修复 Windows 上环境变量未传递给 Claude Code

## v0.2.3

- 内置 MinGit-busybox（含 `bash.exe`）
- 设置 `CLAUDE_CODE_GIT_BASH_PATH`

## v0.2.1

- 修复终端调整大小：防抖 fit 避免行错位

## v0.2.0

- 内嵌终端 GUI（xterm.js + FitAddon）
- Windows 内置 MinGit，零外部依赖

## v0.1.2

- 修复 Windows Git Bash 启动
- UI 中显示错误信息

## v0.1.0

- 初始版本
- 一键桌面应用，封装 Claude Code CLI
- Tauri v2（Rust 后端 + React 前端）
- 内置 Node.js 和 Claude Code
- 配置 API Key 和 Base URL，支持中国区中继/代理服务
- 环境隔离（`~/.claude-launcher/home/`）
- Windows 为主，macOS 为辅
- GitHub Actions 自动构建 Windows 安装包
