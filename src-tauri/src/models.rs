// 功能：定义持久化配置、页面来源和前后端交换的数据结构。
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PageSource {
    Url { url: String },
    Local { entry: String, root: String },
    Example,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowMode {
    #[default]
    Floating,
    Desktop,
}

/// 旧版本没有开关字段时，原有页面保持开启状态。
fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinConfig {
    pub id: String,
    pub name: String,
    pub source: PageSource,
    pub width: f64,
    pub height: f64,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub scale: f64,
    pub always_on_top: bool,
    pub decorations: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub mode: WindowMode,
    #[serde(default)]
    pub rounded: bool,
    #[serde(default)]
    pub position_locked: bool,
    #[serde(default)]
    pub size_locked: bool,
}

impl PinConfig {
    /// 桌面模式使用无边框，浮动模式保留用户选择的系统边框。
    pub fn effective_decorations(&self) -> bool {
        self.mode == WindowMode::Floating && self.decorations
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinSettingsInput {
    pub name: String,
    pub mode: WindowMode,
    pub always_on_top: bool,
    pub decorations: bool,
    pub rounded: bool,
    pub position_locked: bool,
    pub size_locked: bool,
    pub width: f64,
    pub height: f64,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub restore_session: bool,
    pub manager_shortcut: String,
    pub recover_shortcut: String,
    pub pins: Vec<PinConfig>,
}

impl Default for Settings {
    /// 提供首版配置与默认快捷键，允许旧配置缺失新增字段。
    fn default() -> Self {
        Self {
            schema_version: 1,
            restore_session: true,
            manager_shortcut: "Ctrl+Alt+P".into(),
            recover_shortcut: "Ctrl+Alt+B".into(),
            pins: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PinRuntime {
    pub open: bool,
    pub visible: bool,
    pub loading: bool,
    pub current_url: String,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub runtime: HashMap<String, PinRuntime>,
    pub notices: Vec<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceInput {
    Url { address: String },
    Local { ticket: String },
    Example,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePin {
    pub name: String,
    pub source: SourceInput,
    pub always_on_top: bool,
    pub decorations: bool,
    #[serde(default)]
    pub mode: WindowMode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalChoice {
    pub ticket: String,
    pub entry: String,
    pub root: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    pub restore_session: bool,
    pub manager_shortcut: String,
    pub recover_shortcut: String,
}
