// 功能：用本进程创建的隐藏窗口验证原生几何锁、圆角裁剪和桌面挂载，不操作用户窗口。
#![cfg(windows)]
// 被测的平台模块只依赖消息契约，单独引入以便复用同一份错误定义。
#[path = "../src/platform/corners.rs"]
mod corners;
#[path = "../src/platform/desktop.rs"]
mod desktop;
#[allow(dead_code)]
#[path = "../src/errors.rs"]
mod errors;
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
        Self::with_extended_style(0)
    }

    /// 扩展样式在创建时确定，模拟真实内容窗口的合成表面初始化。
    fn with_extended_style(extended: u32) -> Self {
        let handle = unsafe {
            CreateWindowExW(
                extended,
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

#[test]
/// 显示或隐藏时重写缓存样式，不得清除桌面子窗口身份、透明合成和常规鼠标命中。
fn desktop_composition_survives_cached_style_updates() {
    let window = TestWindow::with_extended_style(WS_EX_NOREDIRECTIONBITMAP);
    let policy = unsafe { &*rules::install(window.0).unwrap() };
    let host = desktop::host().expect("当前测试环境需要 Windows Explorer 桌面宿主");
    for parent in [host, null_mut(), host, null_mut()] {
        policy.desktop_child.set(!parent.is_null());
        desktop::set_parent(window.0, parent).unwrap();
        if !parent.is_null() {
            // Tao 的 show/hide 会按创建时的顶层身份重写整组窗口样式。
            unsafe {
                SetWindowLongPtrW(
                    window.0,
                    GWL_STYLE,
                    (WS_OVERLAPPEDWINDOW | WS_CLIPSIBLINGS) as isize,
                );
            }
        }
        corners::apply(window.0, true, false, 1.25).unwrap();
        desktop::place(window.0, 100, 100).unwrap();
        unsafe {
            let extended = GetWindowLongPtrW(window.0, GWL_EXSTYLE) as u32;
            assert_ne!(extended & WS_EX_NOREDIRECTIONBITMAP, 0);
            assert_eq!(extended & (WS_EX_LAYERED | WS_EX_TRANSPARENT), 0);
            assert_eq!(GetParent(window.0), parent);
            assert_eq!(
                GetWindowLongPtrW(window.0, GWL_STYLE) as u32 & WS_CHILD != 0,
                !parent.is_null()
            );
        }
    }
}

#[test]
/// 透明桌面页保持顶层合成与屏幕坐标，桌面拥有关系不能被误当成子窗口父级。
fn transparent_desktop_owner_preserves_screen_coordinates() {
    let owner = TestWindow::new();
    let window = TestWindow::with_extended_style(WS_EX_NOREDIRECTIONBITMAP);
    desktop::set_owner(window.0, owner.0).unwrap();
    assert!(desktop::child_parent(window.0).is_null());
    desktop::place(window.0, -200, 260).unwrap();
    unsafe {
        assert_eq!(GetWindow(window.0, GW_OWNER), owner.0);
        assert_eq!(GetWindowLongPtrW(window.0, GWL_STYLE) as u32 & WS_CHILD, 0);
        assert_eq!(
            GetWindowLongPtrW(window.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST,
            0
        );
        let rect = desktop::parent_rect(window.0);
        assert_eq!((rect.left, rect.top), (-200, 260));
        assert_eq!((window.rect().left, window.rect().top), (-200, 260));
    }
    desktop::set_owner(window.0, null_mut()).unwrap();
    let host = desktop::host().unwrap();
    desktop::set_parent(window.0, host).unwrap();
    assert_eq!(desktop::child_parent(window.0), host);
    assert!(desktop::set_owner(window.0, owner.0).is_err());
    desktop::set_parent(window.0, null_mut()).unwrap();
    unsafe {
        assert!(GetWindow(window.0, GW_OWNER).is_null());
    }
}

#[test]
/// 首次显示或缓存样式重写后，透明桌面页仍为桌面拥有的弹出窗口；切回浮动时解除保护。
fn transparent_desktop_popup_survives_visibility_style_updates() {
    let owner = TestWindow::new();
    let window = TestWindow::with_extended_style(WS_EX_NOREDIRECTIONBITMAP);
    let policy = unsafe { &*rules::install(window.0).unwrap() };
    policy.desktop_popup.set(true);
    desktop::set_owner(window.0, owner.0).unwrap();
    for _ in 0..3 {
        unsafe {
            SetWindowLongPtrW(window.0, GWL_STYLE, WS_OVERLAPPEDWINDOW as isize);
            let style = GetWindowLongPtrW(window.0, GWL_STYLE) as u32;
            assert_ne!(style & WS_POPUP, 0);
            assert_eq!(style & WS_CHILD, 0);
            assert_eq!(GetWindow(window.0, GW_OWNER), owner.0);
        }
    }
    policy.desktop_popup.set(false);
    desktop::set_owner(window.0, null_mut()).unwrap();
    unsafe {
        SetWindowLongPtrW(window.0, GWL_STYLE, WS_OVERLAPPEDWINDOW as isize);
        assert_eq!(
            GetWindowLongPtrW(window.0, GWL_STYLE) as u32 & (WS_POPUP | WS_CHILD),
            0
        );
    }
}
