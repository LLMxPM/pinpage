// 功能：仅为受信任管理窗口提供升级快照和固定升级动作。
use crate::{errors::AppMessage, updates, windows};
use tauri::{AppHandle, WebviewWindow};

/// 返回 Rust 持有的状态，管理界面关闭重建不会丢失后台进度。
#[tauri::command]
pub async fn updates_snapshot(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<updates::Snapshot, AppMessage> {
    windows::ensure_manager(&window)?;
    Ok(updates::snapshot(&app))
}

/// 动作由后端白名单及状态机校验，不接受远程地址和本地可执行文件输入。
#[tauri::command]
pub async fn update_action(
    window: WebviewWindow,
    app: AppHandle,
    action: String,
    enabled: Option<bool>,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    updates::action(&app, &action, enabled)
}
