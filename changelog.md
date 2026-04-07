# 更新日志

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
