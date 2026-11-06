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
