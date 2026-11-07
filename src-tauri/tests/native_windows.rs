// 功能：用本进程创建的隐藏窗口验证原生几何锁、圆角裁剪和桌面挂载，不操作用户窗口。
#![cfg(windows)]
#[path = "../src/platform/corners.rs"]
mod corners;
#[path = "../src/platform/desktop.rs"]
mod desktop;
#[path = "../src/platform/rules.rs"]
mod rules;
use std::ptr::null_mut;
use windows_sys::{
    core::w,
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    },
};

struct TestWindow(HWND);

impl TestWindow {
    /// 创建测试线程独占的隐藏原生窗口，完全独立于用户现有页面。
    fn new() -> Self {
        let handle = unsafe {
            CreateWindowExW(
                0,
                w!("STATIC"),
                w!("PinPage native test"),
                WS_POPUP | WS_THICKFRAME,
                100,
                120,
                400,
                300,
                null_mut(),
                null_mut(),
                null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!handle.is_null(), "无法创建隐藏测试窗口");
        Self(handle)
    }

    /// 读取实际窗口矩形，使用系统结果验证规则而非只检查配置值。
    fn rect(&self) -> RECT {
        let mut rect = RECT::default();
        assert_ne!(unsafe { GetWindowRect(self.0, &mut rect) }, 0);
        rect
    }

    /// 请求真实窗口移动和缩放，消息处理器会同步决定是否放行。
    fn place(&self, x: i32, y: i32, width: i32, height: i32) {
        assert_ne!(
            unsafe {
                SetWindowPos(
                    self.0,
                    null_mut(),
                    x,
                    y,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                )
            },
            0
        );
    }
}

impl Drop for TestWindow {
    /// 仅销毁本测试拥有的隐藏窗口，由 WM_NCDESTROY 释放规则数据。
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.0) != 0 {
                DestroyWindow(self.0);
            }
        }
    }
}

#[test]
/// 分别锁定位置和尺寸，再验证管理设置期间可以明确修改二者。
fn native_geometry_locks_are_independent() {
    let window = TestWindow::new();
    let policy = unsafe { &*rules::install(window.0).unwrap() };
    policy.configuring.set(false);
    policy.position_locked.set(true);
    window.place(200, 240, 500, 320);
    let rect = window.rect();
    assert_eq!(
        (
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top
        ),
        (100, 120, 500, 320)
    );
    policy.position_locked.set(false);
    policy.size_locked.set(true);
    window.place(250, 260, 700, 600);
    let rect = window.rect();
    assert_eq!(
        (
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top
        ),
        (250, 260, 500, 320)
    );
    policy.position_locked.set(true);
    policy.configuring.set(true);
    window.place(300, 310, 600, 340);
    policy.configuring.set(false);
    window.place(0, 0, 900, 800);
    let rect = window.rect();
    assert_eq!(
        (
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top
        ),
        (300, 310, 600, 340)
    );
}

#[test]
/// 圆角必须产生真正的窗口裁剪，取消后移除该区域。
fn rounded_region_can_be_enabled_and_removed() {
    let window = TestWindow::new();
    corners::apply(window.0, true, false, 1.25).unwrap();
    unsafe {
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(window.0, region), 0);
        assert_eq!(PtInRegion(region, 0, 0), 0);
        assert_ne!(PtInRegion(region, 100, 100), 0);
        DeleteObject(region);
        corners::apply(window.0, false, false, 1.25).unwrap();
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_eq!(GetWindowRgn(window.0, region), 0);
        DeleteObject(region);
    }
}

#[test]
/// 隐藏测试窗口能够挂载真实桌面图标宿主，并切回普通顶层窗口。
fn desktop_parent_and_styles_can_be_restored() {
    let window = TestWindow::new();
    let host = desktop::host().expect("当前测试环境需要 Windows Explorer 桌面宿主");
    desktop::set_parent(window.0, host).unwrap();
    unsafe {
        assert_eq!(GetParent(window.0), host);
        assert_ne!(GetWindowLongPtrW(window.0, GWL_STYLE) as u32 & WS_CHILD, 0);
        assert_eq!(GetWindowLongPtrW(window.0, GWL_STYLE) as u32 & WS_POPUP, 0);
    }
    desktop::place(window.0, 160, 180).unwrap();
    desktop::set_parent(window.0, null_mut()).unwrap();
    unsafe {
        assert!(GetParent(window.0).is_null());
        assert_eq!(GetWindowLongPtrW(window.0, GWL_STYLE) as u32 & WS_CHILD, 0);
        assert_ne!(GetWindowLongPtrW(window.0, GWL_STYLE) as u32 & WS_POPUP, 0);
    }
}
