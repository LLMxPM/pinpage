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

清单示例、字段类型与数值约束统一维护在公共技能的[页面包文件格式](../skills/pinpage-pages/references/package-format.md#manifestjson)中；默认布局和命名预设的完整参数见[窗口布局字段](../skills/pinpage-pages/references/package-format.md#窗口布局字段)，存储快照格式与迁移限制也在同一参考中说明。仓库和随包技能直接使用这份规范。

`windowPreset` 表示首次导入的默认布局，`windowPresets` 表示独立的有序命名模板列表。新建导入可分别选择是否应用默认布局和导入列表，示例库添加直接使用两者；默认布局不会自动变成命名模板。布局按当前平台能力回退，更换既有页面来源默认保留当前窗口与列表。

导出保存默认布局和命名列表；工作台采集运行页数据，CLI 默认不附带个人存储。文件夹授权不随包迁移，导入后需重新选择。缺少布局或列表字段的旧包保持兼容，旧应用可能无法保留新增字段，迁移前应升级应用。

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

源码清单补全、离线资源要求、预览、注册、Rust 内嵌与验证流程统一见[示例开发与接入](examples-development.md)。运行 `pnpm examples:pack` 在 `dist/examples/` 生成可独立导入的包；修改源码不会覆盖已导入的独立副本，验证新版时重新添加或重新导入。

## CLI 安装与更新

CLI 首次安装默认恢复合法随包数据、导入命名列表并应用布局；`--import-package-presets false` 和 `--apply-package-preset false` 分别控制两个窗口行为。更新忽略包内存储和默认布局，预设默认 `--presets keep`；`.pinpage` 来源可显式选择 `merge`（按 ID 追加、同 ID 不同内容报冲突）或 `replace`（替换整组，含清空）。保留原 ID、当前窗口和同源持久数据，清除文件夹授权；更新重新导航但不额外销毁已运行的 WebView，关闭页仍保持关闭。

智能体可先安装站点目录，通过 `window presets put/save` 创建模板、逐项应用验证，再 `pages export <页面ID> --out ./工具.pinpage`。导出默认不覆盖已有文件、不附带个人存储，显式 `--with-data true` 要求运行页；结果返回 `presetCount`、`withData`、`dataBytes`。CLI 离线目录打包暂未提供。完整参数与示例见[CLI 指南](agent-cli.md)。
