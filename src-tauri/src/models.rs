// 功能：定义持久化配置、页面来源和前后端交换的数据结构。
use crate::errors::AppMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 用户语言偏好，随配置持久化；`system` 表示由平台解析。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalePreference {
    #[serde(rename = "zh-CN")]
    #[default]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "system")]
    System,
}

/// 已解析的有效语言，即当前实际使用的翻译资源。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLocale {
    #[serde(rename = "zh-CN")]
    #[default]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PageSource {
    Url {
        url: String,
    },
    /// 导入来源：入口是相对应用数据目录中站点副本的路径，原文件缺失也能加载。
    Bundled {
        entry: String,
    },
}

/// 导入方式：目录与单文件来自原生选择器，页面包与内置示例由应用直接提供。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportMode {
    Directory,
    File,
    Package,
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
    /// 缺省表示沿用现有来源；只有用户明确改动地址或重新导入时才提交。
    #[serde(default)]
    pub source: Option<SourceInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    /// 语言偏好未知或类型不符时只回退该字段，不影响页面与快捷键。
    #[serde(deserialize_with = "lenient_locale")]
    pub locale: LocalePreference,
    pub restore_session: bool,
    pub manager_shortcut: String,
    pub recover_shortcut: String,
    /// 单条无法识别的记录只被跳过，不会让整份配置回退为默认值。
    #[serde(deserialize_with = "lenient_pins")]
    pub pins: Vec<PinConfig>,
}

/// 逐条解析页面记录，跳过来源类型已不再支持的旧条目。
fn lenient_pins<'de, D>(deserializer: D) -> Result<Vec<PinConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect())
}

/// 语言字段容错：任何无法识别的取值都只回退中文，不让整份配置解析失败。
fn lenient_locale<'de, D>(deserializer: D) -> Result<LocalePreference, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

impl Default for Settings {
    /// 提供首版配置与默认快捷键，允许旧配置缺失新增字段。
    fn default() -> Self {
        Self {
            schema_version: 1,
            locale: LocalePreference::default(),
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
    /// 只保存消息码与参数，切换语言后可以重新渲染。
    pub error: Option<AppMessage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub runtime: HashMap<String, PinRuntime>,
    pub notices: Vec<AppMessage>,
    /// Rust 已解析的有效语言，前端不再自行猜测系统语言。
    pub resolved_locale: AppLocale,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceInput {
    Url {
        address: String,
    },
    /// 导入来源：票据由原生选择器或应用内置来源签发，入口为用户确认的相对路径。
    Import {
        ticket: String,
        entry: String,
    },
}

/// 导出完成后返回给管理界面的结果。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub files: u64,
    /// 页面数据是否成功打包；超时或页面不支持时只导出站点副本。
    pub with_data: bool,
    /// 页面数据快照的字节数，未包含数据时为 0。
    pub data_bytes: u64,
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

/// 原生选择器完成扫描后返回的导入预览，前端确认入口后才真正复制文件。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub ticket: String,
    pub mode: ImportMode,
    /// 展示用的来源描述，目录导入为目录名，单文件导入为文件名。
    pub label: String,
    /// 可选的入口相对路径；目录导入可能返回多个根级 HTML。
    pub candidates: Vec<String>,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    /// 严格校验：只接受三个约定值字符串，非法输入由命令层返回明确错误。
    pub locale: String,
    pub restore_session: bool,
    pub manager_shortcut: String,
    pub recover_shortcut: String,
}

/// 全局设置的可编辑部分，语言只影响应用界面。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSettings {
    pub locale: LocalePreference,
    pub restore_session: bool,
    pub manager_shortcut: String,
    pub recover_shortcut: String,
}

impl WorkspaceSettings {
    /// 从 IPC 输入构造候选设置，语言取值必须落在约定集合内。
    pub fn from_input(input: &SettingsInput) -> Result<Self, AppMessage> {
        let locale = match input.locale.as_str() {
            "zh-CN" => LocalePreference::ZhCn,
            "en-US" => LocalePreference::EnUs,
            "system" => LocalePreference::System,
            _ => return Err(AppMessage::new("errors.localeInvalid")),
        };
        Ok(Self {
            locale,
            restore_session: input.restore_session,
            manager_shortcut: input.manager_shortcut.clone(),
            recover_shortcut: input.recover_shortcut.clone(),
        })
    }

    /// 从当前配置读出可编辑字段，用于失败回滚前的快照。
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            locale: settings.locale,
            restore_session: settings.restore_session,
            manager_shortcut: settings.manager_shortcut.clone(),
            recover_shortcut: settings.recover_shortcut.clone(),
        }
    }
}
