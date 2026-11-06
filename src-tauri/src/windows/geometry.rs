// 功能：依据当前屏幕工作区恢复窗口，确保断开副屏后窗口仍可操作。
use crate::models::PinConfig;
use tauri::{PhysicalPosition, WebviewWindow};

/// 校正位置到现有显示器，优先选择原位置所在屏幕，否则使用主屏。
pub fn restore(window: &WebviewWindow, pin: &PinConfig) -> Result<(), String> {
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let target = pin
        .x
        .zip(pin.y)
        .and_then(|(x, y)| {
            monitors
                .iter()
                .find(|m| {
                    let area = m.work_area();
                    x >= area.position.x
                        && y >= area.position.y
                        && x < area.position.x + area.size.width as i32
                        && y < area.position.y + area.size.height as i32
                })
                .cloned()
        })
        .or(window.primary_monitor().map_err(|e| e.to_string())?);
    if let Some(monitor) = target {
        let area = monitor.work_area();
        let size = window.outer_size().map_err(|e| e.to_string())?;
        let x = pin.x.unwrap_or(area.position.x + 60);
        let y = pin.y.unwrap_or(area.position.y + 60);
        let x = x.clamp(
            area.position.x,
            area.position.x + (area.size.width.saturating_sub(size.width)) as i32,
        );
        let y = y.clamp(
            area.position.y,
            area.position.y + (area.size.height.saturating_sub(size.height)) as i32,
        );
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
    } else {
        window.center().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 将找回的窗口居中到当前主屏，防止边框恢复后仍位于屏幕外。
pub fn recover(window: &WebviewWindow) -> Result<(), String> {
    window.center().map_err(|e| e.to_string())
}
