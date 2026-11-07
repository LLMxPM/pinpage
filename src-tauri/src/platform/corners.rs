// 功能：为无边框窗口应用真实原生圆角裁剪，兼容桌面子窗口和常规窗口。
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::{
        Dwm::*,
        Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn},
    },
    UI::WindowsAndMessaging::*,
};

/// 系统边框使用系统外观，无边框按配置启用 12 逻辑像素圆角或完全直角。
pub fn apply(window: HWND, rounded: bool, decorated: bool, scale: f64) -> Result<(), String> {
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
                return Err("无法清除窗口圆角区域".into());
            }
        } else {
            let mut rect = RECT::default();
            if GetWindowRect(window, &mut rect) == 0 {
                return Err("无法读取圆角窗口尺寸".into());
            }
            let diameter = (24. * scale).round() as i32;
            let region = CreateRoundRectRgn(
                0,
                0,
                rect.right - rect.left + 1,
                rect.bottom - rect.top + 1,
                diameter,
                diameter,
            );
            if region.is_null() {
                return Err("无法创建圆角窗口区域".into());
            }
            if SetWindowRgn(window, region, 1) == 0 {
                DeleteObject(region);
                return Err("无法应用圆角窗口区域".into());
            }
        }
    }
    Ok(())
}
