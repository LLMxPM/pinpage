// 功能：为 Windows 无边框网页窗口提供 Alt + 左键原生拖动。
use tauri::AppHandle;

/// 只在组合键按下沿检查本应用前台窗口，无需向网页注入管理脚本。
#[cfg(windows)]
pub fn start(app: &AppHandle) {
    use tauri::Manager;
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_MENU},
            WindowsAndMessaging::{GetCursorPos, GetForegroundWindow, GetWindowRect},
        },
    };
    let app = app.clone();
    std::thread::spawn(move || {
        let mut pressed = false;
        loop {
            if app
                .state::<crate::state::AppState>()
                .exiting
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                break;
            }
            let down = unsafe {
                GetAsyncKeyState(VK_MENU as i32) < 0 && GetAsyncKeyState(VK_LBUTTON as i32) < 0
            };
            if down && !pressed {
                let mut point = POINT { x: 0, y: 0 };
                unsafe {
                    GetCursorPos(&mut point);
                }
                let root = unsafe { GetForegroundWindow() };
                for (label, window) in app.webview_windows() {
                    if label == "main" || window.is_decorated().unwrap_or(true) {
                        continue;
                    }
                    if let Ok(handle) = window.hwnd() {
                        if handle.0 == root {
                            let mut rect = RECT::default();
                            unsafe {
                                GetWindowRect(root, &mut rect);
                            }
                            if point.x >= rect.left
                                && point.x < rect.right
                                && point.y >= rect.top
                                && point.y < rect.bottom
                            {
                                let _ = window.start_dragging();
                            }
                            break;
                        }
                    }
                }
            }
            pressed = down;
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    });
}

/// 其他平台保留标准边框移动和托盘恢复入口。
#[cfg(not(windows))]
pub fn start(_app: &AppHandle) {}
