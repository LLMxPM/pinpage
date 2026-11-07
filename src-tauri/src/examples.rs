// 功能：将白名单中的内置工具写成独立站点副本，沿用导入页的加载和导出边界。
use crate::errors::AppMessage;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// IPC 仅接受已编译的示例标识，旧调用省略标识时仍使用计时器。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExampleId {
    #[default]
    Timer,
    Notes,
    Checklist,
    Clock,
}

/// 所有示例导入后统一从站点根入口加载。
pub const ENTRY: &str = "index.html";
type SiteFiles = &'static [(&'static str, &'static [u8])];
const WIDGET_STYLE: &[u8] = include_bytes!("../../examples/widgets/style.css");

/// 根据受信任枚举返回编译内嵌的文件，不使用输入拼接文件系统路径。
fn files(example: ExampleId) -> SiteFiles {
    match example {
        ExampleId::Timer => &[
            (
                "index.html",
                include_bytes!("../../examples/timer/index.html"),
            ),
            (
                "style.css",
                include_bytes!("../../examples/timer/style.css"),
            ),
            ("timer.js", include_bytes!("../../examples/timer/timer.js")),
            (
                "presets.json",
                include_bytes!("../../examples/timer/presets.json"),
            ),
        ],
        ExampleId::Notes => &[
            (
                "index.html",
                include_bytes!("../../examples/widgets/notes.html"),
            ),
            ("style.css", WIDGET_STYLE),
            (
                "notes.js",
                include_bytes!("../../examples/widgets/notes.js"),
            ),
        ],
        ExampleId::Checklist => &[
            (
                "index.html",
                include_bytes!("../../examples/widgets/checklist.html"),
            ),
            ("style.css", WIDGET_STYLE),
            (
                "checklist.js",
                include_bytes!("../../examples/widgets/checklist.js"),
            ),
        ],
        ExampleId::Clock => &[
            (
                "index.html",
                include_bytes!("../../examples/widgets/clock.html"),
            ),
            ("style.css", WIDGET_STYLE),
            (
                "clock.js",
                include_bytes!("../../examples/widgets/clock.js"),
            ),
        ],
    }
}

/// 示例展示名在签发票据时按当前应用语言解析。
pub fn label_key(example: ExampleId) -> &'static str {
    match example {
        ExampleId::Timer => "names.example",
        ExampleId::Notes => "names.notes",
        ExampleId::Checklist => "names.checklist",
        ExampleId::Clock => "names.clock",
    }
}

/// 统计被选中示例的文件数和体积，避免混入其他示例的数据。
pub fn measure(example: ExampleId) -> (u64, u64) {
    let selected = files(example);
    (
        selected.len() as u64,
        selected.iter().map(|(_, data)| data.len() as u64).sum(),
    )
}

/// 只写入当前票据选择的工具；返回的入口与用户导入页完全同构。
pub fn write_site(target: &Path, example: ExampleId) -> Result<String, AppMessage> {
    std::fs::create_dir_all(target)
        .map_err(|error| AppMessage::new("errors.createExampleFailed").detail(error))?;
    for (name, data) in files(example) {
        std::fs::write(target.join(name), data).map_err(|error| {
            AppMessage::new("errors.writeExampleFailed").detail(format!("{name}：{error}"))
        })?;
    }
    Ok(ENTRY.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 非白名单标识不能被反序列化，阻止把示例选择变为任意路径输入。
    fn rejects_unknown_examples() {
        for value in ["../notes", "unknown", "C:\\temp\\clock"] {
            assert!(serde_json::from_value::<ExampleId>(serde_json::json!(value)).is_err());
        }
        assert_eq!(
            serde_json::from_str::<ExampleId>("\"notes\"").unwrap(),
            ExampleId::Notes
        );
    }

    #[test]
    /// 每种工具的 HTML 仅引用随该工具打包的相对样式和脚本。
    fn all_example_resources_are_self_contained() {
        for example in [
            ExampleId::Timer,
            ExampleId::Notes,
            ExampleId::Checklist,
            ExampleId::Clock,
        ] {
            let selected = files(example);
            let html =
                std::str::from_utf8(selected.iter().find(|(name, _)| *name == ENTRY).unwrap().1)
                    .unwrap();
            for attribute in ["href=\"./", "src=\"./"] {
                for suffix in html.split(attribute).skip(1) {
                    let resource = suffix.split('"').next().unwrap();
                    assert!(
                        selected.iter().any(|(name, _)| *name == resource),
                        "missing {resource}"
                    );
                }
            }
            assert!(measure(example).1 > 0);
        }
    }
}
