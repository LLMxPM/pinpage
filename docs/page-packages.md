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

`site/` 保持资源的相对目录结构，入口必须是其中的 HTML。页面不获得文件系统、Shell 或窗口管理权限。解包限制和存储数据范围见[架构与数据](architecture.md#页面包与数据边界)。

## 清单与默认窗口预设

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
  }
}
```

- `schemaVersion` 当前为 `1`，与应用配置版本分别演进。
- `name` 是页面包的默认名称，`entry` 相对于 `site/`；`files` 统计站点文件数，`exportedAt` 使用 Unix 秒。
- `windowPreset` 可选，复用窗口预设的外观与几何字段。宽高为逻辑像素，范围为 `80～8192 × 60～8192`；`fx`、`fy` 为 `0～1` 的工作区位置比例，`0.5` 表示居中。`fullscreen: true` 时按目标显示器整屏计算尺寸。
- 新建导入页时默认勾选「使用随包窗口预设」，也可以取消后手动设置窗口选项；示例库添加直接使用随包布局。原生层从实际包中重新读取布局，不采用前端传入的任意窗口字段。
- 布局在当前平台不支持的能力会安全回退；没有原生拖动手势的平台保留系统边框。位置按当前显示器计算，不携带作者机器的绝对屏幕坐标。
- 随包布局只用于新建页面，不写入全局预设库。需要复用时，可以在创建后将当前窗口另存为预设。更换既有页面的来源会保留当前窗口设置。
- 导出页面包时携带当前窗口布局和可采集的页面数据，不携带页面 ID、开关状态、全局预设库或应用设置。

旧版页面包没有 `windowPreset`，导入时沿用创建表单的窗口设置。新增字段保持可选，旧版应用可读取页面内容，但会忽略随包布局。

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

运行 `pnpm examples:pack`，在 `dist/examples/` 得到四个可通过「导入页面包」打开的 `.pinpage` 文件。构建产物不提交版本库。应用的 Rust 构建也使用同一个打包器，在构建输出目录生成并内嵌这些标准包，不依赖用户机器上还存在仓库源码。

预览页面可复用已有开发服务并访问 `/examples/timer/site/index.html`；按目录导入预览时选择工具的 `site/`，按页面包导入才能携带清单中的窗口预设。不要在已有服务运行时重复启动应用。

页面设计、透明背景、状态保存和示例库注册指引见项目技能 [pinpage-examples](../.agents/skills/pinpage-examples/SKILL.md)。修改源码不会覆盖已导入的独立副本，验证新版时重新添加或重新导入。
