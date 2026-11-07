// 功能：为 Windows 无边框网页窗口提供 Alt + 左键原生拖动。
use crate::errors::AppMessage;
use tauri::AppHandle;
#[cfg(windows)]
mod caption;
#[cfg(windows)]
mod corners;
#[cfg(windows)]
mod desktop;
pub mod locale;
#[cfg(target_os = "macos")]
mod macos_drag;
#[cfg(windows)]
mod recovery;
#[cfg(windows)]
mod rules;
#[cfg(windows)]
mod window_policy;

/// 应用单个页面的原生模式、圆角与锁定策略。
#[cfg(windows)]
pub fn apply(
    window: &tauri::WebviewWindow,
    pin: &crate::models::PinConfig,
) -> Result<(), AppMessage> {
    window_policy::apply(window, pin)
}

/// 其他平台仅支持浮动模式，用 Tauri 跨平台接口应用边框、置顶与尺寸锁；
/// 桌面嵌入、圆角与位置锁依赖 Win32，暂不提供。
#[cfg(not(windows))]
pub fn apply(
    window: &tauri::WebviewWindow,
    pin: &crate::models::PinConfig,
) -> Result<(), AppMessage> {
    use crate::models::WindowMode;
    if pin.mode == WindowMode::Desktop {
        return Err(AppMessage::new("errors.platformUnsupported"));
    }
    let decorated = pin.effective_decorations();
    window
        .set_decorations(decorated)
        .and_then(|_| window.set_shadow(decorated))
        .and_then(|_| window.set_resizable(!pin.size_locked))
        .and_then(|_| window.set_maximizable(!pin.size_locked && !pin.position_locked))
        .and_then(|_| window.set_always_on_top(pin.always_on_top))
        .and_then(|_| window.set_size(tauri::LogicalSize::new(pin.width, pin.height)))
        .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))
}

/// 原生读取屏幕物理坐标，避免桌面子窗口被保存成父窗口局部坐标。
#[cfg(windows)]
pub fn position(window: &tauri::WebviewWindow) -> Result<tauri::PhysicalPosition<i32>, AppMessage> {
    use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::GetWindowRect};
    let handle = window
        .hwnd()
        .map_err(|error| AppMessage::new("errors.readPosition").detail(error))?
        .0;
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(handle, &mut rect) } == 0 {
        return Err(AppMessage::new("errors.readPosition"));
    }
    Ok(tauri::PhysicalPosition::new(rect.left, rect.top))
}

/// 非 Windows 平台使用 Tauri 提供的位置查询。
#[cfg(not(windows))]
pub fn position(window: &tauri::WebviewWindow) -> Result<tauri::PhysicalPosition<i32>, AppMessage> {
    window
        .outer_position()
        .map_err(|error| AppMessage::new("errors.readPosition").detail(error))
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

/// macOS 使用 Option + 左键拖动无边框窗口。
#[cfg(target_os = "macos")]
pub fn start(app: &AppHandle) {
    macos_drag::start(app);
}

/// 其他平台保留标准边框移动和托盘恢复入口。
#[cfg(not(any(windows, target_os = "macos")))]
pub fn start(_app: &AppHandle) {}
