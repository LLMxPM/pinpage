# 示例页面协作规范

本目录遵循根目录的 [AGENTS.md](../AGENTS.md)。创建、修改示例或接入内置示例库前，读取[示例开发与接入](../docs/examples-development.md)，按任务复用唯一页面技能的公共参考：

| 当前任务                                   | 公共规范                                                               |
| ------------------------------------------ | ---------------------------------------------------------------------- |
| 页面结构、布局、滚动、透明、联网与持久状态 | [页面制作指南](../skills/pinpage-pages/references/page-development.md) |
| 宿主环境、系统采样与文件夹能力             | [宿主能力契约](../skills/pinpage-pages/references/host-api.md)         |
| 清单、默认布局与命名预设                   | [页面包文件格式](../skills/pinpage-pages/references/package-format.md) |

工程使用 `examples/<工具名>/manifest.json + site/`，注册、离线分发、源码清单补全和验证按仓库接入文档执行；公共参考中的普通目录安装与 CLI 导出流程用于运行副本操作，不代替源码清单和 Rust 打包。不要直接修改应用数据目录中的副本。
