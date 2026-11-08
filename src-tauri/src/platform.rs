// 功能：提供 Windows 窗口策略、统一修饰键鼠标手势及桌面恢复入口。
use crate::{errors::AppMessage, models::GestureModifier};
use tauri::AppHandle;
#[cfg(windows)]
mod caption;
#[cfg(windows)]
mod corners;
#[cfg(windows)]
mod desktop;
#[cfg(windows)]
mod gestures;
#[cfg(windows)]
pub(crate) use gestures::MenuRequest;
pub mod locale;
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

/// 其他平台暂不提供桌面嵌入和原生锁定。
#[cfg(not(windows))]
pub fn apply(
    _window: &tauri::WebviewWindow,
    _pin: &crate::models::PinConfig,
) -> Result<(), AppMessage> {
    Err(AppMessage::new("errors.platformUnsupported"))
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

/// 启动桌面恢复与唯一鼠标输入线程，不再轮询 Alt + 左键。
#[cfg(windows)]
pub fn start(app: &AppHandle) {
    recovery::start(app);
    gestures::start(app);
}

/// 其他平台保留标准边框移动和托盘恢复入口。
#[cfg(not(windows))]
pub fn start(_app: &AppHandle) {}

/// 全局设置持久化成功后更新两种手势共用的修饰键。
pub fn set_gesture_modifier(modifier: GestureModifier) {
    #[cfg(windows)]
    gestures::set_modifier(modifier);
    #[cfg(not(windows))]
    let _ = modifier;
}

/// 应用退出时释放原生输入钩子。
pub fn stop() {
    #[cfg(windows)]
    gestures::stop();
}
