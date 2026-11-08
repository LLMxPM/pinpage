// 功能：定义持久化配置、页面来源和前后端交换的数据结构。
use crate::errors::AppMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod editor;
mod workspace;
pub use editor::{PinContentInput, PresetInput, WindowChange, WindowPatch};
pub use workspace::{GestureModifier, Settings, SettingsInput, WorkspaceSettings};

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
    /// 当前匹配的预设 ID；调整窗口参数后清空，内容修改不影响它。
    #[serde(default)]
    pub preset: Option<String>,
}

impl PinConfig {
    /// 桌面模式使用无边框，浮动模式保留用户选择的系统边框。
    pub fn effective_decorations(&self) -> bool {
        self.mode == WindowMode::Floating && self.decorations
    }
}

/// 预设包含的外观与几何字段；内置表与用户自定义项共用同一份结构。
///
/// 位置使用目标显示器工作区内的归一化比例 `fx`/`fy`，0 表示贴左或贴上、
/// 1 表示贴右或贴下，避免绝对坐标在换屏或改 DPI 后失效。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetFields {
    pub mode: WindowMode,
    pub always_on_top: bool,
    pub decorations: bool,
    pub rounded: bool,
    pub position_locked: bool,
    pub size_locked: bool,
    pub width: f64,
    pub height: f64,
    pub fx: f64,
    pub fy: f64,
    /// 全屏项在应用时按目标显示器整屏尺寸换算；普通预设沿用表中的宽高。
    #[serde(default)]
    pub fullscreen: bool,
}

/// 用户自定义预设；内置预设不落盘，配置里只保存它的 ID 引用。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPreset {
    pub id: String,
    /// 自定义名称；预留在反序列化时容忍缺失，便于旧配置演进。
    #[serde(default)]
    pub name: String,
    #[serde(flatten)]
    pub fields: PresetFields,
}

/// 下发给管理界面的预设视图：内置项不带名称，由前端按 i18n 键渲染。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetView {
    pub id: String,
    pub name: Option<String>,
    pub builtin: bool,
    #[serde(flatten)]
    pub fields: PresetFields,
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
    /// 全量表单提交时携带“基于哪个预设”，与预设字段不一致时由前端置空。
    #[serde(default)]
    pub preset: Option<String>,
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

/// 无边框窗口的默认或固定拖动修饰键，与前端序列化契约共用。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum PlatformDragModifier {
    Alt,
    /// macOS 实验构建提供固定拖动手势，默认构建不声明此能力。
    Option,
}

/// 当前构建实际支持的窗口能力；前端裁剪入口，Rust 在每次应用前最终校验。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub desktop_mode: bool,
    pub rounded: bool,
    pub position_lock: bool,
    pub size_lock: bool,
    /// 是否允许配置手势修饰键；固定手势只通过 drag_modifier 表达。
    pub window_gestures: bool,
    /// None 表示没有原生修饰键手势，无边框窗口仍可通过托盘找回。
    pub drag_modifier: Option<PlatformDragModifier>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub runtime: HashMap<String, PinRuntime>,
    pub notices: Vec<AppMessage>,
    /// 内置与自定义预设，Rust 是唯一来源，前端只负责渲染。
    pub presets: Vec<PresetView>,
    /// Rust 已解析的有效语言，前端不再自行猜测系统语言。
    pub resolved_locale: AppLocale,
    /// 当前构建支持的窗口能力，管理界面据此裁剪所有入口。
    pub capabilities: PlatformCapabilities,
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
