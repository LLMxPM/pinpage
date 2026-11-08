// 功能：为无边框窗口应用真实原生圆角裁剪，兼容桌面子窗口和常规窗口。
use crate::errors::AppMessage;
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::{
        Dwm::*,
        Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn},
    },
    UI::WindowsAndMessaging::*,
};

/// 圆角半径（逻辑像素）；平台回退统一使用同一取值，避免不同渲染路径外观不一致。
const CORNER_RADIUS: f64 = 6.;

/// 系统边框使用系统外观，无边框按配置启用 6 逻辑像素圆角或完全直角。
pub fn apply(window: HWND, rounded: bool, decorated: bool, scale: f64) -> Result<(), AppMessage> {
    unsafe {
        let preference: i32 = if decorated {
            0
        } else if rounded {
            2
        } else {
            1
        };
        // Windows 10 不支持该属性，下面的窗口区域裁剪仍然生效。
        let _ = DwmSetWindowAttribute(
            window,
            DWMWA_WINDOW_CORNER_PREFERENCE as _,
            &preference as *const _ as _,
            std::mem::size_of::<i32>() as u32,
        );
        if decorated || !rounded {
            if SetWindowRgn(window, std::ptr::null_mut(), 1) == 0 {
                return Err(AppMessage::new("errors.cornerClearFailed"));
            }
        } else {
            let mut rect = RECT::default();
            if GetWindowRect(window, &mut rect) == 0 {
                return Err(AppMessage::new("errors.cornerRectFailed"));
            }
            // DWM 系统圆角固定约 8 逻辑像素，无法与 6px 统一；各平台与桌面挂载路径统一使用区域裁剪。
            let diameter = ((CORNER_RADIUS * scale).round() as i32).max(1) * 2;
            let region = CreateRoundRectRgn(
                0,
                0,
                rect.right - rect.left + 1,
                rect.bottom - rect.top + 1,
                diameter,
                diameter,
            );
            if region.is_null() {
                return Err(AppMessage::new("errors.cornerCreateFailed"));
            }
            if SetWindowRgn(window, region, 1) == 0 {
                DeleteObject(region);
                return Err(AppMessage::new("errors.cornerApplyFailed"));
            }
        }
    }
    Ok(())
}
