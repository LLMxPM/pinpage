// 功能：定位桌面宿主，管理普通页面的子窗口挂载和透明页面的顶层桌面拥有关系。
use crate::errors::AppMessage;
use std::ptr::null_mut;
use windows_sys::Win32::Graphics::Gdi::MapWindowPoints;
use windows_sys::{
    core::w,
    Win32::{
        Foundation::{GetLastError, SetLastError, HWND, LPARAM, POINT, RECT},
        UI::WindowsAndMessaging::*,
    },
};

/// 只检查 Explorer 桌面宿主，覆盖图标层位于 Progman 或 WorkerW 的布局。
pub fn host() -> Result<HWND, AppMessage> {
    let mut result: HWND = null_mut();
    unsafe {
        EnumWindows(Some(find_host), &mut result as *mut HWND as LPARAM);
    }
    if result.is_null() {
        Err(AppMessage::new("errors.desktopUnavailable"))
    } else {
        Ok(result)
    }
}

/// 遍历顶层桌面窗口，选择其 SHELLDLL_DefView 图标层，避免改变 Explorer 窗口本身。
unsafe extern "system" fn find_host(window: HWND, data: LPARAM) -> i32 {
    let mut class = [0u16; 64];
    let count = GetClassNameW(window, class.as_mut_ptr(), class.len() as i32);
    let class = String::from_utf16_lossy(&class[..count.max(0) as usize]);
    if class == "Progman" || class == "WorkerW" {
        let child = FindWindowExW(window, null_mut(), w!("SHELLDLL_DefView"), std::ptr::null());
        if !child.is_null() {
            *(data as *mut HWND) = child;
            return 0;
        }
        EnumChildWindows(window, Some(find_icon_view), data);
        if !(*(data as *mut HWND)).is_null() {
            return 0;
        }
    }
    1
}

/// 新桌面布局可能多包一层 WorkerW，递归查找图标宿主。
unsafe extern "system" fn find_icon_view(window: HWND, data: LPARAM) -> i32 {
    let mut class = [0u16; 64];
    let count = GetClassNameW(window, class.as_mut_ptr(), class.len() as i32);
    if String::from_utf16_lossy(&class[..count.max(0) as usize]) == "SHELLDLL_DefView" {
        *(data as *mut HWND) = window;
        return 0;
    }
    1
}

/// 改变父窗口与必要样式；失败时恢复原样式并返回系统错误。
pub fn set_parent(window: HWND, parent: HWND) -> Result<(), AppMessage> {
    unsafe {
        let old_parent = child_parent(window);
        let old_style = GetWindowLongPtrW(window, GWL_STYLE);
        let next_style = if parent.is_null() {
            (old_style & !(WS_CHILD as isize)) | WS_POPUP as isize
        } else {
            (old_style & !(WS_POPUP as isize)) | WS_CHILD as isize
        };
        if old_parent == parent {
            SetWindowLongPtrW(window, GWL_STYLE, next_style);
            return Ok(());
        }
        if !parent.is_null() {
            SetWindowLongPtrW(window, GWL_STYLE, next_style);
        }
        SetLastError(0);
        let previous = SetParent(window, parent);
        if previous.is_null() && GetLastError() != 0 {
            let error = std::io::Error::last_os_error();
            SetWindowLongPtrW(window, GWL_STYLE, old_style);
            return Err(AppMessage::new("errors.desktopModeFailed").detail(error));
        }
        if parent.is_null() {
            SetWindowLongPtrW(window, GWL_STYLE, next_style);
        }
    }
    Ok(())
}

/// 只返回真实子窗口的父级；顶层拥有关系不得参与客户区坐标换算。
pub fn child_parent(window: HWND) -> HWND {
    unsafe {
        if GetWindowLongPtrW(window, GWL_STYLE) as u32 & WS_CHILD != 0 {
            GetParent(window)
        } else {
            null_mut()
        }
    }
}

/// 透明桌面页保留顶层合成，由 Explorer 顶层宿主拥有；不修改 Explorer 的窗口样式或位置。
pub fn set_owner(window: HWND, owner: HWND) -> Result<(), AppMessage> {
    unsafe {
        if !child_parent(window).is_null() {
            return Err(
                AppMessage::new("errors.windowApplyFailed").detail("子窗口不能设置顶层拥有关系")
            );
        }
        let owner = if owner.is_null() {
            null_mut()
        } else {
            let root = GetAncestor(owner, GA_ROOT);
            if root.is_null() {
                return Err(AppMessage::new("errors.desktopUnavailable"));
            }
            root
        };
        if GetWindow(window, GW_OWNER) == owner {
            return Ok(());
        }
        SetLastError(0);
        if SetWindowLongPtrW(window, GWLP_HWNDPARENT, owner as isize) == 0 && GetLastError() != 0 {
            return Err(
                AppMessage::new("errors.desktopModeFailed").detail(std::io::Error::last_os_error())
            );
        }
    }
    Ok(())
}

/// 仅子窗口换算父坐标；透明桌面拥有窗口保留屏幕坐标，并置于普通应用窗口之下。
pub fn place(window: HWND, x: i32, y: i32) -> Result<(), AppMessage> {
    unsafe {
        let parent = child_parent(window);
        let mut point = POINT { x, y };
        if !parent.is_null() {
            MapWindowPoints(null_mut(), parent, &mut point, 1);
        }
        let owned = !GetWindow(window, GW_OWNER).is_null();
        let after = if !parent.is_null() {
            HWND_TOP
        } else if owned {
            HWND_BOTTOM
        } else {
            null_mut()
        };
        let flags = SWP_NOSIZE
            | SWP_NOACTIVATE
            | SWP_FRAMECHANGED
            | SWP_NOOWNERZORDER
            | if parent.is_null() && !owned {
                SWP_NOZORDER
            } else {
                0
            };
        if SetWindowPos(window, after, point.x, point.y, 0, 0, flags) == 0 {
            return Err(AppMessage::new("errors.setWindowPosFailed")
                .detail(std::io::Error::last_os_error()));
        }
    }
    Ok(())
}

/// 子窗口使用父坐标，顶层拥有窗口保留屏幕坐标，供几何锁保存锚点。
pub unsafe fn parent_rect(window: HWND) -> RECT {
    let mut rect = RECT::default();
    GetWindowRect(window, &mut rect);
    let parent = child_parent(window);
    if !parent.is_null() {
        MapWindowPoints(null_mut(), parent, &mut rect as *mut RECT as *mut POINT, 2);
    }
    rect
}
