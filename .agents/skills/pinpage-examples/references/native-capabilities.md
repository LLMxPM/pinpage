# 页面原生能力契约

内容页是不受信任页面。宿主按分级提供只读能力：页面不获得 Shell、窗口管理或任意文件系统权限，也不为此放宽工作台 CSP 与资源协议边界。本文是能力层与页面作者之间的契约；实现位于 `src-tauri/src/host.rs`、`folder.rs`、`pathguard.rs` 与 `commands/`，权限声明在 `src-tauri/capabilities/page-host.json`、`page-folder.json`。

## 分级模型

| 级别 | 能力 | 授权方式 | 现状 |
| --- | --- | --- | --- |
| L0 环境信息 | `host_info`、`system_stats` | 默认可用，只读聚合 | 已提供 |
| L1 逐页授权 | `folder_pick`、`folder_list`、`folder_open`、`folder_reveal` | 原生选择器逐页确认，根目录边界 | 已提供，只读 |
| L2 更高权限 | 写入、删除、事件推送、通知、剪贴板等 | 需另行设计授权与审计 | 暂不提供 |

- 所有命令先校验调用方是已登记的内容窗口（`ensure_content`），再校验授权与路径边界；不依赖前端隐藏按钮。
- 能力表只增不改：`host_info.capabilities` 返回名称到整数契约版本，如 `{ "folderRead": 1, "systemStats": 1 }`。页面据此特性探测并降级，不猜测宿主版本。
- 错误统一为 `{ code, args?, detail? }`（camelCase）。`code` 是 `locales/*/native.json` 的资源键，页面可自行映射文案；`detail` 只保留系统诊断原文。

## L0：宿主信息与系统采样

- `host_info` → `{ appVersion, platform, locale, capabilities }`。`platform` 为 `windows` / `macos` / `linux`，`locale` 是宿主已解析的有效语言。
- `system_stats` → 最近一次共享采样：`{ sampledAt, cpu: { usagePercent, cores }, memory: { totalBytes, usedBytes }, network: { rxBytesPerSec, txBytesPerSec }, disks: [{ label, totalBytes, usedBytes }], battery: { percent, charging } | null, uptimeSecs }`。
- 采样由宿主单线程约每秒刷新一次，所有页面读同一份缓存；页面数量不增加采样成本。应用启动初期缓存未就绪时以 `errors.statsUnavailable` 拒绝，页面应等待重试而非视为故障。
- 隐私边界：不含进程列表、主机名、用户名与 MAC 地址；磁盘 `label` 为挂载点（如 `C:\`）。台式机或查询失败时 `battery` 为 `null`。
- 页面轮询建议：可见时 1 秒、隐藏时放宽（参考示例用 5 秒），恢复可见立即补一次；只更新变化的文本。

## L1：文件夹只读访问

- `folder_pick` → 打开以内容窗口为父窗口的原生目录选择器；确认返回根目录**显示名**（字符串），取消返回 `null`。完整路径只保存在宿主配置，不返回页面。
- `folder_list { path }` → `{ rootLabel, entries, truncated }`。`path` 是根内相对路径，`/` 分隔，空串为根。`entries` 为 `{ name, kind: "file" | "dir", sizeBytes, modifiedMs, hidden }`，按名称排序，单次上限 2000 条，超出 `truncated: true`。符号链接条目跳过。
- `folder_open { path }` → 以系统默认程序打开根内**文件**；`folder_reveal { path }` → 在资源管理器中打开条目所在目录（目录取自身）。两者都不执行带参数的 shell 命令。
- 路径规则：不做百分号解码，文件名中的 `%` 按字面；拒绝反斜杠、冒号、空字节与上级分量；根与目标规范化后确认边界，符号链接与目录联接指向根外时拒绝。
- 授权生命周期：随页面配置持久化，重启后无需重选；更换页面来源时清除；导出页面包不携带授权，导入后需重新选择；根目录失效报 `errors.folderRootMissing`，页面应引导重选。
- 主要消息码：`folderNoGrant`（未授权）、`folderRootMissing`（根失效）、`folderOutside`（越界或非法字符）、`folderNotFound`（缺失）、`folderReadFailed`、`folderOpenFailed`、`folderPathInvalid`。

## 内容页出站 CSP

资源协议为内容页响应追加独立 CSP（收紧而非放宽）：`default-src 'self'`；`script-src 'self'`；`style-src 'self' 'unsafe-inline'`；`img-src 'self' data:`；`connect-src 'self' ipc: http://ipc.localhost https:`；`object-src 'none'`；`base-uri 'self'`；`frame-src 'none'`。应用生成的数据恢复引导页单独放行内联脚本。

对页面作者的含义：脚本与模块必须同源随包提供，不能引用 CDN 脚本或内联 `<script>`；样式可内联；`fetch` 允许同源与 HTTPS 出站（行情示例即据此联网）；不能嵌入 iframe 或加载 http 明文资源。管理界面 CSP 不受影响。

## 页面接入方式

- 复制参考示例的薄封装：[folder-api.js](../../../../examples/folder/site/folder-api.js)、[monitor-api.js](../../../../examples/system-monitor/site/monitor-api.js)。它们通过 `window.__TAURI_INTERNALS__.invoke` 调用命令，把错误归一为 `{ code, args, detail }`，并在普通浏览器中以 `page.hostUnavailable` 拒绝，便于页面提供明确标注的演示数据降级。
- 不直接依赖 `@tauri-apps/api` 包名导入或 Vite 专用路径；示例是独立静态站点。
- 页面仍遵守示例技能的全部边界：存储键工具专属、用户输入以文本节点渲染、透明背景跟随 `--pinpage-background-opacity`、不实现原生拖窗与窗口按钮。
- 参考实现：[文件夹示例](../../../../examples/folder/site/app.js) 演示授权、导航、打开与定位的完整状态机；[系统监控示例](../../../../examples/system-monitor/site/app.js) 演示共享采样轮询、迷你曲线与演示数据降级。

## 验证要点

- 逻辑层：`cargo test --manifest-path src-tauri/Cargo.toml --lib` 覆盖 pathguard 边界（含 Windows 目录联接越界）、folder 授权门槛与列举、host 能力表与电池标志解析。
- 浏览器预览只验证演示模式与布局；授权选择器模态、越界拒绝的真实提示、CSP 响应头、capability 来源匹配与采样数值必须在真实构建的桌面实例验收，记录见[开发与构建](../../../../docs/development.md#当前状态与验收)。