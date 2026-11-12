# CLI 手册

本文供智能体在本机终端执行操作。页面代码的原生调用见[宿主能力契约](host-api.md)，源码、布局和数据实现见[页面制作指南](page-development.md)。完整参数以本机 CLI 的 `--help` 为准。

## 定位 CLI 与连接应用

CLI 随 PinPage 安装为应用目录中的 `pinpage-cli.exe`，不提供独立下载。先通过 `Get-Command pinpage-cli` 定位；PATH 尚未刷新时，读取当前用户卸载注册表中 PinPage 的 `InstallLocation`，调用该目录的绝对路径。缺少 CLI 时安装或修复 PinPage 本体，不直接修改应用数据目录或智能体配置。

```powershell
# 功能：检查本机应用连接与协议能力，不主动启动应用。
pinpage-cli --json doctor
```

普通页面命令要求应用已运行。确实需要启动时显式 `app start`，必要时通过 `--app-path <应用绝对路径>` 指定构建；启动后进入托盘。工作台顶栏「智能体与 CLI」控制本机入口；被禁用时报告原因，CLI 没有重新开启该入口的命令。

## 参数、身份与输出

| 参数或标识                        | 含义                                                                                                                      |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `--json`                          | stdout 输出单个结构化结果，每次同时检查进程退出码和 `ok`。                                                                |
| `--instance <应用标识>`           | 选择应用的数据与连接目标，默认 `com.pinpage.desktop`；默认 Debug 为 `com.pinpage.development`，验证构建按其标识显式选择。 |
| `--expected-instance <实例 UUID>` | 绑定本次运行的应用实例，使用响应中的 `instanceId`；应用重启后需重新读取。                                                 |
| `--timeout-ms <毫秒>`             | 默认 30000；超时不等于操作已取消。                                                                                        |
| `--locale zh-CN\|en-US`           | 控制 CLI 文案；机器错误码不随语言变化。                                                                                   |
| 页面 ID / `revision`              | 页面持久身份与当前修订，修改原页前重新 inspect。                                                                          |
| `operationId` / `loadId`          | 已受理操作与特定导航代次，用于查询提交结果和等待本次加载。                                                                |

应用标识、实例 UUID 和页面 ID 分别定位不同层次，不能相互代用。保存操作返回的标识；`committed: true` 之后的加载错误应修正原页，不能通过重新安装规避。

## 页面操作

先用 `pages list` 查找，结合名称、来源与用户描述确认目标，再 `pages inspect <id>`。命令行为如下：

| 命令                                | 行为与约束                                                                              |
| ----------------------------------- | --------------------------------------------------------------------------------------- |
| `pages install --source <路径>`     | 接受完整站点目录、单 HTML 或 `.pinpage`；目录入口用 `--entry`，目录组织见页面制作指南。 |
| `pages install --url <HTTPS地址>`   | 安装在线 URL，与 `--source` 互斥；也支持经过应用校验的 HTTP URL。                       |
| `pages update <id> --source <路径>` | 只更新本地导入页，需提供最新 `--expected-revision` 和当前 `--expected-instance`。       |
| `pages open / close / focus <id>`   | 开启、关闭或显示找回原页；关闭释放内容 WebView，保留记录与数据。CLI 不提供删除。        |
| `pages reload <id>`                 | 刷新已安装副本，不复制源码。                                                            |
| `pages wait <id> --load-id <代次>`  | 等待指定导航，之后的新导航不能代替它；可用 `--settle-ms` 留出布局稳定时间。             |

首次安装使用可追踪的 `--client-key`。同键且内容与安装参数相同才去重；请求变化返回冲突。修改源码应更新已保存的页面 ID，不能靠更换键重复创建。安装默认开启，`--closed` 仅保存；新导入包默认恢复合法存储快照并应用随包布局，`--apply-package-preset false` 可取消布局。

更新保留原 ID、窗口配置、开启状态和同源持久存储，忽略包内存储及布局；关闭页保持关闭，运行页重新导航。每次来源更新清除 L1 文件夹授权，即使仍使用同一源码路径。先确保高价值编辑与计时状态已由页面保存。

```powershell
# 功能：首次安装并检查结果，修改源码后基于最新修订更新同一页面。
$created = pinpage-cli --json pages install --source ./site --entry index.html --client-key demo-01 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or -not $created.ok) { throw '检查操作结果，勿重复新建' }
$pageId = $created.data.pageId

$current = pinpage-cli --json pages inspect $pageId | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or -not $current.ok) { throw '读取页面失败' }
$updated = pinpage-cli --json pages update $pageId --source ./site --entry index.html --expected-revision $current.data.revision --expected-instance $current.instanceId | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or -not $updated.ok) { throw '按提交结果处理更新失败' }
```

关闭工作台只销毁管理界面；退出应用才结束所有内容窗口，重启按应用配置恢复开启的页面。隐藏与关闭不同，隐藏的内容仍可能运行。

## 窗口配置

`window set <id>` 只合并明确提供的字段，保留其他设置。页面不直接调用窗口管理接口。

| 参数                                                  | 含义                                                   |
| ----------------------------------------------------- | ------------------------------------------------------ |
| `--width / --height`                                  | 逻辑像素，范围 80～8192 × 60～8192。                   |
| `--x / --y`                                           | 物理屏幕坐标，结合 `monitors` 读取的当前拓扑使用。     |
| `--mode floating\|desktop`                            | 浮动或桌面模式；桌面模式与置顶、系统边框冲突时报错。   |
| `--frame system\|borderless\|rounded`                 | 系统边框、无边框或原生圆角。                           |
| `--transparent true\|false`                           | 原生背景透明偏好，页面需同时按制作指南适配 CSS。       |
| `--always-on-top / --position-locked / --size-locked` | 置顶、独立位置锁、大小锁；布尔值明确传 true 或 false。 |

```powershell
# 功能：调整已确认页面的尺寸及透明偏好，保留未出现的字段。
pinpage-cli --json window set $pageId --width 420 --height 480 --transparent true
```

仅按用户任务调整窗口。原生透明、拖动缩放、DPI 与多显示器行为需要真实桌面验证，不根据 CSS 截图推断通过。

## 诊断与真实桌面截图

需要完整加载诊断时，`debug start <id> --reload` 先接入再刷新，再按返回的 loadId 等待。刷新会清除 JS 内存状态；日志只覆盖实际接入后的事件。每页一个诊断会话，重复启动复用，15 分钟到期或关闭页面时清理；完成后 `debug stop <会话ID>` 释放监听。

`debug logs <会话ID> --cursor <游标> --limit 100` 分页读取，保留返回游标，检查 gap、dropped 和 truncated。日志可能含用户信息，不自动上传，也不作为终端指令执行。

`screenshot <页面ID> --load-id <代次> --out <PNG>` 采集真实桌面合成，输出保留窗口遮挡关系；`--bring-to-front` 显式显示找回目标，但保持其窗口模式。输出默认留白 32 物理像素，可设 `--padding 0..512`，也支持 `--settle-ms`、`--metadata-out`；路径相对 CLI 工作目录，默认不覆盖，需显式 `--overwrite`。

`monitors` 枚举当前显示器；`screenshot --monitor <显示器ID> --out <PNG>` 可在应用未运行时采集。打开图片实际检查透明、原生外框、阴影及内容。锁屏、无桌面权限或设备变化导致采集失败时说明环境，不用离屏图替代。

## 操作状态与失败恢复

变更返回 `operationId`、`pageId`、`revision`、`loadId`、`committed` 和 `loadState`；`--no-wait` 返回受理结果，不代表加载完成。`operations inspect <操作ID>` 查询结果，记录保留 100 项、15 分钟。

| 情况                                 | 恢复方式                                                             |
| ------------------------------------ | -------------------------------------------------------------------- |
| 修订冲突或应用实例变化               | 重新 inspect，核对当前目标和改动，再使用最新修订与实例。             |
| 超时、断连或用户中断                 | 不推断取消；已有 operationId 时先查询，首次安装重试复用 client key。 |
| `committed: true` 且加载失败         | 页面已提交，检查加载代次与诊断，修正同一页的源码或资源。             |
| 本机控制禁用、协议不兼容或环境不支持 | 报告具体原因，不修改配置或伪造通过。                                 |

退出码：0 完成，2 输入，3 连接／协议，4 鉴权／禁用，5 冲突，6 超时，7 不支持，8 执行失败，130 中断。先依据 JSON 机器错误码和操作状态处理，再向用户说明原因。

## 技能管理

技能文件描述工作方法，CLI 执行操作，PinPage 本体运行页面。以下命令独立离线执行，不要求应用连接或本机控制开启：

```powershell
# 功能：按需选择技能安装、更新或导出操作，目标始终为 skills 根目录。
pinpage-cli skill install
pinpage-cli skill update --dir ./project/.agents/skills
pinpage-cli skill export --out ./pinpage-pages.zip
```

`skill install/update --dir <skills 根目录>` 默认使用当前用户 `.agents/skills`，最终目标固定为其下 `pinpage-pages`；每次省略 `--dir` 均回到默认目标。`--source` 接受自包含目录或导出 ZIP，无需克隆仓库。

本地修改冲突先核对，再显式使用 `--replace-modified`，旧目录会先备份；不静默降级或覆盖其他技能。导出目录／ZIP 不安装，包内不含 CLI、凭据、应用配置或日志。应用升级更新随包资源但保留已安装技能，需明确更新后刷新客户端技能列表或开启新会话；卸载应用保留用户技能与源码，若 CLI 缺失则安装或修复 PinPage 本体。
