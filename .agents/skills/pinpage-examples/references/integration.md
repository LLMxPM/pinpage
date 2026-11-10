# 接入内置示例库

仅在新增内置示例、修改打包方式或验证完整添加流程时读取。代码路径均相对仓库根目录。

## 标准包源码与分发

按 [页面包与示例制作](../../../../docs/page-packages.md) 创建 `examples/<工具名>/manifest.json` 和 `site/`。清单入口相对于 `site/`，可使用安全子目录；源码清单的文件数与时间戳由打包器补充。不要把多个工具的资源混入同一个站点，或随示例提交个人存储快照。

- `src-tauri/build/examples.rs` 递归收集每个站点的真实文件，固定顺序和示例时间戳，生成标准 `.pinpage`。新增 CSS、模块、JSON 和图片会自动打包；不需要添加逐文件 Rust 映射。
- `pnpm examples:pack` 将相同的包写到 `dist/examples/`，用于独立导入和分享；不提交生成包或其他构建产物。
- `src-tauri/build.rs` 使用同一打包器，在 Rust 构建输出目录生成包和内嵌索引。只运行前端构建不能更新应用中的内置包。
- 示例与外部包共用 `src-tauri/src/package.rs` 的清单校验和解包，支持资源子目录。不要为新增工具恢复专用资源复制逻辑或绕过解包边界。

## 注册与展示

| 位置 | 需要维护的内容 |
| --- | --- |
| `src-tauri/src/examples.rs` | `ExampleId` 白名单、标识到标准包的选择和 `label_key()` 名称键 |
| `src/types/index.ts` | 与 Rust 序列化值一致的 `ExampleId` 联合类型 |
| `src/features/launcher/examples.ts` | 稳定 ID 与 `ui.examples.*` 标题键 |
| `src/features/launcher/ExampleGallery.vue` | 工具对应的轻量预览，避免新增 ID 意外落入时钟兜底分支 |
| `src/styles/examples.css` | 预览样式及新增卡片后的网格适配 |
| `locales/zh-CN/ui.json`、`locales/en-US/ui.json` | `examples` 下的宿主卡片标题 |
| `locales/zh-CN/native.json`、`locales/en-US/native.json` | `names` 下的新建页面名称，与 `label_key()` 对齐 |
| `src-tauri/src/examples_tests.rs` | 将新工具加入标准包往返与资源完整性验证的枚举清单 |

Rust 枚举使用 `#[serde(rename_all = "camelCase")]`，前端 ID 与实际序列化值一致；源码目录和包索引名与 Rust 中的选择映射一致，不要混淆多词 ID 的大小写和短横线命名。

卡片使用现有 HTML/CSS 或图标示意，不为预览创建额外 WebView 或 iframe。添加沿用 `src/features/pins/useWorkspace.ts` 的 `createExample()`，不需要改导入协议或自行创建原生窗口。

## 随包窗口布局

清单的可选 `windowPreset` 复用宿主预设字段。宽高为逻辑像素，`fx`、`fy` 是 `0～1` 的相对工作区位置；可包含透明背景、边框、置顶、桌面模式及独立几何锁。非法数值会被拒绝，不支持的平台能力在初始化新页面时回退。

示例库添加使用随包布局；手动导入包时可取消「使用随包窗口预设」。包中的布局不自动写入全局预设库，更换已有页面的来源不覆盖其窗口设置。导出时保存当前窗口布局；旧包没有该字段时保留创建表单的行为。

每次添加生成独立副本和来源。修改源码或升级应用不会覆盖旧副本；验证新版时重新添加，保留用户数据。相同页面更换来源仍保留存储，数据结构修改需要兼容旧数据。

## 按改动验证

技能和文档维护只检查结构、链接与内容一致性。实际修改页面时按范围选择：

| 改动 | 相关检查 |
| --- | --- |
| 页面资源 | `pnpm exec prettier --check <改动文件...>`；纯 JS 可用 `node --check <文件>` 检查语法，预览核心交互 |
| 卡片、类型与宿主文案 | `pnpm check`；新增宿主文案时运行 `pnpm check:i18n` |
| 包源码、注册或打包器 | `pnpm examples:pack`；`cargo test --manifest-path src-tauri/Cargo.toml --lib examples::tests` |
| 包格式或随包布局 | `cargo test --manifest-path src-tauri/Cargo.toml --lib package::tests` 及相关预设、来源测试 |
| 原生能力页面 | `cargo test --manifest-path src-tauri/Cargo.toml --lib` 覆盖 pathguard／folder／host；浏览器预览只验演示降级与布局，授权模态、越界拒绝、CSP 与采样数值按[原生能力契约](native-capabilities.md)列入实机验收 |
| 构建与跨模块接入 | 按需要运行 `pnpm build`、Rustfmt 和 Clippy；前端构建不能替代 Rust 包生成和原生验收 |

在对应新构建的应用中添加示例和独立生成的包，验证核心操作、资源完整性、默认布局选择与取消、两个副本的数据隔离。有持久化内容时检查刷新、关闭再开启，以及运行中导出后导入；有计时功能时覆盖休眠校正。现有资源测试检查 HTML 直接引用，新增 CSS `url()` 或 JS 动态读取时仍需验证实际加载。

分别记录 CSS 模拟、浏览器预览与真实 Windows WebView2 桌面验收；透明背景、原生位置、DPI 和桌面模式不能仅靠包往返测试判定通过。

## 文档收尾

更新 [用户指南的示例库](../../../../docs/user-guide.md#示例库) 中的用途和预览入口。数量、格式、功能或验收范围变化时，更新 [开发与构建](../../../../docs/development.md) 及相关文档；概览变化同步 [README.md](../../../../README.md) 与 [README.zh-CN.md](../../../../README.zh-CN.md)，不将未执行的验证写成已通过。
