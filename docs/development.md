# 开发与构建

## 环境要求

- Node.js 与 pnpm（当前验证环境：Node.js 22.23.4、pnpm 10.30.3）
- Rust MSVC 工具链，最低 1.90（当前验证环境：Rust 1.98.1）
- Visual Studio Build Tools，包含桌面 C++ 工作负载
- WebView2 Runtime

以上为 Windows 环境。macOS 的前置环境、源码开发、打包和报告方式见[macOS 开发与社区验收](macos-development.md)。当前完整支持 Windows x64；平台能力边界见[平台支持](platforms.md)。

## 安装与启动

```powershell
pnpm install
pnpm tauri dev
```

开发地址固定为 `http://127.0.0.1:1420`。`pnpm tauri dev` 会启动前端服务；如果本项目的服务已在运行，先确认现有进程，避免重复启动。

仅预览工作台可运行 `pnpm dev`。浏览器中不能创建原生悬浮窗，原生窗口行为必须通过 Tauri 应用验证。

## 目录结构

```text
crates/                  CLI、协议与离线技能管理模块（共享根 Cargo.lock）
skills/pinpage-pages/     唯一页面技能及仓库共用的随包参考
src/
  features/launcher/      添加页面、来源导入
  features/pins/          工作台、即时窗口设置、独立内容保存与页面级预设管理器
  features/updates/       版本、下载进度与安装确认
  components/             通用组件
  services/               Tauri 调用封装
  i18n/                   前端多语言与消息映射
  styles/                 界面样式
  types/                  前端类型
src-tauri/src/
  commands/               前端命令入口
  windows/                窗口生命周期、事件与几何
  windows/editor.rs       窗口局部事务与内容独立保存
  windows/presets/         页面级预设查找、相对几何、读写及测试
  platform.rs / platform/ 跨平台能力、回退校验、Windows 适配与 macOS 实验拖动
  updates/                GitHub 更新检查、下载验签与安装
  content.rs              URL 与本地资源协议
  import.rs               导入复制与站点副本
  package.rs              页面包导入导出
  examples.rs             内置标准页面包索引
  data.rs                 页面数据采集与回填
  state.rs                配置读写与运行态
  tray.rs                 托盘
  lib.rs / main.rs        应用装配与入口
locales/                  中英文界面与原生消息
examples/                 七个内置工具（行情需要联网）的 manifest.json + site/ 标准包源码
scripts/                  版本同步、发布、签名构建、文案检查与性能采样
tests/                    UI 表单测试与本地路径样例
docs/                     用户、架构、开发、平台与发布文档
```

按实际复杂度拆分模块，不为占位提前生成空文件。单个代码文件接近 300 行时检查是否需要拆分。

## 创建示例库页面

仓库只维护 [pinpage-pages](../skills/pinpage-pages/SKILL.md) 一份页面技能。示例任务由根 [AGENTS.md](../AGENTS.md) 与 [examples/AGENTS.md](../examples/AGENTS.md) 引导，按[示例开发与接入](examples-development.md)直接复用公共页面制作、宿主能力和包格式参考；示例注册、预览卡片、中英文文案与 Rust 打包由接入文档说明。仅编写源码无需先连接应用或安装页面。

页面包清单和默认窗口布局见[页面包与示例制作](page-packages.md)。运行 `pnpm examples:pack` 在 `dist/examples/` 生成标准 `.pinpage`；Rust 构建自动使用同一打包器生成内嵌包，`site/` 中的资源按相对目录自动收集。修改 Rust 构建内嵌的示例后需使用对应新构建验证，已添加的独立副本不会自动更新。

## 检查与测试

| 命令                                                                                            | 覆盖范围                                                                                       |
| ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `pnpm examples:pack`                                                                            | 将标准示例源码打包到 `dist/examples/`，不启动桌面应用                                          |
| `pnpm check:i18n`                                                                               | 中英文资源的键、参数、原生资源语法，以及代码中引用的消息键是否落库                             |
| `pnpm check:limits`                                                                             | 前端与原生窗口尺寸上下限是否一致                                                               |
| `pnpm check:skill`                                                                              | 唯一技能、随包资源完整性、自包含文件与章节引用、示例接入文档链接                               |
| `pnpm check`                                                                                    | TypeScript 类型检查（`vue-tsc --noEmit`）                                                      |
| `pnpm build`                                                                                    | 文案、尺寸限制、技能资源与引用、类型检查及 Vite 生产构建                                       |
| `pnpm test:examples`                                                                            | 示例旧数据、计时、日期、看板状态、行情解析和请求竞态                                           |
| `pnpm test:ui`                                                                                  | 设置表单回传合并、连续输入、失败保留、另存前等待、操作队列及平台能力过滤与回退，不连接真实配置 |
| `pnpm test:rust`                                                                                | Rust 单元测试与原生窗口集成测试                                                                |
| `pnpm version:check`                                                                            | 前端、Tauri、Cargo 与 Cargo 锁文件的应用版本一致性                                             |
| `pnpm test:release`                                                                             | 版本同步、标签约束与 GitHub 更新清单                                                           |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`                                  | Rust 格式检查                                                                                  |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D clippy::all` | Rust 静态检查                                                                                  |
| `pnpm format:check`                                                                             | 前端、语言资源、脚本、配置与样例文件的 Prettier 检查                                           |

只运行与改动相关的检查；既有检查通过后，除非发现新问题，不重复扩大测试范围。涉及 URL 校验、本地资源路径边界、状态迁移和窗口恢复时，优先补充对应测试。

窗口置顶、无边框拖动缩放、DPI 与多显示器行为必须在真实桌面环境验证，自动化测试不能替代这些场景。

## 当前状态与验收

实现状态、历次检查、桌面实测与待验收项已独立维护在[当前状态与验收](acceptance.md)。本节保留原链接入口，验收结果以独立文档为准。

## 打包

```powershell
pnpm release:build
```

- Release 可执行文件：`src-tauri/target/release/pinpage.exe`
- NSIS 安装包：`src-tauri/target/release/bundle/nsis/`
- 构建产物不提交 Git。

正式发布只提供 Windows NSIS 目标；本地签名构建与离线验签命令见[版本与升级](releases.md)。macOS 使用独立平台配置构建验证 `.app`，命令见[macOS 开发指南](macos-development.md#本地打包与独立实例)。Windows 安装包代码签名暂未配置。

直接运行可执行文件时可追加 `--tray`，只创建托盘并按设置恢复悬浮窗，工作台按需打开。

## 持续集成与发布

`.github/workflows/ci.yml` 在推送到 `main`、`feature/macos-*` 和合并请求时运行，也可手动触发：

- 前端（Ubuntu）：Prettier、文案检查、类型检查、UI 测试与生产构建。
- Rust（Windows）：Rustfmt、Clippy、单元测试与原生窗口集成测试。
- Rust（macOS 默认／Option 实验）：Rustfmt、Clippy、单元测试，构建 `com.pinpage.verification` 标识的 Debug `.app` 并归档上传。任务使用 runner 原生架构；未运行的分支不能记录为 CI 通过，构建成功不替代实机验收。

`.github/workflows/release.yml` 在推送 `v*.*.*` 标签时构造 Windows 安装包并创建 GitHub Release：

1. 运行 `pnpm version:set <版本>`，同步前端、Tauri、Cargo 与 Cargo 锁文件中的应用版本。
2. 提交并推送 `main`，再创建并推送标签，例如 `git tag v0.2.0` 和 `git push origin v0.2.0`。
3. 工作流校验版本、签名构建并离线验签，准备草稿，上传 NSIS 安装包、`.sig`、`.sha256` 和 `latest.json`，全部就绪才正式发布。预发布和旧版补发不会覆盖 Latest。

也可在 Actions 页面手动运行 Release 并输入已存在的标签。已发布版本禁止覆盖，操作细节见[版本与升级](releases.md)。当前安装包未进行 Windows 代码签名，Windows SmartScreen 可能提示未知发布者。

标签推送后的发布脚本修复不移动标签：修复提交进入 `main` 后，从该提交手动运行 Release 工作流，仍构建指定标签的应用，只从工作流 SHA 加载修复后的发布脚本。草稿查询使用完整 Release 列表，覆盖尚未公开的版本。

## 独立验证实例

需要在不影响当前应用会话的情况下验证时，使用单独的标识与输出目录：

```powershell
pnpm build
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) "target/i18n-verify"
pnpm tauri build --config src-tauri/tauri.verification.conf.json --no-bundle
```

验证实例使用独立标识（`com.pinpage.verification`）与独立配置目录，产物写入自定义目标目录；验证结束后删除验证目录，不修改正常应用的配置。macOS 的对应命令见[开发指南](macos-development.md#本地打包与独立实例)。

## 性能测量

使用 Release 构建，先在工作台确认实际打开的页面数量：

```powershell
./scripts/measure-memory.ps1 -WindowCount 1 -Label timer-idle
./scripts/measure-memory.ps1 -WindowCount 3 -Label timer-idle
./scripts/measure-memory.ps1 -WindowCount 5 -Label timer-idle
```

原始结果写入已忽略的 `runtime-data/memory-samples.jsonl`。读取指标时区分私有提交内存与工作集，说明共享内存口径，并同时记录系统、硬件、WebView2 版本与测量时长。首轮数据与统计口径见[架构与数据](architecture.md#性能与资源)。

## 文档维护

- [README.md](../README.md)（英文）与 [README.zh-CN.md](../README.zh-CN.md)（中文）只保留项目概览、特点、快速开始与文档索引；使用细节、架构和开发说明放在 `docs/`。
- 功能、命令或验收结论变化时同步更新对应文档与中英文版本，并检查链接和内容一致性。
- 平台能力、回退规则或 macOS 验收结论变化时，同步更新[平台支持](platforms.md)与[当前状态与验收](acceptance.md)。
- [docs/screenshots](../docs/screenshots) 存放 README 与用户指南的界面截图，更新界面后按同名文件替换即可。
- 协作规范见 [AGENTS.md](../AGENTS.md)。

## CLI 独立验收脚本

先构建独立标识的验证应用或安装测试包，然后运行：

```powershell
# 功能：验证新目录页面、存储、冲突、诊断和 1／3／5 页资源，保留报告与源码。
./scripts/verify-agent-cli.ps1 -CliPath <CLI绝对路径> -AppPath <应用绝对路径> -Instance com.pinpage.verification
```

脚本拒绝正式实例标识，只关闭自己创建的页面并停止自己的诊断，不删除记录。默认截图失败即失败；无桌面环境可显式传 -AllowCaptureUnavailable 继续功能采样，但报告仍保留截图失败且 ok 为 false。报告在 runtime-data/agent-cli/acceptance 的独立子目录。测量工具新增 -AppPath 与 -BuildMode，可精确选择隔离实例，额外记录句柄与线程。

## 窗口预设与页面包独立验收

复用或构建独立标识的验证应用与同版 CLI，再执行：

```powershell
# 功能：验证命名预设、包往返、冲突保留与真实桌面效果，不操作正式或开发实例。
./scripts/verify-window-presets.ps1 -CliPath <CLI绝对路径> -AppPath <应用绝对路径> -Instance com.pinpage.preset-verification
```

脚本保留源码、页面包、截图与报告到忽略的 runtime-data/window-presets，结束时关闭自己创建的页面。无桌面采集能力时可加 `-AllowCaptureUnavailable`，报告会保留原因；截图需要实际打开检查后才能形成视觉结论。
