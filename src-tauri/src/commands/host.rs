// 功能：内容页宿主信息与系统监控命令入口，只暴露聚合只读数据。
use crate::{errors::AppMessage, host, state::AppState, windows};
use tauri::{AppHandle, Manager, WebviewWindow};

/// 返回宿主版本、平台、语言与页面能力表，供页面特性探测与降级。
#[tauri::command]
pub async fn host_info(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<host::HostInfo, AppMessage> {
    windows::ensure_content(&window, &app)?;
    Ok(host::info(&app))
}

/// 返回共享采样器的最新系统快照；采样未就绪时明确报错，不阻塞调用。
#[tauri::command]
pub async fn system_stats(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<host::SystemStats, AppMessage> {
    windows::ensure_content(&window, &app)?;
    app.state::<AppState>()
        .stats
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| AppMessage::new("errors.statsUnavailable"))
}
