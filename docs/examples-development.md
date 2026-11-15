# 示例开发与接入

本文用于创建、修改 `examples/` 页面、接入内置示例库，以及维护示例打包和验收。代码路径均相对仓库根目录，协作入口为根 [AGENTS.md](../AGENTS.md) 与 [examples/AGENTS.md](../examples/AGENTS.md)。只要求页面原型或独立分发包时，无需注册到内置示例库。

## 公共规范与仓库流程

页面规范统一维护在 [pinpage-pages](../skills/pinpage-pages/SKILL.md) 技能中，仓库直接引用，不复制正文或生成另一份宿主契约：

| 当前任务                                          | 阅读材料                                                                                                  |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| 页面工程、视觉交互、尺寸、透明、联网与状态保存    | [页面制作指南](../skills/pinpage-pages/references/page-development.md)                                    |
| 环境信息、系统采样、文件夹授权、能力探测与 CSP    | [宿主能力契约](../skills/pinpage-pages/references/host-api.md)                                            |
| `manifest.json`、默认布局、命名预设与数据迁移格式 | [页面包文件格式](../skills/pinpage-pages/references/package-format.md)                                    |
| 安装运行副本、窗口操作、诊断与桌面截图            | [CLI 手册](../skills/pinpage-pages/references/cli.md)，连接开发应用见 [CLI 指南](agent-cli.md#获取与连接) |

公共技能的 CLI 流程用于向运行中的应用交付页面；仓库示例的交付物是源码清单和静态站点，由 Rust 打包器生成分发包。仅编写源码不需要连接应用。使用 CLI 调整布局或制作预设后，应将用于分发的参数写回源码清单，不能只保留在本机运行页中。

## 标准包源码与分发

新工具使用 `examples/<工具名>/manifest.json` 和 `site/`。清单入口相对于 `site/`，通常为 `index.html`，也可使用安全子目录；源码清单可省略 `files` 和 `exportedAt`，打包器会统计文件数并固定示例时间戳。字段和数值约束以公共页面包参考为准。

- 示例是独立静态站点，不经过工作台 Vue/Vite 构建。优先使用浏览器原生 HTML、CSS 和 JavaScript，运行文件不直接引用工作台 `src/` 或依赖包名。
- 示例的脚本、样式和必要资源随站点提供；时间工具、便签与待办、日历和看板离线可用，文件夹与监控使用宿主能力。[自选行情](../examples/stocks/README.md)已接入内置示例库并明确标记需要联网，按公共制作指南处理数据来源、CORS、缓存和失败状态。
- 页面内容默认中文，不随工作台语言切换。示例库卡片标题和新建页面名称属于宿主文案，需要维护中英文资源。
- `src-tauri/build/examples.rs` 递归收集 `site/` 下的真实文件，保留相对目录，生成可复现的标准 `.pinpage`。新增 CSS、模块、JSON 和图片自动打包；不维护逐文件 Rust 映射，不使用符号链接或目录联接。
- `pnpm examples:pack` 将包写到 `dist/examples/`，用于独立导入和分享。不要混入其他工具资源或个人存储快照，不提交生成包。
- `src-tauri/build.rs` 使用同一打包器，在 Rust 构建输出目录生成包和内嵌索引；前端构建不能更新应用内置包。
- 示例与外部包共用 `src-tauri/src/package.rs` 的清单校验和解包。不要为新增工具恢复专用复制逻辑或绕过路径边界。

## 参考实现与宿主接入

按任务选择最接近的现有实现：

| 示例                                                      | 参考内容                                 |
| --------------------------------------------------------- | ---------------------------------------- |
| [计时器](../examples/timer/site/index.html)               | ES 模块、JSON、截止时间和 IndexedDB      |
| [便签](../examples/notes/site/notes.js)                   | 编辑即保存                               |
| [待办](../examples/notes/site/tasks.js)                   | 旧数据迁移、编辑、排序和撤销删除         |
| [时钟](../examples/timer/site/clock.js)                   | 基于真实时间刷新                         |
| [文件夹](../examples/folder/site/controller.js)           | 能力探测、逐页授权、导航竞态、打开和定位 |
| [系统监控](../examples/system-monitor/site/controller.js) | 共享采样串行轮询、时效判断与演示降级     |
| [日历与倒数日](../examples/calendar/site/date-state.js)   | 本地日历日计算与跨夏令时边界             |
| [轻量看板](../examples/kanban/site/board-state.js)        | 卡片校验和状态流转                       |
| [自选行情](../examples/stocks/site/list-model.js)         | 缓存名称筛选、展示排序与联网缓存         |

宿主接入可复用 [folder-api.js](../examples/folder/site/folder-api.js) 或 [monitor-api.js](../examples/system-monitor/site/monitor-api.js) 的薄封装；调用方式、能力版本、错误结构、权限和出站限制以公共宿主契约为准，不自行发明 IPC 通道。普通浏览器中的演示数据应明确标注。

维护宿主实现时，对照 `src-tauri/src/host.rs`、`folder.rs`、`pathguard.rs` 与 `commands/`，权限声明在 `src-tauri/capabilities/page-host.json`、`page-folder.json`。消息码对应 `locales/*/native.json` 的资源键；接口或能力变化时直接更新公共契约，并同步权限和中英文消息。

## 注册与展示

新增内置示例时维护以下位置；仅修改已有页面内容时无需重复注册：

| 位置                                                     | 需要维护的内容                                                |
| -------------------------------------------------------- | ------------------------------------------------------------- |
| `src-tauri/src/examples.rs`                              | `ExampleId` 白名单、标识到标准包的选择和 `label_key()` 名称键 |
| `src/types/index.ts`                                     | 与 Rust 序列化值一致的 `ExampleId` 联合类型                   |
| `src/features/launcher/examples.ts`                      | 稳定 ID 与 `ui.examples.*` 标题键                             |
| `src/features/launcher/ExampleGallery.vue`               | 工具对应的轻量预览，避免新增 ID 意外落入时钟兜底分支          |
| `src/styles/examples.css`                                | 预览样式及新增卡片后的网格适配                                |
| `locales/zh-CN/ui.json`、`locales/en-US/ui.json`         | `examples` 下的宿主卡片标题                                   |
| `locales/zh-CN/native.json`、`locales/en-US/native.json` | `names` 下的新建页面名称，与 `label_key()` 对齐               |
| `src-tauri/src/examples_tests.rs`                        | 将新工具加入标准包往返与资源完整性验证的枚举清单              |

Rust 枚举使用 `#[serde(rename_all = "camelCase")]`，前端 ID 与实际序列化值一致；源码目录和包索引名与 Rust 选择映射一致，不要混淆多词 ID 的大小写和短横线命名。

卡片沿用 HTML/CSS 或图标示意，不为预览创建额外 WebView 或 iframe。添加使用 `src/features/pins/useWorkspace.ts` 的 `createExample()`，不需要改导入协议或自行创建原生窗口。

## 随包布局与独立副本

在源码清单中提供适合工具的 `windowPreset`，需要有用的布局变体时提供带稳定页内 ID 的 `windowPresets`。两者独立，默认布局不会自动成为命名模板；格式、顺序与完整字段见[窗口布局字段](../skills/pinpage-pages/references/package-format.md#窗口布局字段)。不要把作者的页面 ID、绝对屏幕坐标或应用设置写进包。

示例库添加直接使用随包布局和列表。手动导入包可以分别取消列表和默认布局；包中的布局不写入全局预设库。首次导入和后续更新语义见[页面包与示例制作](page-packages.md)。

每次添加生成独立副本和页面来源，可复用同一工具的存储键。修改源码或升级应用不会覆盖旧副本；验证新版时重新添加或重新导入，保留用户数据。相同页面更换来源仍保留同源存储，数据结构修改需要兼容旧数据。

合并后的规范入口为 `timer`（时间工具）和 `notes`（便签与待办）；旧 `clock`、`checklist` 白名单标识映射到对应新包，不重复分发旧源码。新页面读取旧计时、时钟、便签和待办的存储键，只能迁移当前页面来源中的数据，不能跨独立副本自动合并。窗口预设只管理原生布局，计时模式、同屏视图和外观偏好由页面分别保存。

## 按改动验证

先检查并复用已有开发服务，静态预览可访问 `/examples/<工具名>/site/index.html`；使用模块或 `fetch` 时不要用 `file://` 判断加载结果。按目录导入选择 `site/`，只有按页面包导入才携带源码清单的布局和预设。

技能和文档维护只检查结构、链接与内容一致性。实际修改页面时按范围选择：

| 改动                         | 相关检查                                                                                                                     |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| 技能、公共参考或仓库接入文档 | `pnpm check:skill`；检查改动文档与格式                                                                                       |
| 页面资源                     | `pnpm exec prettier --check <改动文件...>`；纯 JS 可用 `node --check <文件>` 检查语法，预览核心交互                          |
| 示例状态、日期和行情逻辑     | `pnpm test:examples`：覆盖旧数据恢复、真实截止时间、日期边界、状态流转、行情解析与请求竞态                                   |
| 卡片、类型与宿主文案         | `pnpm check`；新增宿主文案时运行 `pnpm check:i18n`                                                                           |
| 包源码、注册或打包器         | `pnpm examples:pack`；`cargo test --manifest-path src-tauri/Cargo.toml --lib examples::tests`                                |
| 包格式或随包布局             | `cargo test --manifest-path src-tauri/Cargo.toml --lib package::tests` 及相关预设、来源测试                                  |
| 原生能力页面                 | `cargo test --manifest-path src-tauri/Cargo.toml --lib` 覆盖 pathguard／folder／host；按公共宿主契约检查原生授权、CSP 和采样 |
| 构建与跨模块接入             | 按需要运行 `pnpm build`、Rustfmt 和 Clippy；前端构建不能替代 Rust 包生成和原生验收                                           |

页面内容验证按[页面制作指南](../skills/pinpage-pages/references/page-development.md)覆盖本次改动涉及的布局、交互和状态场景；原生能力按[宿主契约的联调与降级要求](../skills/pinpage-pages/references/host-api.md#联调与降级)验收。

内置示例需在对应新构建中添加，并独立导入生成包，检查资源完整性、卡片、中英文名称、默认布局与列表的选择和取消、两个副本的数据隔离。有持久化内容时检查运行中导出后导入。现有资源测试检查 HTML 直接引用，新增 CSS `url()` 或 JS 动态读取仍需验证实际加载。

分别记录 CSS 模拟、浏览器预览与真实 Windows WebView2 桌面验收。原生授权选择器、取消授权、根失效与越界拒绝、CSP 响应头和真实采样需要实机检查；透明、DPI、多显示器和桌面模式同样只报告实际覆盖项。使用 CLI 时打开并检查真实截图，记录见[当前状态与验收](development.md#当前状态与验收)。

## 文档与交付

交付时说明源码位置、是否接入示例库、预览或添加方法、检查结果与未验证项。更新[用户指南的示例库](user-guide.md#示例库)中的用途和入口；数量、格式、功能或验收范围变化时更新[开发与构建](development.md)及相关文档，概览变化同步 [README.md](../README.md) 与 [README.zh-CN.md](../README.zh-CN.md)。
