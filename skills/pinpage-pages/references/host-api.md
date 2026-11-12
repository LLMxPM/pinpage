# 宿主能力契约

内容页是不受信任页面。宿主按分级提供只读能力：页面不获得 Shell、窗口管理或任意文件系统权限，也不为此放宽工作台 CSP 与资源协议边界。本文约定内容页可调用的原生能力；智能体的 CLI 命令见 [CLI 手册](cli.md)。

## 接入与能力探测

只在需要宿主数据时接入，通过 `window.__TAURI_INTERNALS__.invoke(command, args)` 调用下文列出的命令。独立静态页无需引入 Tauri npm 包；普通浏览器没有宿主对象时显示明确的不可用状态，演示数据需标注。

```javascript
// 功能：提供独立页面的宿主环境探测入口。
/// 返回宿主信息；普通浏览器返回 null，原生调用失败交给调用方显示结构化错误。
export async function readHostInfo() {
  const host = window.__TAURI_INTERNALS__;
  if (!host || typeof host.invoke !== 'function') return null;
  return host.invoke('host_info');
}
```

先读取 `host_info.capabilities`，按能力名和契约版本探测后再使用；不根据应用版本猜测能力，不尝试其他应用命令。异步调用失败时处理消息码、保留当前可用内容，不把失败当成零值或空文件夹。

L1 提供文件列表和打开／定位操作，不读取文件正文。CLI 安装源码不会授予内容页访问源码目录的权限；来源更新后需要重新授权。用户主动选择数据文件、联网及缓存实现见 [页面制作指南](page-development.md#数据来源联网与持久状态)。

## 分级模型

| 级别        | 能力                                                         | 授权方式                       | 现状         |
| ----------- | ------------------------------------------------------------ | ------------------------------ | ------------ |
| L0 环境信息 | `host_info`、`system_stats`                                  | 默认可用，只读聚合             | 已提供       |
| L1 逐页授权 | `folder_pick`、`folder_list`、`folder_open`、`folder_reveal` | 原生选择器逐页确认，根目录边界 | 已提供，只读 |
| L2 更高权限 | 写入、删除、事件推送、通知、剪贴板等                         | 需另行设计授权与审计           | 暂不提供     |

- 所有命令先校验调用方是已登记的内容窗口（`ensure_content`），再校验授权与路径边界；不依赖前端隐藏按钮。
- 能力表只增不改：`host_info.capabilities` 返回名称到整数契约版本，如 `{ "folderRead": 1, "systemStats": 1 }`。页面据此特性探测并降级，不猜测宿主版本。
- 错误统一为 `{ code, args?, detail? }`（camelCase）。`code` 是宿主消息码，页面可自行映射为提示；`detail` 只保留系统诊断原文。

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

对页面作者的含义：脚本与模块必须同源随包提供，不能引用 CDN 脚本或内联 `<script>`；样式可内联；`fetch` 允许同源与 HTTPS 出站；不能嵌入 iframe 或加载 http 明文资源。管理界面 CSP 不受影响。

## 联调与降级

浏览器预览只检查不可用／演示状态及布局。原生选择器、取消授权、根目录失效、越界拒绝、真实采样和 CSP 响应头需要在 PinPage 中验证；使用 [CLI 诊断与截图](cli.md#诊断与真实桌面截图)记录结果。页面目录、透明、滚动和尺寸适配统一见 [页面制作指南](page-development.md)。
