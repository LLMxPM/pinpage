// 功能：只向 CLI 提供目标页真实外框，不开放任意 HWND 操作。
use pinpage_protocol::Failure;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
/// DWM 外框与 CLI 采集均使用物理屏幕坐标，页面模式保持原值。
#[cfg(windows)]
pub fn target(app: &AppHandle, id: &str) -> Result<Value, Failure> {
    use windows_sys::Win32::{
        Foundation::RECT,
        Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
    };
    let pin = super::pin(app, id)?;
    let window = app
        .get_webview_window(id)
        .ok_or_else(|| Failure::new("pageNotVisible", "页面未开启"))?;
    if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(true) {
        return Err(Failure::new(
            "pageNotVisible",
            "页面隐藏或最小化，请显式 --bring-to-front",
        ));
    }
    let hwnd = window
        .hwnd()
        .map_err(|e| Failure::new("captureUnavailable", e))?;
    let mut rect = RECT::default();
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd.0,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&mut rect as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if hr < 0 {
        return Err(Failure::new(
            "captureUnavailable",
            format!("DWM 外框查询失败：{hr:#x}"),
        ));
    }
    let inspected = super::inspect(app, id)?;
    Ok(
        json!({"instanceId":app.state::<super::AutomationState>().instance_id,"pageId":id,"revision":inspected["revision"],"loadId":inspected["loadId"],"mode":pin.mode,"bounds":{"left":rect.left,"top":rect.top,"right":rect.right,"bottom":rect.bottom}}),
    )
}
/// 其他平台不能用逻辑尺寸冒充物理桌面外框。
#[cfg(not(windows))]
pub fn target(_: &AppHandle, _: &str) -> Result<Value, Failure> {
    Err(Failure::new("unsupported", "当前平台不支持桌面截图"))
}
