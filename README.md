# PinPage

PinPage 是使用 Tauri 2 实现的桌面网页悬浮工具，用于把计时器、看板和本地工具页固定在其他普通窗口上方。

首版优先支持 Windows，围绕同时使用 1～5 个轻量页面设计。

## 当前状态

首个开发版本已实现管理界面、多悬浮窗、URL 与本地 HTML 加载、置顶和边框切换、托盘、全局快捷键及会话保存恢复。完整验收进度见 [验证记录](docs/verification.md)。

## 首版目标

- 输入 HTTP/HTTPS 地址，在独立的原生 WebView 窗口中打开网页。
- 选择本地 HTML 文件，加载资源根目录内的 CSS、JavaScript、图片和其他静态资源。
- 支持窗口置顶、普通边框与无边框切换、移动和自由调整大小。
- 悬浮页不显示任务栏图标，管理界面保留任务栏入口；悬浮页可通过托盘和快捷键找回。
- 通过托盘和快捷键完成刷新、显示、隐藏、恢复边框和关闭等操作。
- 保存页面来源、窗口位置、大小和显示设置，并支持重启恢复。
- 支持多个悬浮窗，默认共享 WebView2 环境和应用数据目录。

## 使用方式

1. 打开管理界面，输入网页地址或选择本地 HTML 文件。
2. 创建悬浮窗，调整位置和大小，设置置顶与边框模式。
3. 关闭管理界面，继续使用悬浮窗；通过托盘或 `Ctrl+Alt+P` 重新打开管理界面。
4. 无边框窗口使用 `Alt + 左键` 拖动；`Ctrl+Alt+B` 找回全部窗口、恢复边框并居中。
5. 从工作台或托盘显式退出，下次启动时根据设置恢复会话。

关闭单个悬浮窗会删除该窗口的会话记录；隐藏窗口保留页面运行状态。关闭管理界面会销毁管理 WebView，悬浮窗继续运行。快捷键和启动恢复可在偏好设置中修改。

工作台提供内置专注计时器，支持预设时长、暂停和基于截止时间的计时。可独立打开 [示例 HTML](examples/timer/index.html)，体验本地工具页。

本地前端项目可以通过开发服务器地址打开；静态构建产物可以通过 HTML 入口文件打开。

## 技术方案

| 层次 | 采用技术 | 职责 |
| --- | --- | --- |
| 桌面框架 | Tauri 2、Rust | 窗口管理、托盘、快捷键、资源加载和配置保存 |
| 管理界面 | Vue 3、TypeScript、Vite | 地址输入、文件选择、悬浮窗列表和设置 |
| Windows 渲染 | WebView2 | 渲染远程网页和本地页面 |
| 前端包管理 | pnpm | 安装依赖和执行前端脚本 |
| 配置存储 | 带版本号的 JSON | 保存应用设置和悬浮窗状态 |

目标网页与管理界面使用独立 WebView，系统权限仅授予受信任的管理界面。首版采用托盘和快捷键控制悬浮窗，窗内自定义工具栏在完成多 WebView 技术验证后评估。

## 性能原则

多个 WebView 可以共享浏览器基础设施，但网页仍有各自的运行开销，进程数量也由引擎和站点隔离策略决定。

管理界面按需创建、关闭后销毁，每个悬浮窗只承载一个内容 WebView。项目提供 [性能采样脚本](scripts/measure-memory.ps1)，测量整个应用进程组，结果与统计口径见 [验证记录](docs/verification.md)。

首轮 Release 短时采样中，1、3、5 个静置计时器的整组私有提交内存约为 205、291、377 MiB。这不是物理驻留内存，也不代表复杂看板或长时间运行的资源占用。

## 开发准备

开发环境需要 Node.js、pnpm、Rust MSVC 工具链、带桌面 C++ 工作负载的 Visual Studio Build Tools，以及 WebView2 Runtime。当前验证环境为 Node.js 22.23.4、pnpm 10.30.3、Rust 1.98.1。

```powershell
pnpm install
pnpm tauri dev
```

开发地址固定为 `http://127.0.0.1:1420`。`pnpm tauri dev` 会启动前端服务；已有本项目服务时应先确认当前进程，避免重复启动。

仅预览管理界面可运行 `pnpm dev`，浏览器中不能创建原生悬浮窗。

## 检查与打包

```powershell
pnpm build
pnpm format:check
pnpm test:rust
cargo fmt --manifest-path src-tauri/Cargo.toml --check
pnpm tauri build
```

Release 可执行文件位于 `src-tauri/target/release/pinpage.exe`，NSIS 安装包位于 `src-tauri/target/release/bundle/nsis/`。构建产物不会提交 Git。

直接运行可执行文件时可追加 `--tray`，只创建托盘并按设置恢复悬浮窗，管理界面按需打开。

## 本地文件与配置

- 本地 HTML 默认只能访问所在目录内的资源；需要引用上级目录资源时，在选择文件前勾选“另外指定资源根目录”。
- 文件路径支持中文和空格；目录穿越、越界目录联接和非法协议会被拒绝。
- 配置保存在 `%APPDATA%\com.pinpage.desktop\settings.json`；网页登录数据由应用的 WebView2 用户数据目录管理。
- 网页运行态由页面自身决定，刷新、关闭或重启后不会自动恢复完整 JS 内存状态。
- 新窗口链接交给默认浏览器，依赖应用内弹窗的登录流程需要按站点验证。

## 文档

- [首版项目规划](docs/plan.md)：功能范围、架构决策、实施阶段和验收标准。
- [协作规范](AGENTS.md)：代码、文档、工具和验证要求。
- [验证记录](docs/verification.md)：已验证项目、性能数据及待验收场景。

## 官方参考

- [Tauri Windows 开发环境要求](https://v2.tauri.app/start/prerequisites/)
- [Tauri 窗口配置](https://v2.tauri.app/reference/config/)
- [WebView2 进程模型](https://learn.microsoft.com/microsoft-edge/webview2/concepts/process-model)
