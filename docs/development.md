# 开发与构建

## 环境要求

- Node.js 与 pnpm（当前验证环境：Node.js 22.23.4、pnpm 10.30.3）
- Rust MSVC 工具链，最低 1.90（当前验证环境：Rust 1.98.1）
- Visual Studio Build Tools，包含桌面 C++ 工作负载
- WebView2 Runtime

当前完整支持 Windows x64；macOS 与其他平台的能力边界见[平台支持](platforms.md)。

## 安装与启动

```powershell
pnpm install
pnpm tauri dev
```

开发地址固定为 `http://127.0.0.1:1420`。`pnpm tauri dev` 会启动前端服务；如果本项目的服务已在运行，先确认现有进程，避免重复启动。

仅预览工作台可运行 `pnpm dev`。浏览器中不能创建原生悬浮窗，原生窗口行为必须通过 Tauri 应用验证。

## 目录结构

```text
src/
  features/launcher/      添加页面、来源导入
  features/pins/          工作台、即时窗口设置、独立内容保存与共用预设管理器
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
  windows/presets/         内置定义、相对几何、预设读写及测试
  platform.rs / platform/ 跨平台能力声明、回退校验与 Windows 专用适配
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
examples/                 四个离线示例的 manifest.json + site/ 标准包源码
scripts/                  版本同步、发布、签名构建、文案检查与性能采样
tests/                    UI 表单测试与本地路径样例
docs/                     用户、架构、开发、平台与发布文档
```

按实际复杂度拆分模块，不为占位提前生成空文件。单个代码文件接近 300 行时检查是否需要拆分。

## 创建示例库页面

项目内技能 [pinpage-examples](../.agents/skills/pinpage-examples/SKILL.md) 说明独立工具页的资源边界、透明背景、小窗口布局与状态保存约束，并提供接入内置示例库的文件清单和验证指引。在本仓库中可使用 `$pinpage-examples` 发起页面创建或修改任务。

页面包清单和默认窗口布局见[页面包与示例制作](page-packages.md)。运行 `pnpm examples:pack` 在 `dist/examples/` 生成标准 `.pinpage`；Rust 构建自动使用同一打包器生成内嵌包，`site/` 中的资源按相对目录自动收集。修改 Rust 构建内嵌的示例后需使用对应新构建验证，已添加的独立副本不会自动更新。

## 检查与测试

| 命令 | 覆盖范围 |
| --- | --- |
| `pnpm examples:pack` | 将标准示例源码打包到 `dist/examples/`，不启动桌面应用 |
| `pnpm check:i18n` | 中英文资源的键、参数、原生资源语法，以及代码中引用的消息键是否落库 |
| `pnpm check:limits` | 前端与原生窗口尺寸上下限是否一致 |
| `pnpm check` | TypeScript 类型检查（`vue-tsc --noEmit`） |
| `pnpm build` | 文案与尺寸限制检查、类型检查及 Vite 生产构建 |
| `pnpm test:ui` | 设置表单回传合并、连续输入、失败保留、另存前等待、操作队列及平台能力过滤与回退，不连接真实配置 |
| `pnpm test:rust` | Rust 单元测试与原生窗口集成测试 |
| `pnpm version:check` | 前端、Tauri、Cargo 与 Cargo 锁文件的应用版本一致性 |
| `pnpm test:release` | 版本同步、标签约束与 GitHub 更新清单 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Rust 格式检查 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D clippy::all` | Rust 静态检查 |
| `pnpm format:check` | 前端、语言资源、脚本、配置与样例文件的 Prettier 检查 |

只运行与改动相关的检查；既有检查通过后，除非发现新问题，不重复扩大测试范围。涉及 URL 校验、本地资源路径边界、状态迁移和窗口恢复时，优先补充对应测试。

窗口置顶、无边框拖动缩放、DPI 与多显示器行为必须在真实桌面环境验证，自动化测试不能替代这些场景。

## 当前状态与验收

截至 2026-11-08，应用版本 `0.2.0`，配置格式 `schemaVersion = 1`。Windows x64 [v0.2.0 已正式发布](https://github.com/LLMxPM/pinpage/releases/tag/v0.2.0)，为完整支持平台；macOS 仅声明标准浮动窗口，尚未交叉编译或实机验收，详见[平台支持](platforms.md)。历史改动不再逐条保留，以 Git 提交和[版本与升级](releases.md)为准。

### 已实现

- **页面来源**：HTTP/HTTPS、目录与单文件导入、页面包导入导出、四个内置示例。
- **窗口**：浮动与桌面模式、置顶、系统边框、无边框圆角、独立位置锁和大小锁、窗口预设、修饰键拖动与右键菜单、会话恢复。
- **数据与安全**：工作台与内容页独立 WebView、命令与资源协议边界、localStorage／sessionStorage／IndexedDB 页面包、配置原子写入与跨平台回退。
- **系统集成**：托盘、按需工作台、中英文界面、GitHub Release 应用内更新、可配置的 Windows 登录自启（未发布）。

### 2026-11-10 背景透明改动（未发布）

- Windows 新增逐页 `transparentBackground`，旧配置与旧预设默认关闭；窗口设置、预设、撤销、跨平台能力回退与找回入口均接入。透明切换不重建、不刷新内容 WebView。
- 新添加的四个示例通过 CSS 变量适配背景透明；旧示例副本不会自动改写。第三方页面需自行提供透明背景，不支持透明的平台拒绝显式开启并在加载旧配置时关闭该偏好。
- 自动检查通过：`pnpm build`（含 219 个 UI 文案键校验）、`pnpm test:ui`（26 项）、Rust 测试（88 项单元测试与 6 项原生窗口集成测试）、Clippy，以及相关源文件的 Prettier / Rustfmt 检查。
- 独立实例使用 `com.pinpage.transparency-verification`，环境为 Windows 11 企业版 `10.0.26300`、WebView2 `157.0.4341.42`、Debug 构建。通过调试接口确认：逐页透明切换、Vue 设置入口到原生事务、计时器内存状态保留、透明预设保存与应用、撤销及过期拒绝、刷新沿用最新外观、找回关闭透明、关闭页保存后再开启、配置落盘及应用重启恢复。锁住配置文件制造保存失败时，配置与当前文档 CSS 同时回滚。英文 `760 × 620` 的 WebView 渲染截图中新增入口未溢出。
- **验收边界**：桌面自动化截图返回黑屏，鼠标接口返回 `GetCursorPos failed: 拒绝访问 (0x80070005)`，调试接口检查不能代替真实画面验收。当前桌面时钟的透明效果已由用户确认；其他页面、半透明面板、不同 WebView2 版本、DPI、多显示器、透明区域命中与手势、资源变化仍待实机验收。
- **桌面黑底跟进**：用户已确认当前 Windows 桌面时钟能够透明，Explorer 重启后恢复也通过。透明桌面页改为桌面拥有的底层顶层窗口，普通桌面页继续子窗口挂载；同时保护普通桌面页的子窗口样式，并在模式变化时明确刷新 WebView2 背景。原生回归覆盖 Tao 显示/隐藏样式重写、拥有窗口与子窗口往返、透明样式保留，以及拥有关系下屏幕坐标和几何锚点。
- **显示桌面跟进**：修正首次显示丢失 `WS_POPUP`、与模式切换后身份不一致的问题；在样式重写时保护透明桌面页的弹出窗口身份，切回浮动时解除保护，6 项原生集成测试通过，重建后的真实窗口样式也已确认。用户已确认当前开发构建中，关闭再开启透明桌面时钟后执行“显示桌面”，页面仍然可见。此结论限于当前实机与页面；其他 Shell 场景、页面与运行时版本、DPI、多显示器和长时间稳定性仍待验收。

### 2026-11-10 开机自启改动（未发布）

- 「设置 → 通用」新增默认关闭的「开机自启」，点击「保存设置」后生效；登录 Windows 时以 `--tray` 启动，不打开工作台，是否恢复页面沿用独立的会话恢复设置。重复 `--tray` 启动也不唤起已有工作台。
- 通过 Windows 平台模块注册当前用户的启动项，完整可执行路径始终加引号；快照读取系统实际状态，不把自启注册写入可迁移配置。未编辑开关时不修改系统注册；其他平台禁用开关并显示说明。
- 自动检查通过：`pnpm build`（ui 222 / native 173 文案键校验）、8 项设置事务测试、Clippy，以及相关源文件的 Prettier / Rustfmt 检查。事务用例覆盖开启与关闭、重复保存、读取及注册失败、配置保存失败的双向回滚、回滚失败报告和只读保护。
- 独立 Release 实例使用 `com.pinpage.autostart-verification`。通过工作台调试接口验证中文开启、英文关闭、未保存时不注册、重启后读取已开启状态；注册命令中的中文与空格路径已核对为完整带引号路径。阻断独立配置临时文件写入时，系统开关和配置保持旧状态，表单保留失败草稿。首次和重复 `--tray` 启动均未创建工作台 WebView，之后手动启动可重新打开工作台。验证结束已关闭测试启动项并退出独立实例。
- **验收边界**：桌面点击仍返回 `GetCursorPos failed: 拒绝访问 (0x80070005)`，以上调试接口检查不能代替真实鼠标与画面验收。实际注销或重启登录后的系统触发、任务管理器禁用后再开启、安装卸载后的启动项行为仍待实机验收。

### 2026-11-10 标准示例包改动（未发布）

- 四个示例改为 `manifest.json + site/` 的标准包源码，Rust 构建和 `pnpm examples:pack` 共用打包器；构建内嵌的是 `.pinpage`，添加时与外部包共用清单校验和解包，支持资源子目录。旧示例副本不自动覆盖。
- 包清单新增可选 `windowPreset`，携带外观、逻辑尺寸与归一化位置；新建导入时可选择使用，示例库直接采用。导出保存当前布局；旧包仍可导入，更换既有来源保留窗口设置，包不会自动创建全局预设。跨平台不支持的能力回退，无拖动手势的平台保留边框。
- 自动检查通过：`pnpm build`（ui 223 / native 173 文案键校验）、21 项相关 Rust 测试（页面包、内置示例、预设和来源）、Clippy，以及相关前端源文件的 Prettier / Rustfmt 检查。`pnpm examples:pack` 已生成四个可独立导入的包；测试覆盖内存与磁盘包往返、可复现打包、资源完整性、非法预设拒绝和平台回退。
- **验收边界**：新建导入的预设勾选与取消、实际窗口初始位置与外观、当前窗口导出再导入，以及不同 DPI / 多显示器下的布局仍待桌面验收。上述包与纯逻辑测试不等同于原生窗口验收。

### 最近自动检查

以下为 2026-11-08 最近一次相关检查的记录，分布在不同改动上，未在单一提交上重跑全部命令。

| 检查 | 结果 |
| --- | --- |
| `pnpm build` | 通过：中英文 ui 217 键 / native 169 键校验、尺寸限制、Vue/TypeScript 与 Vite 生产构建 |
| `pnpm test:ui` | 24 项通过：设置表单回传合并、连续输入、失败保留、另存前等待、操作队列、平台能力过滤与回退 |
| `pnpm test:rust` | 85 项单元测试与 3 项隐藏原生窗口集成测试通过 |
| `pnpm check` / `pnpm check:i18n` / `pnpm check:limits` | 通过；类型检查、中英文键与参数、前后端尺寸上下限一致 |
| `pnpm test:release` | 5 项通过：版本同步、标签约束、更新清单边界、草稿分页查询与 API 错误中止 |
| `pnpm release:build` | 本地 NSIS 安装包与 `.sig` 生成成功；Windows 代码签名未配置 |
| `pnpm format:check` / `cargo fmt --check` | 通过 |
| `cargo clippy --all-targets -- -D clippy::all` | 通过 |

2026-11-08，[主分支 CI](https://github.com/LLMxPM/pinpage/actions/runs/42462223685) 与 [v0.2.0 发布工作流](https://github.com/LLMxPM/pinpage/actions/runs/42462225046) 均通过。四项 Release 附件已公开，固定更新入口返回 `0.2.0`；重新下载的安装包通过 SHA-256、离线签名及版本绑定校验，篡改包与错误版本被拒绝。发布时修复了只查询已发布版本而读不到草稿的问题，保留原标签并使用工作流版本的发布脚本完成补发。安装交互、应用内升级与重启数据保留仍待验收。

### 已完成桌面验收

使用独立标识的验证实例完成，未修改正常应用会话：

- 窗口设置即时生效：宽度 420 → 520 后窗口、表单和配置文件一致；拖动窗口后坐标同步；撤销尺寸调整恢复原宽度。
- 预设管理：另存、改名与删除模板；模板改名和改宽不改变现有窗口；删除后页面记录和开关保持。
- 已关闭页面的置顶偏好可保存，`enabled` 保持 `false`，不创建内容窗口。
- 早期版本完成内置计时器与 HTTPS 页面渲染、无边框缩放与找回、管理窗口重建、1 / 3 / 5 页会话恢复，以及中英文原生标题验证。

以上结果来自 125% DPI 的 Windows 桌面；早期版本结果不能直接视为当前构建的完整回归。

### 待验收

- **窗口手势**：真实鼠标的持续拖动、快速点击、越界松开、Esc、混合按键、捕获丢失、手势中关闭或重建窗口、系统菜单冲突；连续 Alt+右键、Alt 释放后的菜单时延与常驻；网页 pointer / mouse / contextmenu 事件隔离；有边框、无边框、锁定、桌面、遮挡、负坐标多屏与各 DPI 下的菜单定位。
- **窗口层级与几何**：悬浮页隐藏任务栏、工作台保留任务栏入口的实际效果；桌面层显示、显示桌面后的交互、Explorer 重启恢复、原生标题栏关闭；「带到最前」对被遮挡、最小化和完全离屏窗口的最终层级与焦点；80 宽窗口的鼠标缩放下限；5120 超宽屏的全屏预设；跨显示器与 150% / 200% DPI 的预设和贴边偏差；桌面与浮动往返的闪烁和父窗口重建。
- **设置与预设**：来源重新导入、落盘失败回滚、覆盖更新和过期撤销的完整桌面交互；预设切换后继续撤销字段调整的组合操作；英文和窄窗口下的预设管理器分支；设置单页章节在真实 WebView2、不同 DPI 与窄窗口下的滚动定位。
- **导入与页面包**：目录导入 → 切换边框 → 移动 → 删除 → 副本清理；导出 → 删除 → 导入还原 → 数据回填；内容页数据回传通道；含 IndexedDB 的复合主键与 Blob 还原。
- **更新与安装**：从已发布 GitHub Release 完成旧版 → 下载 → 验签 → 安装 → 重启；安装取消、权限拒绝、installer 启动后失败；重启后的页面开关、几何、导入副本和登录数据；干净环境安装与卸载；Windows 代码签名。
- **多语言**：真实设置保存语言后的托盘、tooltip 与通知重渲染；管理页重建、重启和 `--tray` 启动的一致性；切换语言不重载内容页；英文在 760 × 620 与 100% / 150% / 200% DPI 的排版；安装器语言选择。
- **性能**：使用优化后的计时器按 Release 构建重采样 1 / 3 / 5 页；复杂看板、长时间运行和创建销毁循环的资源趋势。统计口径见[架构与数据](architecture.md#性能与资源)。
- **平台**：macOS 的构建、窗口、托盘、DPI 与多显示器行为，详见[平台支持](platforms.md#macos-待验收)。
- **常见场景**：真实看板的登录、断网重连、睡眠恢复；多显示器与 DPI 的组合切换。

## 打包

```powershell
pnpm release:build
```

- Release 可执行文件：`src-tauri/target/release/pinpage.exe`
- NSIS 安装包：`src-tauri/target/release/bundle/nsis/`
- 构建产物不提交 Git。

当前只配置 Windows NSIS 目标；本地签名构建与离线验签命令见[版本与升级](releases.md)，其他平台的构建限制见[平台支持](platforms.md#构建与验收状态)。Windows 安装包代码签名暂未配置。

直接运行可执行文件时可追加 `--tray`，只创建托盘并按设置恢复悬浮窗，工作台按需打开。

## 持续集成与发布

`.github/workflows/ci.yml` 在推送到 `main` 和合并请求时运行：

- 前端（Ubuntu）：Prettier、文案检查、类型检查、UI 测试与生产构建。
- Rust（Windows）：Rustfmt、Clippy、单元测试与原生窗口集成测试。

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
$env:CARGO_TARGET_DIR = "target/i18n-verify"
pnpm tauri build --config src-tauri/tauri.verification.conf.json --no-bundle
```

验证实例使用独立标识（`com.pinpage.verification`）与独立配置目录，产物写入自定义目标目录；验证结束后删除验证目录，不修改正常应用的配置。

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
- 平台能力、回退规则或 macOS 验收结论变化时，同步更新[平台支持](platforms.md)与本文「当前状态与验收」。
- [docs/screenshots](../docs/screenshots) 存放 README 与用户指南的界面截图，更新界面后按同名文件替换即可。
- 协作规范见 [AGENTS.md](../AGENTS.md)。
