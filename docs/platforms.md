# 平台支持

PinPage 以 Windows x64 为完整支持与发布平台。macOS 为社区协助验证的实验性平台：默认只声明标准浮动窗口能力，Option 拖动需显式启用实验构建。桌面嵌入、圆角和几何锁不在 macOS 开放，也不会被静默保存。使用说明见[用户指南](user-guide.md)，模块设计见[架构与数据](architecture.md)，参与开发见[macOS 开发与社区验收](macos-development.md)。

## 能力矩阵

| 能力 | Windows x64 | macOS | 其他平台 |
| --- | --- | --- | --- |
| 标准浮动窗口、置顶、系统边框、尺寸与位置 | 支持 | ARM64 CI 编译、测试与打包通过；窗口交互待实机验收 | 使用同一回退分支，未构建验收 |
| 桌面模式（嵌入 Explorer 桌面层） | 支持 | 不支持 | 不支持 |
| 无边框圆角（原生区域裁剪） | 支持 | 不支持 | 不支持 |
| 位置锁、大小锁 | 支持 | 不支持 | 不支持 |
| 修饰键拖动 | 支持 | `experimental-macos-drag` 提供固定 Option 拖动，默认关闭、待验收 | 不支持 |
| 窗口原生右键菜单、可配置手势 | 支持 | 不支持 | 不支持 |
| 工作台、托盘、导入、示例、页面包 | 支持 | 共用代码，未实机验收 | 未构建验收 |
| 简体中文 / 英文 | 支持；可跟随系统语言 | 支持手动切换；跟随系统回退中文 | 同 macOS |
| 应用内更新 | 支持；仅 Windows x64 正式构建 | 不支持 | 不支持 |
| 安装包 | NSIS x64 | 已配置本地 `.app` 与 CI Debug 验证包，未正式分发 | 未提供 |

## 能力表与裁剪

Rust 在快照中下发 `capabilities`，前端所有窗口入口只使用这一份数据，不读取 `userAgent`，也不按构建条件自行推断。

| 字段 | 含义 |
| --- | --- |
| `desktopMode` | 是否支持桌面模式 |
| `rounded` | 是否支持无边框圆角 |
| `positionLock` / `sizeLock` | 是否支持独立的位置锁和大小锁 |
| `windowGestures` | 是否提供手势修饰键设置 |
| `dragModifier` | 平台固定的原生拖动修饰键；`null` 表示没有原生手势 |

- Windows 全部为 `true`，`dragModifier` 为 `alt`。
- macOS 默认全部为 `false`，`dragModifier` 为 `null`；启用 `experimental-macos-drag` 后只有 `dragModifier` 变为 `option`，`windowGestures` 仍为 `false`，不开放修饰键设置。
- 其他平台全部为 `false`，`dragModifier` 为 `null`，不会继承 macOS 的实验能力。
- `platform::validate_pin` 与 `platform::validate_preset` 在窗口创建、应用、预设写入和会话恢复时做最终校验；`platform::apply` 是窗口属性的唯一应用入口。
- 页面设置、新建页、卡片预设菜单和托盘按能力禁用或隐藏入口，不可用选项带平台限制提示。

## 配置回退

- 读取其他平台写下的配置时，桌面模式转为浮动并恢复系统边框，清除圆角、两把锁与失效预设标记；名称、来源、尺寸、位置和开启状态保留，工作台显示一次说明。
- 无手势平台新建页面默认保留系统边框；手动改为无边框仍可创建，但缺少原生拖动入口，只能通过托盘或工作台找回。
- 显式 IPC 提交当前平台不支持的能力时返回 `errors.platformUnsupported`，保留原窗口与配置，不做静默降级。
- 预设菜单只展示能在当前平台完整应用的模板；预设管理器保留跨平台旧模板，编辑前转换为当前平台可保存的组合。
- 非 Windows 平台的“跟随系统”回退简体中文；手动选择中文或英文仍然可用。
- 应用内更新只在 Windows x64 正式构建启用；其他平台在“关于”中显示版本和 Release 入口。

## macOS 当前范围

以下实现已通过 macOS ARM64 CI 编译、测试与打包，但交互行为仍待实机验收：

- 标准浮动窗口，包含置顶、系统边框、尺寸与位置。
- 工作台、托盘、页面导入、内置示例、页面包与手动语言切换。
- 托盘「找回全部窗口」和页面卡片找回。
- 显式实验构建中的 Option + 左键原生拖动，来源与接入范围见[开发指南](macos-development.md#两种构建)。

不支持：

- 桌面模式、无边框圆角、位置锁与大小锁。
- 默认构建的修饰键拖动，以及所有构建的窗口右键菜单、修饰键设置。
- 应用内更新、NSIS 安装包、跟随系统语言。

需要注意：

- 默认构建没有无边框原生拖动，建议保留系统边框；实验构建可用固定 Option 拖动。两种构建均保留托盘与工作台找回入口。
- macOS 使用 WKWebView，与 Windows 的 WebView2 在页面兼容性、导出数据注入和存储行为上可能存在差异，尚未验证。
- 配置目录与网页数据由 Tauri 在 macOS 的应用数据目录管理，路径与 Windows 的 `%APPDATA%` 不同。

## 构建与验收状态

- 通用配置保留 Windows NSIS 与签名更新；`tauri.macos.conf.json` 覆盖为 `.app`、禁用 updater 产物并使用 ad-hoc 签名。
- 2026-11-08，[提交 `f113fc8` 的 CI](https://github.com/LLMxPM/pinpage/actions/runs/42459516022) 全部通过：macOS 默认／Option 实验构建分别通过 Clippy、73 项单元测试、独立标识的 Debug `.app` 打包及 ZIP 上传；Windows 与前端任务也通过。
- 本次 runner 为 macOS 26.7.2、ARM64，Rust 1.99.1；对应构建产物与提交 SHA 可从运行页面核对，详见[构建证据](macos-development.md#已记录的构建证据)。
- CI 按 runner 原生架构构建，不宣称 Intel 与 Apple Silicon 均已验收；维护者仍使用 Windows 开发环境。
- 自动测试覆盖能力值、配置回退、预设过滤和界面提示；CI 通过也不代表 macOS 窗口交互已通过。
- macOS 仅提供开发验证流程，不加入正式 Release、`latest.json` 或 Windows 签名发布任务。

### macOS 待验收

- 交叉编译、启动、工作台与托盘创建。
- 浮动窗口的置顶、边框、尺寸、位置、缩放和找回。
- 默认构建的无边框降级提示与托盘／卡片找回；实验构建的 Option 拖动、网页点击隔离与退出清理。
- Retina、DPI 与多显示器布局校正。
- 导入页、示例、页面包导出导入、数据回填和语言切换。
- 从 Windows 配置目录迁移后的回退说明与首次启动结果。

维护者负责通用代码与 Windows 回归，Mac 协作者填写包含提交 SHA、芯片、系统版本和逐项结果的实机报告。环境、构建命令、CI 包使用方式和合并门槛见[macOS 开发与社区验收](macos-development.md)；GitHub 提供“macOS 能力验收”Issue 模板。只更新有证据的状态，不把某台机器上的通过扩大到其他架构。

## 扩展新平台

1. 在 `platform.rs` 为目标平台声明 `PlatformCapabilities`，并新增目标特定模块处理原生窗口策略。
2. 在 `validate_pin`、`validate_preset` 和 `apply` 中补齐支持范围；未声明能力必须拒绝显式提交。
3. 前端入口只用快照能力表裁剪，不增加按操作系统分支的逻辑。
4. 配置对应的 bundle target 与 CI 任务；只有实际支持更新时才接入该平台的更新清单。
5. 以明确关闭未实现能力为前提，先验收基础窗口，再按能力逐项扩展；记录证据后更新平台矩阵和[当前状态与验收](development.md#当前状态与验收)。
