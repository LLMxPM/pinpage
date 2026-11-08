// 功能：手势期间保留宿主焦点以接收 Esc，结束后恢复宿主正常的 WebView 焦点转交。
use super::*;

/// CAPTURE 必须先设置，以阻止 Wry 在 WM_SETFOCUS 中立即把焦点交回网页。
pub(super) fn acquire(hwnd: HWND) {
    unsafe {
        // 桌面子窗口不把 Explorer 整体抬到前台。
        if GetWindowLongPtrW(hwnd, GWL_STYLE) & WS_CHILD as isize == 0 {
            SetForegroundWindow(hwnd);
        }
        SetFocus(hwnd);
    }
}

/// 只恢复仍属于本次实例且仍实际持有键盘焦点的宿主，不抢回其他窗口的焦点。
pub(super) fn restore(active: Capture) {
    if !target_for(active.instance).is_some_and(|target| target.hwnd == active.hwnd as usize) {
        return;
    }
    unsafe {
        if GetFocus() == active.hwnd && IsWindowVisible(active.hwnd) != 0 {
            // 宿主此刻确实持有焦点；重走正常消息链，让 Wry 的现有处理器选择 WebView 焦点。
            // 同一 UI 线程同步执行，窗口实例不能在校验和发送之间被关闭或重用。
            SendMessageW(active.hwnd, WM_SETFOCUS, 0, 0);
        }
    }
}
