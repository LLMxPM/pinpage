// 功能：定义窗口局部修改、页面内容保存及预设编辑的严格 IPC 输入。
use super::{PinConfig, PresetFields, SourceInput, WindowMode};
use serde::{Deserialize, Serialize};

/// 返回原生事务前后状态，撤销不依赖前端订阅到达的时机。
#[derive(Serialize)]
pub struct WindowChange {
    pub before: PinConfig,
    pub after: PinConfig,
}

/// 窗口仅提交实际修改的字段；不接受内容、身份或开启状态。
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowPatch {
    pub mode: Option<WindowMode>,
    pub always_on_top: Option<bool>,
    pub decorations: Option<bool>,
    pub rounded: Option<bool>,
    pub position_locked: Option<bool>,
    pub size_locked: Option<bool>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

/// 内容修改与窗口设置隔离，来源缺省表示不导航、不复制副本。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PinContentInput {
    pub name: String,
    pub source: Option<SourceInput>,
}

/// 预设编辑沿用归一化位置，不接受内置标志或页面来源。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInput {
    pub name: String,
    #[serde(flatten)]
    pub fields: PresetFields,
}
