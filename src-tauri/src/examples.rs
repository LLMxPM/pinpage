// 功能：通过稳定白名单选择内嵌的标准 .pinpage，示例与外部页面包共用校验和解包。
use crate::{errors::AppMessage, package};
use serde::{Deserialize, Serialize};
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/example_packages.rs"));

/// IPC 仅接受已登记的示例标识，旧调用省略标识时仍使用计时器。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExampleId {
    #[default]
    Timer,
    Notes,
    Checklist,
    Clock,
}

/// 白名单标识映射到构建时生成的标准包，不从调用方输入拼接文件路径。
fn package_bytes(example: ExampleId) -> &'static [u8] {
    let id = match example {
        ExampleId::Timer => "timer",
        ExampleId::Notes => "notes",
        ExampleId::Checklist => "checklist",
        ExampleId::Clock => "clock",
    };
    PACKAGES
        .iter()
        .find(|(name, _)| *name == id)
        .expect("内置示例缺少标准页面包")
        .1
}

/// 示例名称按当前应用语言解析，页面内容仍使用包内的默认语言。
pub fn label_key(example: ExampleId) -> &'static str {
    match example {
        ExampleId::Timer => "names.example",
        ExampleId::Notes => "names.notes",
        ExampleId::Checklist => "names.checklist",
        ExampleId::Clock => "names.clock",
    }
}

/// 用标准包解析器读取预览和推荐布局，字节数按实际压缩包计算。
pub fn preview(example: ExampleId) -> Result<(package::Manifest, u64), AppMessage> {
    let bytes = package_bytes(example);
    Ok((package::read_manifest_bytes(bytes)?, bytes.len() as u64))
}

/// 沿用外部包的解包边界，支持子目录、页面数据与可选默认窗口预设。
pub fn extract(target: &Path, example: ExampleId) -> Result<package::Extracted, AppMessage> {
    package::extract_bytes(package_bytes(example), target)
}

#[cfg(test)]
#[path = "examples_tests.rs"]
mod tests;
