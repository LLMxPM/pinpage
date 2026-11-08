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
  examples.rs             内置示例
  data.rs                 页面数据采集与回填
  state.rs                配置读写与运行态
  tray.rs                 托盘
  lib.rs / main.rs        应用装配与入口
locales/                  中英文界面与原生消息
examples/                 四个离线示例源码
scripts/                  版本同步、发布、签名构建、文案检查与性能采样
tests/                    UI 表单测试与本地路径样例
docs/                     用户、架构、开发、平台与发布文档
```

按实际复杂度拆分模块，不为占位提前生成空文件。单个代码文件接近 300 行时检查是否需要拆分。

## 检查与测试

| 命令 | 覆盖范围 |
| --- | --- |
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

截至 2026-11-08，应用版本 `0.2.0`，配置格式 `schemaVersion = 1`。Windows x64 为完整支持平台；macOS 仅声明标准浮动窗口，尚未交叉编译或实机验收，详见[平台支持](platforms.md)。历史改动不再逐条保留，以 Git 提交和[版本与升级](releases.md)为准。

### 已实现

- **页面来源**：HTTP/HTTPS、目录与单文件导入、页面包导入导出、四个内置示例。
- **窗口**：浮动与桌面模式、置顶、系统边框、无边框圆角、独立位置锁和大小锁、窗口预设、修饰键拖动与右键菜单、会话恢复。
- **数据与安全**：工作台与内容页独立 WebView、命令与资源协议边界、localStorage／sessionStorage／IndexedDB 页面包、配置原子写入与跨平台回退。
- **系统集成**：托盘、按需工作台、中英文界面、GitHub Release 应用内更新。

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
