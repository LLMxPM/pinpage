# 页面包与示例制作

`.pinpage` 是 PinPage 的 ZIP 页面包格式。内置示例与手动导入的页面包共用清单解析、路径校验、解包和页面数据恢复流程。

## 包结构

```text
专注计时器.pinpage
  manifest.json
  site/
    index.html
    style.css
    timer.js
    presets.json
  data/
    storage.json       可选，由运行中页面导出时采集
```

`site/` 保持资源的相对目录结构，入口必须是其中的 HTML。页面不获得 Shell 或窗口管理权限；文件夹只读需经原生选择器逐页授权，环境信息为只读聚合，见[架构与数据](architecture.md#权限与安全边界)。解包限制和存储数据范围见[架构与数据](architecture.md#页面包与数据边界)。

## 清单、默认布局与命名预设

```json
{
  "schemaVersion": 1,
  "name": "专注计时器",
  "entry": "index.html",
  "exportedAt": 0,
  "files": 4,
  "windowPreset": {
    "mode": "floating",
    "alwaysOnTop": true,
    "decorations": false,
    "rounded": false,
    "transparentBackground": false,
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

- `schemaVersion` 当前为 `1`，与应用配置版本分别演进。
- `name` 是页面包的默认名称，`entry` 相对于 `site/`；`files` 统计站点文件数，`exportedAt` 使用 Unix 秒。
- `windowPreset` 可选，复用窗口预设的外观与几何字段。宽高为逻辑像素，范围为 `80～8192 × 60～8192`；`fx`、`fy` 为 `0～1` 的工作区位置比例，`0.5` 表示居中。`fullscreen: true` 时按目标显示器整屏计算尺寸。
- `windowPresets` 是可选的有序命名预设列表，每项包含 `id`、`name` 和完整布局字段。ID 为 1～80 位 ASCII 字母、数字、短横线或下划线，仅需在本页唯一；名称 1～40 字、每页最多 20 项。同名不同 ID 合法；重复 ID、缺失字段、非法数值或超额列表整包拒绝，不逐项跳过。
- 新建导入默认勾选「导入随包窗口预设」与「应用随包默认布局」，可分别取消。取消应用布局仍可保留列表；示例库添加直接使用随包列表和默认布局。原生层从实际包中重新读取并校验，不采用前端提交的任意窗口字段。
- 布局在当前平台不支持的能力会安全回退；没有原生拖动手势的平台保留系统边框。位置按当前显示器计算，不携带作者机器的绝对屏幕坐标。
- 默认布局只初始化新页面，不自动创建命名模板；列表独立导入到该页，原始跨平台定义保留，应用时按平台能力回退。更换既有页面的来源默认保留当前窗口和预设。
- 导出携带当前默认布局、预设列表的 ID／名称／顺序／参数，以及按入口选择采集的页面数据；不携带页面 ID、开关状态或应用设置。工作台采集运行页数据，CLI 默认只导出站点、默认布局和命名预设。
- 文件夹授权属于页面配置，不随包导出；导入使用文件夹能力的页面后需要重新选择授权目录。

旧版包缺少 `windowPreset` 时沿用创建表单的窗口设置，缺少 `windowPresets` 时列表为空。格式仍为 `schemaVersion: 1`，新字段保持可选；旧应用会忽略不认识的字段，因此可能不能完整保留命名列表，再次导出前应升级应用。

## 仓库中的示例源码

每个工具使用同一种可编辑目录布局，例如：

```text
examples/timer/
  manifest.json
  site/
    index.html
    style.css
    timer.js
    presets.json
```

源码清单可省略 `files` 和 `exportedAt`，打包器会统计站点文件并固定示例时间戳。同一份源码生成可复现的包；内置示例不附带个人存储快照。`site/` 下的子目录资源自动收集，不再维护 Rust 逐文件嵌入清单。符号链接和目录联接不允许作为示例资源。

运行 `pnpm examples:pack`，在 `dist/examples/` 得到可通过「导入页面包」打开的 `.pinpage` 文件，包括六个内置工具，以及独立的[自选行情联网验证示例](../examples/stocks/README.md)。行情示例尚未注册到工作台示例库。构建产物不提交版本库。应用的 Rust 构建也使用同一个打包器，在构建输出目录生成并内嵌这些标准包，不依赖用户机器上还存在仓库源码。

预览页面可复用已有开发服务并访问 `/examples/timer/site/index.html`；按目录导入预览时选择工具的 `site/`，按页面包导入才能携带清单中的窗口预设。不要在已有服务运行时重复启动应用。

页面设计、透明背景、状态保存和示例库注册指引见项目技能 [pinpage-examples](../.agents/skills/pinpage-examples/SKILL.md)。修改源码不会覆盖已导入的独立副本，验证新版时重新添加或重新导入。

## CLI 安装与更新

CLI 首次安装默认恢复合法随包数据、导入命名列表并应用布局；`--import-package-presets false` 和 `--apply-package-preset false` 分别控制两个窗口行为。更新忽略包内存储和默认布局，预设默认 `--presets keep`；`.pinpage` 来源可显式选择 `merge`（按 ID 追加、同 ID 不同内容报冲突）或 `replace`（替换整组，含清空）。保留原 ID、当前窗口和同源持久数据，清除文件夹授权；更新重新导航但不额外销毁已运行的 WebView，关闭页仍保持关闭。

智能体可先安装站点目录，通过 `window presets put/save` 创建模板、逐项应用验证，再 `pages export <页面ID> --out ./工具.pinpage`。导出默认不覆盖已有文件、不附带个人存储，显式 `--with-data true` 要求运行页；结果返回 `presetCount`、`withData`、`dataBytes`。CLI 离线目录打包暂未提供。完整参数与示例见[CLI 指南](agent-cli.md)。
