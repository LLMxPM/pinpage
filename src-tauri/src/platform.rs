// 功能：为 Windows 无边框网页窗口提供 Alt + 左键原生拖动。
use tauri::AppHandle;
#[cfg(windows)]
mod corners;
#[cfg(windows)]
mod desktop;
#[cfg(windows)]
mod recovery;
#[cfg(windows)]
mod rules;
#[cfg(windows)]
mod window_policy;

/// 应用单个页面的原生模式、圆角与锁定策略。
#[cfg(windows)]
pub fn apply(window: &tauri::WebviewWindow, pin: &crate::models::PinConfig) -> Result<(), String> {
    window_policy::apply(window, pin)
}

/// 其他平台暂不提供桌面嵌入和原生锁定。
#[cfg(not(windows))]
pub fn apply(
    _window: &tauri::WebviewWindow,
    _pin: &crate::models::PinConfig,
) -> Result<(), String> {
    Err("此版本的窗口模式设置仅支持 Windows".into())
}

/// 原生读取屏幕物理坐标，避免桌面子窗口被保存成父窗口局部坐标。
#[cfg(windows)]
pub fn position(window: &tauri::WebviewWindow) -> Result<tauri::PhysicalPosition<i32>, String> {
    use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::GetWindowRect};
    let handle = window.hwnd().map_err(|e| e.to_string())?.0;
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(handle, &mut rect) } == 0 {
        return Err("无法读取窗口位置".into());
    }
    Ok(tauri::PhysicalPosition::new(rect.left, rect.top))
}

/// 非 Windows 平台使用 Tauri 提供的位置查询。
#[cfg(not(windows))]
pub fn position(window: &tauri::WebviewWindow) -> Result<tauri::PhysicalPosition<i32>, String> {
    window.outer_position().map_err(|e| e.to_string())
}

/// 只在组合键按下沿检查本应用前台窗口，无需向网页注入管理脚本。
#[cfg(windows)]
pub fn start(app: &AppHandle) {
    recovery::start(app);
    use tauri::Manager;
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        UI::{
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_MENU},
            WindowsAndMessaging::{
                GetCursorPos, GetForegroundWindow, GetWindowRect, IsChild, WindowFromPoint,
            },
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
                    let locked = app
                        .state::<crate::state::AppState>()
                        .settings
                        .lock()
                        .unwrap()
                        .pins
                        .iter()
                        .find(|pin| pin.id == label)
                        .is_some_and(|pin| pin.position_locked);
                    if locked {
                        continue;
                    }
                    if label == "main" || window.is_decorated().unwrap_or(true) {
                        continue;
                    }
                    if let Ok(handle) = window.hwnd() {
                        let under_pointer = unsafe { WindowFromPoint(point) };
                        if handle.0 == root
                            || handle.0 == under_pointer
                            || unsafe { IsChild(handle.0, under_pointer) } != 0
                        {
                            let mut rect = RECT::default();
                            unsafe {
                                GetWindowRect(handle.0, &mut rect);
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
