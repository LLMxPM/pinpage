// 功能：提供仅受信任管理界面可用的窗口局部修改、内容保存和预设编辑命令。
use crate::{
    errors::AppMessage,
    models::{PinContentInput, PresetInput, WindowChange, WindowPatch},
    state::AppState,
    windows,
};
use tauri::{AppHandle, Manager, WebviewWindow};

/// 提交局部窗口参数，原生层以最新窗口状态合并并返回最终结果。
#[tauri::command]
pub async fn update_window(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    input: WindowPatch,
    expected: Option<WindowPatch>,
) -> Result<WindowChange, AppMessage> {
    windows::ensure_manager(&window)?;
    windows::editor::update_window(&app, &id, input, expected)
}

/// 保存名称与来源，不覆盖窗口参数；来源改动才触发导航。
#[tauri::command]
pub async fn update_content(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    input: PinContentInput,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    windows::editor::update_content(&app, &id, input)
}

/// 创建或更新自定义预设；内置项、输入范围和写入状态均由 Rust 校验。
#[tauri::command]
pub async fn write_preset(
    window: WebviewWindow,
    app: AppHandle,
    preset_id: Option<String>,
    input: PresetInput,
) -> Result<String, AppMessage> {
    windows::ensure_manager(&window)?;
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    windows::presets::write(&app, preset_id.as_deref(), input)
}
