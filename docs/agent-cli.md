# 智能体 CLI 与公开 skill

当前源码已实现 Windows 本机控制、页面安装更新、桌面截图、按页诊断与离线 skill 管理；尚未发布。公开的 v0.2.0 安装包不包含此次新增能力。实测范围与发布前待办见[当前状态与验收](development.md#当前状态与验收)，原始设计见[实施规划](agent-cli-plan.md)。

## 获取与连接

CLI 随应用安装为同目录的 `pinpage-cli.exe`，不提供独立下载。安装器可选择加入当前用户 PATH；旧终端仍可用安装目录的绝对路径。源码构建使用 `pnpm cli:build`，产物为 `src-tauri/target/release/pinpage-cli.exe`。不需要 Node 或 Python 来运行已安装 CLI。

```powershell
# 功能：检查连接，或在用户明确需要时启动应用到托盘。
pinpage-cli --json doctor
pinpage-cli --json app start
```

默认选择 `com.pinpage.desktop`。默认 Debug 开发应用使用 `com.pinpage.development`，需显式选择。独立验证需指定 `--instance com.pinpage.verification`；启动独立构建还需 `--app-path <应用绝对路径>`。普通页面命令不会启动应用，也不退出应用。本机控制默认开启，可从工作台顶栏的独立「智能体与 CLI」入口关闭；关闭释放管道和诊断监听，不删除页面，CLI 无重新开启权限的命令。

全局参数为 `--json`、`--instance`、`--expected-instance`、`--timeout-ms`（默认 30000）、`--locale zh-CN|en-US`、`--help` 和 `--version`。JSON 模式只在 stdout 输出一个 UTF-8 对象；检查 `ok` 与退出码，不依赖中文错误文案。

## 安装和更新

```powershell
# 功能：安装完整目录，并按同一实例和最新修订更新已有页面。
$created = pinpage-cli --json pages install --source ./site --entry index.html --name '专注计时' --client-key focus-01 | ConvertFrom-Json
if (-not $created.ok) { throw '检查错误和操作状态' }
$pageId = $created.data.pageId
$current = pinpage-cli --json pages inspect $pageId | ConvertFrom-Json
if (-not $current.ok) { throw '无法读取页面' }
pinpage-cli --json pages update $pageId --source ./site --entry index.html --expected-revision $current.data.revision --expected-instance $current.instanceId
```

`pages install` 接受目录、单 HTML、`.pinpage`，或互斥的 `--url https://...`。多个根级 HTML 需指定入口。默认开启，可用 `--closed` 仅保存；首次包安装默认应用随包布局，`--apply-package-preset false` 可取消。同一 client key 和相同内容返回现有页面；不同内容返回冲突，删除页面时才移除对应映射。

`pages update` 只用于导入页，保留 ID、名称、窗口、预设、enabled 和同源持久存储；忽略包内布局与存储快照，清除文件夹授权。更新重新导航，JS 内存会丢失；sessionStorage 只在原 WebView 生命周期内延续，不保证关闭重开。页面应持久保存高价值编辑和计时截止时间。

`pages list/inspect/open/close/reload/focus` 分别查询、开启、关闭、刷新副本和显示找回；关闭保留记录，CLI 不提供删除。`window set <id>` 只合并出现的字段，支持逻辑尺寸、物理位置、模式、边框、透明、置顶和独立锁定；布尔值必须显式传 true/false。

变更可用 `--no-wait` 先取得 operationId，再 `operations inspect <id>`。超时或 Ctrl+C 不回滚已提交修改。失败响应中的 `committed: true` 表示内容已经保存，应修复同一页面。操作保留 100 项、15 分钟；`pages wait <id> --load-id <id>` 绑定本次导航，不会把新导航当作旧导航成功。

## 诊断与截图

先 `debug start <页面 ID> --reload`，再按返回的 loadId 等待，并通过 `debug logs <会话 ID> --cursor 0 --limit 100` 分页读取。覆盖从实际接入开始，显式刷新可覆盖该次加载；加载成功、脚本无异常与视觉通过是不同结论。结束时 `debug stop <会话 ID>`，关闭页面或 15 分钟到期也会清理。

日志不收集请求正文、Cookie、Authorization 或全部存储；URL 去除凭据、查询和片段。console 文本仍可能包含用户数据，不自动上传。每页最多 1000 条／2 MiB，单条最多 16 KiB，分页提示丢弃与游标缺口。

`screenshot <页面 ID> --load-id <加载 ID> --out ./preview.png` 采集真实桌面，包含原生外框和遮挡；默认留白 32 物理像素、稳定等待 300ms，可用 `--padding`、`--settle-ms`、`--metadata-out` 调整。默认拒绝隐藏／最小化页面；显式 `--bring-to-front` 会显示页面并保持原模式。先 `monitors` 再 `screenshot --monitor <ID> --out ...` 可在应用未启动时截屏。

采集使用 Desktop Duplication，不用 WebView 离屏图替代；图片与元数据关联实例、页面、修订、加载代次、物理范围及显示器。默认不覆盖，需显式 `--overwrite`。截图必须实际打开检查；截图成功不等于鼠标交互、透明命中、混合 DPI 或长期稳定性通过。

## 独立安装、更新与导出 skill

工作台顶栏的「智能体与 CLI」面板集中展示内置版本、用户目录版本、技能目录和 CLI 路径，并分组提供用户目录安装更新、其他目录安装更新与版本查询、目录／ZIP 导出。

当前源码内置技能版本为 `1.0.4`，按任务分层组织：

| 文件                                                                   | 职责                                                                            |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| [SKILL.md](../skills/pinpage-pages/SKILL.md)                           | 产品与运行机制、按任务阅读导航、工作流程和执行边界。                            |
| [页面制作指南](../skills/pinpage-pages/references/page-development.md) | 统一说明目录与产物、HTML／CSS、布局滚动、透明背景、外部数据、持久状态和页面包。 |
| [宿主能力契约](../skills/pinpage-pages/references/host-api.md)         | 页面 JavaScript 的能力探测、L0／L1 接口、权限、CSP 与降级；从统一契约生成。     |
| [CLI 手册](../skills/pinpage-pages/references/cli.md)                  | 智能体终端的连接与身份、页面操作、窗口配置、诊断截图、失败恢复和技能管理。      |

技能自包含，具体说明按主题保留一处，通过链接关联，不依赖用户克隆仓库。已安装旧技能需要在独立面板明确更新，再刷新智能体技能列表或开启新会话；更新会替换旧版参考文件。

```powershell
# 功能：离线管理技能，--dir 始终表示 skills 根目录。
pinpage-cli skill install
pinpage-cli skill update --dir ./.agents/skills
pinpage-cli skill export --out ./pinpage-pages.zip
pinpage-cli skill install --dir ./other-skills --source ./pinpage-pages.zip
```

默认目标为当前用户的 `.agents/skills/pinpage-pages`，自定义目标固定为所选根目录下的 `pinpage-pages`。不受应用连接和控制开关影响。来源为内置资源或本地目录／ZIP，不下载或执行安装脚本。

更新检查资源清单、独立 skill 版本、最低 CLI 能力和文件摘要；本地修改冲突需核对后显式 `--replace-modified`，先完整备份到用户本地 `PinPage/skill-backups`。不静默降级，不覆盖其他技能。导出不安装，不含 CLI、凭据、配置或日志。应用升级不自动覆盖已安装 skill，卸载保留用户技能、源码与截图；智能体客户端需刷新技能列表或开启新会话。

退出码：0 完成，2 输入，3 连接／协议，4 鉴权／禁用，5 冲突，6 超时，7 不支持，8 执行失败，130 中断。完整参数见 `--help` 与随包[CLI 手册](../skills/pinpage-pages/references/cli.md)。
