// 功能：用真实 HWND 检查系统边框、无边框及圆角切换后的 DWM 状态和窗口区域。
use super::{caption, corners, desktop, rules, TestWindow};
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::{Dwm::*, Gdi::*},
    UI::WindowsAndMessaging::*,
};

/// 直接读取 DWM 状态；新系统属性在旧 Windows 上不可用时返回 None。
fn attribute(window: HWND, attribute: DWMWINDOWATTRIBUTE) -> Option<u32> {
    let mut value = 0u32;
    let result = unsafe {
        DwmGetWindowAttribute(
            window,
            attribute as _,
            &mut value as *mut _ as _,
            std::mem::size_of_val(&value) as u32,
        )
    };
    (result >= 0).then_some(value)
}

/// 检查系统绘制和圆角策略，不以配置布尔值代替 DWM 实际结果。
fn assert_system_frame(window: HWND, decorated: bool) {
    let child = unsafe { GetWindowLongPtrW(window, GWL_STYLE) } as u32 & WS_CHILD != 0;
    // 隐藏窗口即使启用系统边框也可能报告未绘制；无边框的禁用状态必须明确成立。
    if !decorated && !child {
        assert_eq!(attribute(window, DWMWA_NCRENDERING_ENABLED), Some(0));
    }
    if !child {
        if let Some(color) = attribute(window, DWMWA_BORDER_COLOR) {
            assert_eq!(color == DWMWA_COLOR_NONE, !decorated, "系统描边存在残留");
        }
    }
    if let Some(preference) = attribute(window, DWMWA_WINDOW_CORNER_PREFERENCE) {
        assert_eq!(
            preference,
            if decorated {
                DWMWCP_DEFAULT
            } else {
                DWMWCP_DONOTROUND
            } as u32
        );
    }
}

/// 顶层窗口检查真实裁剪点位及取消后的区域清除；桌面挂载后的系统可见区域另行处理。
fn assert_frame(window: HWND, decorated: bool, rounded: bool) {
    assert_system_frame(window, decorated);
    unsafe {
        let region = CreateRectRgn(0, 0, 0, 0);
        assert!(!region.is_null());
        let kind = GetWindowRgn(window, region);
        if rounded && !decorated {
            assert_ne!(kind, 0);
            assert_eq!(PtInRegion(region, 0, 0), 0);
            assert_ne!(PtInRegion(region, 100, 100), 0);
        } else {
            assert_eq!(kind, 0, "直角或系统边框遗留了裁剪区域");
        }
        DeleteObject(region);
    }
}

#[test]
/// 往返三种边框，验证无边框清除系统外观，系统边框恢复绘制且不残留圆角区域。
fn frame_switches_clear_system_outline_and_restore_decorations() {
    for extended in [0, WS_EX_NOREDIRECTIONBITMAP] {
        let window = TestWindow::with_extended_style(extended);
        for _ in 0..3 {
            for (decorated, rounded) in
                [(true, false), (false, true), (false, false), (true, false)]
            {
                caption::apply(window.0, decorated).unwrap();
                corners::apply(window.0, rounded, decorated, 1.25).unwrap();
                assert_frame(window.0, decorated, rounded);
                window.place(100, 120, 480, 320);
                corners::apply(window.0, rounded, decorated, 1.25).unwrap();
                assert_frame(window.0, decorated, rounded);
            }
        }
    }
}

#[test]
/// 模拟配置事务：先切换桌面挂载和缓存样式，再应用外观；验证顶层 DWM 策略恢复。
fn borderless_desktop_roundtrips_keep_system_rendering_disabled() {
    let window = TestWindow::with_extended_style(WS_EX_NOREDIRECTIONBITMAP);
    let host = desktop::host().expect("当前测试环境需要 Windows Explorer 桌面宿主");
    let policy = unsafe { &*rules::install(window.0).unwrap() };
    policy.decorated.set(false);
    for parent in [null_mut(), host, null_mut(), host, null_mut()] {
        policy.desktop_child.set(!parent.is_null());
        desktop::set_parent(window.0, parent).unwrap();
        for rounded in [false, true] {
            unsafe {
                SetWindowLongPtrW(window.0, GWL_STYLE, WS_OVERLAPPEDWINDOW as isize);
                // 与 Tao 一样提交非客户区重算，不能停在样式已改但外框尚未刷新的中间态。
                SetWindowPos(
                    window.0,
                    null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
            }
            caption::apply(window.0, false).unwrap();
            corners::apply(window.0, rounded, false, 1.25).unwrap();
            assert_system_frame(window.0, false);
        }
    }
}
