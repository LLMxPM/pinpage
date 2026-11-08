# 版本管理与 GitHub Release 升级

PinPage 从 `0.2.0` 起接入 Windows x64 应用内更新。版本清单和安装包只从 `LLMxPM/pinpage` 的 GitHub Releases 获取；更新包使用 Tauri updater 签名，Windows 安装包代码签名暂未配置。应用内更新只在 Windows x64 正式构建启用，其他平台范围见[平台支持](platforms.md)。

签名构建、离线验签和界面检查已完成；真实发布后的跨版本安装、重启及数据保留仍待验收，详见[开发与构建](development.md#当前状态与验收)。

## 版本规则

- 应用版本采用 SemVer，例如 `0.2.0`、`0.2.1`、`0.3.0`；预发布可用 `0.3.0-beta.1`。
- `package.json` 是版本来源，脚本同步 `src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 与 `Cargo.lock` 中的应用版本，不修改依赖版本。
- Git 标签固定为 `v<版本>`。构建元数据和非规范版本会被脚本拒绝；CI 校验四处版本一致。
- 应用只接收高于当前版本的正式版，不自动降级。预发布 Release 不设置为 Latest。
- 配置的 `schemaVersion` 与应用版本独立。当前配置格式仍为 `1`；读取未来格式时保留原文件、停止写入并提示使用新程序。
- 已发布版本禁止覆盖安装包或签名；修复应发布更高版本。草稿允许重试上传。

```powershell
pnpm version:set 0.2.1
pnpm version:check
pnpm version:check v0.2.1
pnpm test:release
```

版本脚本沿用项目 Prettier 配置写入 JSON，不需要另外手动格式化版本文件。

## 本地签名构建

Rust 最低版本为 `1.90`，当前已验证工具链为 `1.98.1`。前端依赖使用 pnpm。

```powershell
pnpm install --frozen-lockfile
pnpm release:build
cargo run --locked --manifest-path src-tauri/Cargo.toml --example verify-update -- src-tauri/target/release/bundle/nsis/PinPage_0.2.0_x64-setup.exe
```

`verify-update` 离线验证安装包，不启动安装程序。它验证真实安装包的 Minisign 签名、签名绑定版本，并确认内容篡改和错误版本会被拒绝。应用配置启用 `requireSignedVersion`，拒绝未绑定版本或版本不符的更新签名。

产物位于 `src-tauri/target/release/bundle/nsis/`，不提交 Git。独立验证实例继续使用[开发文档](development.md#独立验证实例)中的标识及目录，应用内安装入口禁用。

## GitHub 发布流程

1. 用版本脚本同步版本，完成相关测试及文档更新。
2. 提交并推送代码，再创建并推送对应标签，例如 `git tag v0.2.0` 和 `git push origin v0.2.0`。
3. Release 工作流校验版本与标签、执行发布脚本测试，并构建签名的 x64 NSIS 安装包。
4. 离线验证安装包签名和版本绑定，失败则停止。
5. 准备 GitHub Release 草稿和更新说明，生成 `latest.json` 与 `.sha256`。
6. 向草稿上传以下四个文件，检查名称和大小后才公开发布。

| 附件 | 内容 |
| --- | --- |
| `PinPage_<版本>_x64-setup.exe` | NSIS 安装包，首次安装及 updater 共用 |
| `PinPage_<版本>_x64-setup.exe.sig` | 安装包的 updater 签名 |
| `PinPage_<版本>_x64-setup.exe.sha256` | 手动完整性校验 |
| `latest.json` | 版本、说明、发布时间、平台、安装包 URL 和签名内容 |

所有发布任务串行执行。只有高于当前 Latest 的正式版本才设为 Latest；补发旧版和预发布不会抢占。手动运行工作流时填写已存在且与代码版本一致的标签。

草稿通过可分页的 Release 列表按标签定位，不能使用只返回已发布版本的按标签查询接口。权限或网络错误直接中止，不按“不存在”处理；接口行为见 [GitHub Release API](https://docs.github.com/en/rest/releases/releases#list-releases)。

若标签已经推送但发布脚本需要修复，保持标签不动，在 `main` 修复并验证发布工具，然后手动运行最新 `main` 的 Release 工作流，输入原标签。应用仍检出并构建原标签源码；准备与发布步骤使用该次工作流 SHA 对应的 `publish.mjs`，只替换发布脚本，不更改安装包的应用源码。已公开的版本仍禁止覆盖；只有草稿可重试。

GitHub Release 的更新清单固定入口为：

```text
https://github.com/LLMxPM/pinpage/releases/latest/download/latest.json
```

清单平台为 `windows-x86_64`，安装包 URL 固定到该版本的 `releases/download/v<版本>/` 附件，签名字段使用 `.sig` 文件内容。客户端不使用 GitHub API Token，不提供自定义更新服务器或镜像设置。

## 客户端行为

- 「设置 → 关于」显示最近检查时间、更新状态、检查按钮、更新说明、GitHub Release 入口和当前版本。
- 正式 Windows x64 Release 构建启动后延迟 15 秒尝试检查，后台每小时检查是否达到 24 小时间隔；网络失败也计入间隔。可以随时手动检查。
- 自动检查开关与忽略版本保存到独立的 `updates.json`，操作立即生效，不依赖设置表单的保存按钮。
- 自动检查只提示；下载和安装需要用户主动操作。忽略版本不再触发自动更新提示，手动检查仍可找回。
- 下载和验签由 Rust 持有任务；关闭工作台不取消下载，重建后读取现有进度。每次只运行一个更新任务。
- 已下载包暂存在当前进程内，应用退出后需要重新下载；首版不做断点续传。忽略已下载版本会释放包内容。
- 更新说明按纯文本显示，不执行 Release 内容中的脚本或 HTML。
- 安装前有独立确认，说明关闭页面及未保存网页状态可能丢失；Windows 使用 `passive` 模式展示安装进度并重启。
- 开发构建、非 Windows x64 平台和独立验证标识不启用应用内升级；可在「关于」查看版本并打开 Release。

## 安装与数据保护

安装任务先冻结配置变更，与页面创建、窗口操作及设置保存共用操作边界；窗口几何事件也停止改写最终快照。保存最后配置并生成同目录 `settings.pre-update.json` 备份后，才调用 updater 安装。

保存、备份、验签或安装程序启动失败时不主动关闭页面，解除冻结并保留已下载包供重试。安装程序成功启动后 updater 会退出当前进程；此后的安装结果需要由安装程序及下次启动确认，不能保证旧进程仍在。

应用标识 `com.pinpage.desktop`、配置目录、导入页面副本及 WebView2 数据目录保持一致。重启只在 `restoreSession` 为真时恢复 `enabled` 为真的页面；网页未持久化的 JS 内存和 sessionStorage 不能承诺完整恢复。

首个接入 updater 的版本需要用户手动安装一次。发生问题时从本仓库 Release 手动下载安装包；自动更新不执行降级，也不自动回滚配置。

## 验收范围

- 自动测试覆盖版本同步和标签、更新来源白名单、预发布拒绝、每日节流、升级状态占用、偏好保存、最终配置备份、冻结失败解除和未来配置保护。
- 本地签名包通过离线验签及篡改拒绝检查；这不代表真实 GitHub 下载和 Windows 安装已验收。
- 浏览器使用官方 IPC mock 验证中英文界面；真实双版本安装需在独立测试环境完成。
- 发布后需要验证 GitHub Latest 附件可访问、旧版到新版的下载和安装、安装取消或权限拒绝、重启后的页面开关与数据保留，以及配置目录不可写时的中止行为。

## 参考

- [Tauri updater](https://v2.tauri.app/plugin/updater/)：静态清单、构建签名及 Windows 安装方式。
- [Tauri Update API](https://docs.rs/tauri-plugin-updater/2.14.1/tauri_plugin_updater/struct.Update.html)：下载验签和安装退出行为。
- [GitHub Release 管理](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)：草稿、附件和正式发布。
