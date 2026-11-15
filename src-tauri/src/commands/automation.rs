// 功能：为受信任工作台提供本机控制开关与公开技能的原生目录选择入口。
use crate::{
    errors::AppMessage,
    state::{self, AppState},
    windows,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// 读取开关与随包 skill 状态，不返回管道凭据。
#[tauri::command]
pub async fn automation_snapshot(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<Value, AppMessage> {
    windows::ensure_manager(&window)?;
    let mut status = pinpage_skills::status(None).unwrap_or_else(
        |error| json!({"error":error.message,"builtinVersion":pinpage_skills::VERSION}),
    );
    status["localControl"] = json!(
        app.state::<AppState>()
            .settings
            .lock()
            .unwrap()
            .local_control
    );
    status["cliPath"] = json!(std::env::current_exe()
        .ok()
        .map(|p| p.with_file_name("pinpage-cli.exe")));
    Ok(status)
}
/// 开关立即保存，失败恢复旧值；CLI 不拥有此接口。
#[tauri::command]
pub async fn automation_control(
    window: WebviewWindow,
    app: AppHandle,
    enabled: bool,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    let state = app.state::<AppState>();
    let _lock = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    let previous = state.settings.lock().unwrap().local_control;
    state.settings.lock().unwrap().local_control = enabled;
    if let Err(error) = state.save() {
        state.settings.lock().unwrap().local_control = previous;
        return Err(error);
    }
    drop(_lock);
    crate::automation::configure(&app, enabled).map_err(failed)?;
    if !enabled {
        crate::automation::diagnostics::stop_all(&app);
    }
    state::notify(&app);
    Ok(())
}
/// 自定义目录只来自原生选择器；与 CLI 共用安装模块，不在管理 WebView 中递归操作文件。
#[tauri::command]
pub async fn automation_skill(
    window: WebviewWindow,
    app: AppHandle,
    action: String,
    custom: bool,
) -> Result<Option<Value>, AppMessage> {
    windows::ensure_manager(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let dialog = app.dialog().file();
        let directory: Option<PathBuf> = if custom || action == "exportDirectory" {
            let Some(path) = app.dialog().file().blocking_pick_folder() else {
                return Ok(None);
            };
            Some(path.into_path().map_err(failed)?)
        } else {
            None
        };
        let result = match action.as_str() {
            "install" => pinpage_skills::install(directory.as_deref(), None, false, false),
            "update" => pinpage_skills::install(directory.as_deref(), None, true, false),
            "status" => pinpage_skills::status(directory.as_deref()),
            "exportDirectory" => pinpage_skills::export(directory.as_deref().unwrap(), false),
            "exportZip" => {
                let Some(file) = dialog
                    .set_file_name("pinpage-pages.zip")
                    .add_filter("ZIP", &["zip"])
                    .blocking_save_file()
                else {
                    return Ok(None);
                };
                pinpage_skills::export(&file.into_path().map_err(failed)?, false)
            }
            _ => return Err(failed("操作不合法")),
        }
        .map_err(failed)?;
        Ok(Some(result))
    })
    .await
    .map_err(failed)?
}
/// 技能冲突的机器码也保留在详情中，用户可以选择 CLI 的显式备份替换。
fn failed(error: impl ToString) -> AppMessage {
    AppMessage::new("errors.automationFailed").detail(error.to_string())
}
