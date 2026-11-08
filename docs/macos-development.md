# macOS 开发与社区验收

macOS 当前为实验性支持。维护者可以在 Windows 上维护通用逻辑，通过 macOS CI 检查编译和生成验证包；持有 Mac 的开发者负责窗口交互验收。能力范围见[平台支持](platforms.md)，本页提供可复现的开发步骤与反馈约定。

## 两种构建

| 构建 | 原生拖动 | 用途 |
| --- | --- | --- |
| 默认构建 | 不提供；新页面默认保留系统边框 | 验证标准浮动窗口与通用业务 |
| `experimental-macos-drag` | 固定 Option + 左键拖动无边框页面 | 单独验证 PR #1 接入后的原生手势 |

两种构建都不支持桌面模式、原生圆角、位置锁、大小锁、窗口右键菜单和应用内升级。实验开关不开放这些能力，也不开放手势修饰键设置。其他操作系统不会因启用此 feature 获得 macOS 能力。

Option 拖动改编自 [loikein 的 PR #1](https://github.com/LLMxPM/pinpage/pull/1)，保留现有通用窗口策略，增加了监视器退出注销和页面身份判断。原 PR 中发布工作流的改动未接入；Windows Release 继续使用现有签名升级流程。

## 环境准备

- 安装 Xcode Command Line Tools，可使用 `xcode-select --install`；有完整 Xcode 的开发者可按自己的环境配置。
- 安装 Rust，版本至少 1.90，使用与芯片匹配的原生工具链。
- 安装 Node.js 22 和 pnpm，pnpm 版本以 `package.json` 的 `packageManager` 为准。
- 运行 `pnpm install --frozen-lockfile`。

完整前置要求见 [Tauri 官方文档](https://v2.tauri.app/start/prerequisites/)。常规安装、开发和构建步骤使用普通用户权限；如果遇到权限错误，记录具体路径与日志并排查目录权限，不把 `sudo pnpm` 作为正常步骤。

## 源码开发

首次运行默认构建：

```sh
pnpm tauri dev
```

验证 Option 拖动时，先从托盘退出已有应用，再运行：

```sh
pnpm tauri dev --features experimental-macos-drag
```

开发服务固定使用 `127.0.0.1:1420`；启动前先确认是否已有该项目的开发进程。已有前端服务时可使用 `--config src-tauri/tauri.verification.conf.json` 复用服务并隔离配置。普通构建使用 `com.pinpage.desktop`，验证配置使用 `com.pinpage.verification`；单实例策略可能把第二次启动转交给第一个进程，不能据此判断实验构建已经生效。

默认构建的拖动提示说明需通过系统边框移动；实验构建应显示 Option 拖动提示，同时仍禁用桌面、圆角和几何锁。拖动验收应先覆盖普通鼠标或触控板按下拖动，不以系统三指拖动辅助功能作为前提。

## 自动检查

```sh
pnpm build
pnpm test:ui
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D clippy::all
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features experimental-macos-drag -- -D clippy::all
cargo test --locked --manifest-path src-tauri/Cargo.toml --features experimental-macos-drag
```

能力规则测试使用显式的完整／受限能力组合，避免以当前测试机为 Windows。平台声明测试单独检查默认与实验 feature。Windows 隐藏原生窗口集成测试只在 Windows 上运行；macOS 的 `cargo test` 通过不代表鼠标、窗口层级或 Retina 行为已验收。

## 本地打包与独立实例

`tauri.macos.conf.json` 在 macOS 自动合并：目标为 `.app`，禁用 updater 产物，使用本地 ad-hoc 签名。它不要求 Apple 开发者证书，也不修改 Windows 的 NSIS 与更新配置；此类验证包尚未经过 Developer ID 签名与公证。配置机制、打包和正式签名分别见 [Tauri 配置](https://v2.tauri.app/develop/configuration-files/)、[应用打包](https://v2.tauri.app/distribute/macos-application-bundle/)和[签名文档](https://v2.tauri.app/distribute/sign/macos/)。

为避免测试覆盖日常配置，推荐构建独立标识的调试应用：

```sh
pnpm build
CARGO_TARGET_DIR="$PWD/target/macos-default" pnpm tauri build --debug --config src-tauri/tauri.verification.conf.json --bundles app
open target/macos-default/debug/bundle/macos/PinPage.app
```

完全退出默认验证实例后，再检查实验拖动版本：

```sh
CARGO_TARGET_DIR="$PWD/target/macos-option" pnpm tauri build --debug --config src-tauri/tauri.verification.conf.json --bundles app --features experimental-macos-drag
open target/macos-option/debug/bundle/macos/PinPage.app
```

两个版本使用相同的验证标识，便于比较相同测试配置；它们必须依次运行。`target/` 已被忽略，不提交应用包、浏览器数据和本地配置。性能采样需另外移除 `--debug` 使用 Release 构建。

需要验证 DMG 时可将 `--bundles app` 改为 `--bundles dmg`。这不是正式发布流程。Universal 构建需要安装两个 Rust 目标并使用 `--target universal-apple-darwin`；通用二进制编译通过不能替代 Intel 和 Apple Silicon 各自的实机验收。

## 使用 CI 验证包

推送到 `main`、`feature/macos-*` 或提交 PR 时，CI 会分别运行 Windows、macOS 默认、macOS Option 实验构建。macOS 使用 runner 的原生架构，不宣称覆盖两种芯片；fork PR 的首次工作流可能需要维护者批准运行。

1. 打开对应提交的 Actions，确认两个 macOS 任务均通过，记录提交 SHA 与运行链接。
2. 下载 `pinpage-macos-default-<SHA>` 或 `pinpage-macos-option-<SHA>` artifact。
3. 解压 artifact 后再解压其中的应用 ZIP，查看 `build-info.txt` 中的架构、变体与标识。
4. 先退出其他验证实例，从应用包启动，再执行下面的实机清单。

应用 ZIP 使用 `ditto` 保留应用包信息与可执行权限。首次启动可能受系统下载来源检查影响；按系统提示处理并记录具体行为。构建未正式签名、公证的问题与应用自身启动异常应分别记录。

CI 提供的是独立标识的 Debug 验证包，不上传 GitHub Release，不生成 `latest.json`，也不能用于运行内存结论。只有对应 SHA 的任务实际通过后，才能记录“CI 已构建通过”。

## 实机验收清单

每次验收记录：提交 SHA、构建变体、来源（本地命令或 CI 链接）、macOS 版本、芯片、显示器数量与缩放。每一项填写“通过／失败／未测”，失败附最短复现步骤、预期与实际结果；日志中的个人路径、URL 和页面数据先脱敏。

| 项目 | 操作与预期 |
| --- | --- |
| 工作台与托盘 | 启动；关闭工作台后从托盘重开；退出后进程正常结束 |
| 页面生命周期 | 创建示例与 HTTPS 页面，关闭后保留记录，重开，删除只移除目标页面 |
| 浮动窗口 | 置顶、切换边框、修改尺寸与 X/Y、撤销；窗口读数、表单和重启结果一致 |
| 找回 | 隐藏或移出屏幕，再从托盘与页面卡片找回，恢复可操作的系统边框 |
| 受限能力 | 桌面、圆角和两把锁不可用；旧 Windows 配置回退后内容、开关与几何保留 |
| 来源与数据 | 中文／空格目录导入；CSS、模块、JSON；页面包导出导入与存储回填 |
| 显示器 | Retina、缩放、负坐标副屏、断开显示器后重开与找回 |
| 语言 | 中英文切换后的工作台与托盘；切换不重载页面；跟随系统目前回退中文 |
| Option 拖动（实验） | 无边框页面上 Option + 左键移动，网页不收到触发点击；普通点击正常 |
| 手势边界（实验） | 系统边框、工作台、混合修饰键、快速点击、窗口关闭与退出；不误吞点击或卡住事件循环 |

使用 GitHub 的“macOS 能力验收”Issue 模板提交结果，并关联相关 PR。没有条件测试的场景保持“未测”；单台 Apple Silicon 机器通过不能填写 Intel 通过。

## 协作与合并门槛

- 维护者负责能力契约、通用业务、Windows 回归和 CI；Mac 协作者负责平台实现与实机报告。
- 先将公开的平台基线同步进原 PR，保留贡献者提交与署名；不必关闭 PR #1 重新提同一份改动。若重建提交，保留原作者贡献记录。
- 基础支持可以在 Windows 回归、macOS CI 与核心实机验收通过后按实验性状态合并，不要求首版达到 Windows 的功能范围。
- Option 拖动仍通过 feature 显式验证；移除实验开关、默认声明该能力前，需记录包含普通左键拖动和手势边界的实机结果。
- 桌面模式、圆角和两把锁按能力分别实现、测试与验收，不能仅把能力布尔值改为 `true`。
- Developer ID 签名、公证、Universal 分发和 macOS 更新另行处理，避免阻塞 Windows 签名发布。

验收状态更新到[平台支持](platforms.md)和[当前状态与验收](development.md#当前状态与验收)，中英文 README 同步说明支持边界。
