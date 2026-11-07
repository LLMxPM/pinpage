# 开发与构建

## 环境要求

- Node.js 与 pnpm（当前验证环境：Node.js 22.23.4、pnpm 10.30.3）
- Rust MSVC 工具链（当前验证环境：Rust 1.98.1）
- Visual Studio Build Tools，包含桌面 C++ 工作负载
- WebView2 Runtime

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
  features/pins/          工作台、页面卡片与页面设置
  components/             通用组件
  services/               Tauri 调用封装
  i18n/                   前端多语言与消息映射
  styles/                 界面样式
  types/                  前端类型
src-tauri/src/
  commands/               前端命令入口
  windows/                窗口生命周期、事件与几何
  platform/               Windows 专用适配
  content.rs              URL 与本地资源协议
  import.rs               导入复制与站点副本
  package.rs              页面包导入导出
  examples.rs             内置示例
  data.rs                 页面数据采集与回填
  state.rs                配置读写与运行态
  tray.rs                 托盘
  shortcuts.rs            全局快捷键
  lib.rs / main.rs        应用装配与入口
locales/                  中英文界面与原生消息
examples/                 四个离线示例源码
scripts/                  文案检查与性能采样脚本
tests/                    UI 表单测试与本地路径样例
docs/                     用户、架构、开发与验证文档
```

按实际复杂度拆分模块，不为占位提前生成空文件。单个代码文件接近 300 行时检查是否需要拆分。

## 检查与测试

| 命令 | 覆盖范围 |
| --- | --- |
| `pnpm check:i18n` | 中英文资源的键、参数、原生资源语法，以及代码中引用的消息键是否落库 |
| `pnpm check` | TypeScript 类型检查（`vue-tsc --noEmit`） |
| `pnpm build` | 文案检查、类型检查与 Vite 生产构建 |
| `pnpm test:ui` | 设置表单回传合并、编辑保留与提交行为，不连接真实配置 |
| `pnpm test:rust` | Rust 单元测试与原生窗口集成测试 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Rust 格式检查 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D clippy::all` | Rust 静态检查 |
| `pnpm format:check` | 前端、语言资源、脚本、配置与样例文件的 Prettier 检查 |

只运行与改动相关的检查；既有检查通过后，除非发现新问题，不重复扩大测试范围。涉及 URL 校验、本地资源路径边界、状态迁移和窗口恢复时，优先补充对应测试。

窗口置顶、无边框拖动缩放、DPI 与多显示器行为必须在真实桌面环境验证，自动化测试不能替代这些场景。

## 打包

```powershell
pnpm tauri build
```

- Release 可执行文件：`src-tauri/target/release/pinpage.exe`
- NSIS 安装包：`src-tauri/target/release/bundle/nsis/`
- 构建产物不提交 Git。

直接运行可执行文件时可追加 `--tray`，只创建托盘并按设置恢复悬浮窗，工作台按需打开。

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

原始结果写入已忽略的 `runtime-data/memory-samples.jsonl`。读取指标时区分私有提交内存与工作集，说明共享内存口径，并同时记录系统、硬件、WebView2 版本与测量时长。首轮数据见[验证记录](verification.md)。

## 文档维护

- [README.md](../README.md)（英文）与 [README.zh-CN.md](../README.zh-CN.md)（中文）只保留项目概览、特点、快速开始与文档索引；使用细节、架构和开发说明放在 `docs/`。
- 功能、命令或验收结论变化时同步更新对应文档与中英文版本，并检查链接和内容一致性。
- [docs/screenshots](../docs/screenshots) 存放 README 与用户指南的界面截图，更新界面后按同名文件替换即可。
- 协作规范见 [AGENTS.md](../AGENTS.md)。
