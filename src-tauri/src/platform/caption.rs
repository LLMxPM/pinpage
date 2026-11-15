// 功能：统一内容窗口的非客户区策略，无边框关闭系统阴影和描边，系统边框恢复标题栏配色。
use crate::errors::AppMessage;
use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::Dwm::*,
    UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_STYLE, WS_CHILD},
};

// COLORREF 按 0x00BBGGRR 排列，与应用浅色界面保持一致。
const CAPTION: u32 = 0x00FA_F8F7;
const TEXT: u32 = 0x0038_2D25;
const BORDER: u32 = 0x00EC_E7E3;

/// 在所属 UI 线程随边框切换双向应用；禁用非客户区绘制但保留窗口样式与缩放命中。
pub fn apply(window: HWND, decorated: bool) -> Result<(), AppMessage> {
    // 普通桌面页已是子窗口，不参与 DWM 顶层外框绘制；切回浮动后会重新应用。
    if unsafe { GetWindowLongPtrW(window, GWL_STYLE) } as u32 & WS_CHILD != 0 {
        return Ok(());
    }
    // Windows 10 不支持配色属性；无边框仍由非客户区策略关闭系统阴影与外框。
    write(
        window,
        DWMWA_BORDER_COLOR,
        if decorated { BORDER } else { DWMWA_COLOR_NONE },
    );
    if decorated {
        write(window, DWMWA_CAPTION_COLOR, CAPTION);
        write(window, DWMWA_TEXT_COLOR, TEXT);
    }
    let policy = if decorated {
        DWMNCRP_USEWINDOWSTYLE
    } else {
        DWMNCRP_DISABLED
    };
    let result = unsafe {
        DwmSetWindowAttribute(
            window,
            DWMWA_NCRENDERING_POLICY as _,
            &policy as *const _ as _,
            std::mem::size_of_val(&policy) as u32,
        )
    };
    if result < 0 {
        return Err(AppMessage::new("errors.windowApplyFailed")
            .detail(format!("DWM 非客户区策略失败：0x{:08X}", result as u32)));
    }
    Ok(())
}

/// 单个属性失败不影响其余属性，也不能阻断窗口创建。
fn write(window: HWND, attribute: DWMWINDOWATTRIBUTE, color: u32) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            window,
            attribute as _,
            &color as *const u32 as _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}
