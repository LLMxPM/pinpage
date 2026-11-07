// 功能：定位 Windows 桌面图标宿主，将网页窗口嵌入或移出桌面层。
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
pub fn host() -> Result<HWND, String> {
    let mut result: HWND = null_mut();
    unsafe {
        EnumWindows(Some(find_host), &mut result as *mut HWND as LPARAM);
    }
    if result.is_null() {
        Err("当前桌面暂不可用，请稍后重试桌面模式".into())
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
pub fn set_parent(window: HWND, parent: HWND) -> Result<(), String> {
    unsafe {
        let old_parent = GetParent(window);
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
            return Err(format!("无法切换桌面模式：{error}"));
        }
        if parent.is_null() {
            SetWindowLongPtrW(window, GWL_STYLE, next_style);
        }
    }
    Ok(())
}

/// 将屏幕物理坐标转换为当前父窗口的客户区坐标，再更新位置和层级。
pub fn place(window: HWND, x: i32, y: i32) -> Result<(), String> {
    unsafe {
        let parent = GetParent(window);
        let mut point = POINT { x, y };
        if !parent.is_null() {
            MapWindowPoints(null_mut(), parent, &mut point, 1);
        }
        let after = if parent.is_null() {
            null_mut()
        } else {
            HWND_TOP
        };
        let flags = SWP_NOSIZE
            | SWP_NOACTIVATE
            | SWP_FRAMECHANGED
            | if parent.is_null() { SWP_NOZORDER } else { 0 };
        if SetWindowPos(window, after, point.x, point.y, 0, 0, flags) == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}

/// 获取父坐标系中的窗口矩形，供原生几何锁保存锚点。
pub unsafe fn parent_rect(window: HWND) -> RECT {
    let mut rect = RECT::default();
    GetWindowRect(window, &mut rect);
    let parent = GetParent(window);
    if !parent.is_null() {
        MapWindowPoints(null_mut(), parent, &mut rect as *mut RECT as *mut POINT, 2);
    }
    rect
}
