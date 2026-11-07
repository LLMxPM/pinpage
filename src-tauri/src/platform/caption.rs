// 功能：统一内容窗口系统标题栏的配色，让标题栏与页面背景衔接。
use windows_sys::Win32::{Foundation::HWND, Graphics::Dwm::DwmSetWindowAttribute};

// Windows 10 不认识这三个属性，调用失败时保留系统默认外观。
const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWA_CAPTION_COLOR: u32 = 35;
const DWMWA_TEXT_COLOR: u32 = 36;
// COLORREF 按 0x00BBGGRR 排列，与应用浅色界面保持一致。
const CAPTION: u32 = 0x00FA_F8F7;
const TEXT: u32 = 0x0038_2D25;
const BORDER: u32 = 0x00EC_E7E3;

/// 把标题栏、标题文字和边框调整为应用配色；不支持的系统静默跳过。
pub fn apply(window: HWND) {
    write(window, DWMWA_CAPTION_COLOR, CAPTION);
    write(window, DWMWA_TEXT_COLOR, TEXT);
    write(window, DWMWA_BORDER_COLOR, BORDER);
}

/// 单个属性失败不影响其余属性，也不能阻断窗口创建。
fn write(window: HWND, attribute: u32, color: u32) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            window,
            attribute as _,
            &color as *const u32 as _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}
