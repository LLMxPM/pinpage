# 页面包文件格式

制作或检查 `.pinpage` 时阅读本文。运行操作与参数见 [CLI 手册](cli.md#页面包导出)。普通页面开发不需要手写包：先安装站点、创建命名预设，再由 CLI 或工作台导出。

## 目录与生成职责

`.pinpage` 是 ZIP，根级为 `manifest.json`，页面资源在 `site/`，可选存储快照为 `data/storage.json`。入口、CSS、模块、图片与 JSON 保持站点内相对路径。

智能体可编写或核对清单与窗口预设；文件数、导出时间和实际运行数据宜由应用生成。`storage.json` 是宿主的存储迁移格式，不是页面自己的业务数据文件；通常通过 `pages export --with-data true` 采集，无需智能体模拟浏览器存储。制作可公开分发的工具默认省略它，避免附带个人数据。

## manifest.json

下面是完整页面包清单示例，三个资源为 `site/index.html`、`site/style.css`、`site/app.js`：

```json
{
  "schemaVersion": 1,
  "name": "我的工具",
  "entry": "index.html",
  "exportedAt": 0,
  "files": 3,
  "windowPreset": {
    "mode": "floating",
    "alwaysOnTop": true,
    "decorations": false,
    "rounded": true,
    "transparentBackground": true,
    "positionLocked": false,
    "sizeLocked": false,
    "width": 420,
    "height": 360,
    "fx": 0.5,
    "fy": 0.5,
    "fullscreen": false
  },
  "windowPresets": [
    {
      "id": "compact",
      "name": "精简",
      "mode": "floating",
      "alwaysOnTop": true,
      "decorations": false,
      "rounded": true,
      "transparentBackground": true,
      "positionLocked": false,
      "sizeLocked": false,
      "width": 320,
      "height": 220,
      "fx": 1,
      "fy": 0,
      "fullscreen": false
    }
  ]
}
```

| 字段 | 类型与含义 |
| --- | --- |
| `schemaVersion` | 必填整数，当前只能为 `1`，与应用配置版本分别演进。 |
| `name` | 必填字符串，页面默认名称，建议非空且不超过 80 字。 |
| `entry` | 必填字符串，相对于 `site/` 的 HTML／HTM 路径，可含子目录。不能带 `site/` 前缀、绝对路径、上级目录、反斜杠、冒号或 NUL。 |
| `exportedAt` | 必填非负整数，Unix 秒；可复现分发包可以为 `0`。 |
| `files` | 必填非负整数，`site/` 中实际文件数，不计清单或存储快照；宿主仍实际检查资源数量和体积。 |
| `windowPreset` | 可选布局对象，表示首次导入的默认窗口状态；缺省时沿用创建设置。 |
| `windowPresets` | 可选命名预设数组，缺省为 `[]`，顺序即菜单和管理器顺序；每项为 `id`、`name` 与平铺的布局字段，没有嵌套 `fields`。 |

清单采用 UTF-8 JSON，解压后最大 256 KiB。整个站点上限为 4000 个文件、256 MiB；资源路径不能越出 `site/`。包中不携带作者的页面 ID、绝对屏幕坐标、文件夹授权或应用设置。离线目录打包尚无公开 CLI 命令；直接安装 `site/` 也不会自动读取旁边的清单。

### 窗口布局字段

`windowPreset` 与 `windowPresets` 每项使用同一套布局字段；命名项额外具有 `id` 和 `name`。`put --file` 的 JSON 使用 `name` 与这组字段，ID 通过 CLI `--id` 提交，不写进模板文件。

| 字段 | 类型与约束 |
| --- | --- |
| `mode` | 必填，`floating` 或 `desktop`。 |
| `alwaysOnTop` | 必填布尔值，浮动模式置顶；桌面模式归一化为 `false`。 |
| `decorations` | 必填布尔值，系统边框偏好，桌面模式不显示系统边框。 |
| `rounded` | 必填布尔值，无边框圆角；浮动模式有系统边框时归一化为 `false`。 |
| `transparentBackground` | 布尔值，原生背景透明；旧包缺省为 `false`，页面 CSS 也需适配。 |
| `positionLocked` / `sizeLocked` | 必填布尔值，位置锁与大小锁分别控制。 |
| `width` / `height` | 必填有限数值，逻辑像素，范围分别为 `80～8192`、`60～8192`。 |
| `fx` / `fy` | 必填有限数值，`0～1` 的工作区相对位置；`0,0` 左上、`1,1` 右下、`0.5,0.5` 居中。 |
| `fullscreen` | 布尔值，缺省为 `false`；为真时按目标显示器整屏动态计算尺寸，仍需提供合法宽高。 |

命名预设 ID 为 1～80 位 ASCII 字母、数字、短横线或下划线，只需页内唯一；名称去除首尾空白后为 1～40 字，同名不同 ID 合法，每页最多 20 项。重复 ID、非法名称／数值或缺失必填字段会在解包前拒绝整包。

默认布局与命名列表互相独立：导入列表不会强制应用其中一项；只应用默认布局也不自动创建模板。旧应用忽略不认识的列表字段，无法保证再次导出后保留列表。新建、更新和显式同步规则见 [CLI 页面操作](cli.md#页面操作)。

## data/storage.json

文件为 UTF-8 JSON 对象，直接包含 `localStorage`、`sessionStorage` 和 `indexedDB`，没有 `schemaVersion`、`entry` 或 `payload` 外层。`pending-data.json` 的内部包装不属于包格式。下面的示例同时展示字符串存储、对象仓库、索引和带类型的记录：

```json
{
  "localStorage": {
    "my-tool-preferences": "{\"theme\":\"dark\"}",
    "my-tool-count": "3"
  },
  "sessionStorage": {},
  "indexedDB": [
    {
      "name": "my-tool-v1",
      "version": 1,
      "stores": [
        {
          "name": "items",
          "keyPath": "id",
          "autoIncrement": false,
          "indexes": [
            {
              "name": "byCreatedAt",
              "keyPath": "createdAt",
              "unique": false,
              "multiEntry": false
            }
          ],
          "records": [
            {
              "key": "note-1",
              "value": {
                "$t": "obj",
                "v": {
                  "id": "note-1",
                  "text": "示例便签",
                  "createdAt": { "$t": "date", "v": 0 }
                }
              }
            }
          ],
          "truncated": false
        }
      ]
    }
  ]
}
```

| 字段 | 格式与恢复规则 |
| --- | --- |
| `localStorage` / `sessionStorage` | 键到**字符串**的对象。即使业务内容是 JSON 或数字，值也先字符串化；恢复只写入字符串值。缺省视为 `{}`。 |
| `indexedDB` | 数据库数组，缺省视为 `[]`。每个数据库含 `name`、正整数 `version` 和 `stores`。 |
| `stores[]` | 对象仓库，含 `name`、`keyPath`（`null`、字符串或字符串数组）、`autoIncrement`、`indexes`、`records` 与 `truncated`。 |
| `indexes[]` | 索引定义，含 `name`、`keyPath`、`unique` 与 `multiEntry`，遵守 IndexedDB 自身的约束。 |
| `records[]` | `{ "key": 编码后的主键, "value": 编码后的值 }`。`keyPath: null` 时恢复显式主键；内联主键由值中对应字段提供。 |
| `truncated` | 采集截断标记；当前每个对象仓库最多采集 3000 条，超出时为 `true`，不能报告数据完整迁移。 |

### IndexedDB 值编码

字符串、布尔值、`null` 和有限数字直接保存；数组逐项编码。普通对象必须包装为 `{ "$t": "obj", "v": { ...逐字段编码... } }`，避免业务对象本身的 `$t` 字段与类型标签冲突。主键也使用同一编码。

| 原始值 | JSON 编码 |
| --- | --- |
| 普通对象 | `{ "$t": "obj", "v": { "text": "内容" } }` |
| `Date` | `{ "$t": "date", "v": 0 }`，`v` 为 Unix 毫秒。 |
| `NaN` / 正负无穷 | `{ "$t": "number", "v": "NaN" }`，或 `"Infinity"`、`"-Infinity"`。 |
| `BigInt` | `{ "$t": "bigint", "v": "123" }`，十进制字符串。 |
| `undefined` | `{ "$t": "undefined" }`。 |
| `RegExp` | `{ "$t": "regexp", "v": "pattern", "f": "gi" }`。 |
| `ArrayBuffer` | `{ "$t": "buffer", "v": "AQI=" }`，Base64 字节。 |
| TypedArray / DataView | `{ "$t": "typed", "c": "Uint8Array", "v": "AQI=" }`，`c` 为构造器名，内容为有效视区字节。 |
| `Blob` | `{ "$t": "blob", "m": "text/plain", "v": "aGk=" }`，MIME 与 Base64 字节。 |
| 不支持的值 | `{ "$t": "unsupported", "v": "object" }`，恢复为 `undefined`；嵌套深度超过 12 标为 `nested`。 |

Map、Set、函数、Symbol 等不能完整迁移。不应手写类型标签来声称原浏览器中存在某些数据；需要演示数据时由页面正常初始化，或清楚说明是人工制作的快照。

### 限制与验收

快照解压后最大 16 MiB。原生导入先校验体积和 JSON 合法性，实际回填再按上述字段尝试恢复；合法 JSON 不等于所有数据库、索引和记录都已成功恢复。数据库采集失败可能出现 `error` 和空 `stores`，存储读取失败可能出现 `__error`，均不能当作成功采集。

Cookie、HTTP 缓存、Service Worker、JS 变量、DOM、未保存输入与文件夹授权不在快照中。数据只在新页面首次打开时通过同源引导页写入一次；更新已有页忽略包内快照，保留原页存储。sessionStorage 不保证跨 WebView 生命周期延续。

交付时检查导出结果的 `withData`、`dataBytes`、快照中的截断／错误标记，并按任务验证关键数据恢复。仅导出成功不能证明完整迁移；预设则核对 ID、名称、顺序与参数，并按 [CLI 预设流程](cli.md#命名窗口预设)实际应用、截图检查。
