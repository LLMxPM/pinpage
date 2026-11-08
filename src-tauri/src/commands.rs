// 功能：提供经过管理窗口身份校验的 IPC 命令入口，并协调窗口创建与状态通知。
pub mod session;
pub mod settings;
pub mod updates;

use crate::{
    errors::AppMessage,
    i18n,
    models::{CreatePin, PageSource, PinConfig, Snapshot},
    state::{self, AppState},
    tray, windows,
};
use tauri::{AppHandle, Manager, WebviewWindow};

/// 读取管理快照，只向受信任的管理窗口返回路径及会话配置。
#[tauri::command]
pub async fn snapshot(window: WebviewWindow, app: AppHandle) -> Result<Snapshot, AppMessage> {
    windows::ensure_manager(&window)?;
    Ok(app.state::<AppState>().snapshot())
}

/// 创建悬浮窗；导入来源必须来自原生选择器或内置来源签发的授权票据。
#[tauri::command]
pub async fn create_pin(
    window: WebviewWindow,
    app: AppHandle,
    input: CreatePin,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    // 站点副本以页面 ID 命名，先分配 ID 再复制，避免复制完成后还要改名。
    let id = format!("p-{}", uuid::Uuid::new_v4().simple());
    let source = crate::source::prepare(&app, &id, &input.source)?;
    let name = match &source {
        PageSource::Url { url } => user_name(&input.name, || {
            url::Url::parse(url)
                .ok()
                .and_then(|parsed| parsed.host_str().map(String::from))
                .unwrap_or_else(|| {
                    i18n::text(state::locale(&app), "names.webPage", &Default::default())
                })
        }),
        PageSource::Bundled { entry } => user_name(&input.name, || {
            std::path::Path::new(entry)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        }),
    };
    let pin = PinConfig {
        id,
        name,
        source,
        width: 420.,
        height: 360.,
        x: None,
        y: None,
        scale: 1.,
        always_on_top: input.always_on_top,
        decorations: input.decorations,
        enabled: true,
        mode: input.mode,
        rounded: false,
        position_locked: false,
        size_locked: false,
        preset: None,
    };
    state.settings.lock().unwrap().pins.push(pin.clone());
    state::schedule_save(&app);
    let result = windows::open_pin(&app, &pin);
    if let Err(error) = &result {
        crate::windows::show_manager(&app);
        state::report(&app, error.clone());
    }
    tray::refresh_or_report(&app);
    state::notify(&app);
    result
}

/// 从管理界面操作窗口，Rust 层再次校验目标 ID 与操作类型。
#[tauri::command]
pub async fn pin_action(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    action: String,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    windows::action(&app, &id, &action)
}

/// 更新单个页面设置，关闭的页面只保存配置，运行中的页面立即应用。
#[tauri::command]
pub async fn update_pin(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    input: crate::models::PinSettingsInput,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    windows::update(&app, &id, input)
}

/// 显式退出保留会话，与关闭单个悬浮窗的删除语义分离。
#[tauri::command]
pub async fn quit_app(window: WebviewWindow, app: AppHandle) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    windows::quit(&app);
    Ok(())
}

/// 把页面当前外观与几何存为自定义预设，返回新预设 ID。
#[tauri::command]
pub async fn save_preset(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    name: String,
) -> Result<String, AppMessage> {
    windows::ensure_manager(&window)?;
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    let pin = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))?;
    windows::presets::save(&app, &pin, &name)
}

/// 删除自定义预设；内置预设由代码定义，不接受删除。
#[tauri::command]
pub async fn delete_preset(
    window: WebviewWindow,
    app: AppHandle,
    preset_id: String,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    windows::presets::delete(&app, &preset_id)
}

/// 用户给定的名称优先，留空时使用来源推导的兜底名称。
fn user_name(input: &str, fallback: impl FnOnce() -> String) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        fallback()
    } else {
        trimmed.chars().take(80).collect()
    }
}
