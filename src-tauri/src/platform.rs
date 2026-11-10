// 功能：声明各平台窗口能力、执行统一校验，并提供 Windows 原生策略与安全回退。
#[cfg(windows)]
use crate::models::PlatformDragModifier;
use crate::{
    errors::AppMessage,
    models::{GestureModifier, PinConfig, PlatformCapabilities, PresetFields, WindowMode},
};
use tauri::AppHandle;
#[cfg(windows)]
mod caption;
#[cfg(windows)]
mod corners;
#[cfg(windows)]
mod desktop;
#[cfg(windows)]
mod gestures;
#[cfg(all(target_os = "macos", feature = "experimental-macos-drag"))]
mod macos_drag;
#[cfg(windows)]
pub(crate) use gestures::MenuRequest;
pub mod locale;
#[cfg(windows)]
mod recovery;
#[cfg(windows)]
mod rules;
#[cfg(windows)]
mod window_policy;

/// Windows 完整支持桌面嵌入、圆角、独立几何锁与可配置手势。
#[cfg(windows)]
pub fn capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        desktop_mode: true,
        rounded: true,
        position_lock: true,
        size_lock: true,
        window_gestures: true,
        drag_modifier: Some(PlatformDragModifier::Alt),
    }
}

/// macOS 默认只提供标准浮动窗口，显式实验构建才声明固定 Option 拖动。
#[cfg(target_os = "macos")]
pub fn capabilities() -> PlatformCapabilities {
    let mut capabilities = floating_capabilities();
    if cfg!(feature = "experimental-macos-drag") {
        capabilities.drag_modifier = Some(crate::models::PlatformDragModifier::Option);
    }
    capabilities
}

/// 其他平台保持最保守的浮动窗口组合，不继承 macOS 的实验能力。
#[cfg(not(any(windows, target_os = "macos")))]
pub fn capabilities() -> PlatformCapabilities {
    floating_capabilities()
}

/// 标准浮动窗口的共同基线；所有未实现的原生能力明确关闭。
#[cfg(not(windows))]
fn floating_capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        desktop_mode: false,
        rounded: false,
        position_lock: false,
        size_lock: false,
        window_gestures: false,
        drag_modifier: None,
    }
}

/// 页面是否只使用当前平台支持的能力。
fn supports_pin(capabilities: PlatformCapabilities, pin: &PinConfig) -> bool {
    (pin.mode == WindowMode::Floating || capabilities.desktop_mode)
        && (!pin.rounded || capabilities.rounded)
        && (!pin.position_locked || capabilities.position_lock)
        && (!pin.size_locked || capabilities.size_lock)
}

/// 预设是否能在当前平台完整应用，避免通过菜单或 IPC 绕过前端裁剪。
fn supports_preset(capabilities: PlatformCapabilities, fields: &PresetFields) -> bool {
    (fields.mode == WindowMode::Floating || capabilities.desktop_mode)
        && (!fields.rounded || capabilities.rounded)
        && (!fields.position_locked || capabilities.position_lock)
        && (!fields.size_locked || capabilities.size_lock)
}

/// 显式提交不支持的能力时拒绝，调用方保留原窗口与配置。
pub fn validate_pin(capabilities: PlatformCapabilities, pin: &PinConfig) -> Result<(), AppMessage> {
    if supports_pin(capabilities, pin) {
        Ok(())
    } else {
        Err(AppMessage::new("errors.platformUnsupported"))
    }
}

/// 预设写入与应用前使用同一套断言，内置项与自定义项不区分处理。
pub fn validate_preset(
    capabilities: PlatformCapabilities,
    fields: &PresetFields,
) -> Result<(), AppMessage> {
    if supports_preset(capabilities, fields) {
        Ok(())
    } else {
        Err(AppMessage::new("errors.platformUnsupported"))
    }
}

/// 把从其他平台带来的页面降级为标准浮动窗口，保证内容仍可访问和找回。
pub fn fallback_pin(capabilities: PlatformCapabilities, pin: &mut PinConfig) -> bool {
    let before = (
        pin.mode,
        pin.decorations,
        pin.rounded,
        pin.position_locked,
        pin.size_locked,
    );
    if pin.mode == WindowMode::Desktop && !capabilities.desktop_mode {
        // 桌面模式本身依赖 Windows 专属宿主；回到浮动层时恢复系统边框作为可靠找回入口。
        pin.mode = WindowMode::Floating;
        pin.decorations = true;
        pin.rounded = false;
        pin.position_locked = false;
        pin.size_locked = false;
    }
    if !capabilities.rounded {
        pin.rounded = false;
    }
    if !capabilities.position_lock {
        pin.position_locked = false;
    }
    if !capabilities.size_lock {
        pin.size_locked = false;
    }
    let changed = before
        != (
            pin.mode,
            pin.decorations,
            pin.rounded,
            pin.position_locked,
            pin.size_locked,
        );
    if changed {
        // 回退后的组合通常不再匹配原预设，清除标记避免界面继续展示失效来源。
        pin.preset = None;
    }
    changed
}

/// 应用单个页面的原生配置；所有调用都在此做平台能力终检。
pub fn apply(window: &tauri::WebviewWindow, pin: &PinConfig) -> Result<(), AppMessage> {
    validate_pin(capabilities(), pin)?;
    apply_supported(window, pin)
}

/// Windows 使用完整窗口策略，包括桌面宿主、圆角与几何锁。
#[cfg(windows)]
fn apply_supported(window: &tauri::WebviewWindow, pin: &PinConfig) -> Result<(), AppMessage> {
    window_policy::apply(window, pin)
}

/// 其他平台只应用 Tauri 通用的浮动窗口属性，不支持项已在上层校验。
#[cfg(not(windows))]
fn apply_supported(window: &tauri::WebviewWindow, pin: &PinConfig) -> Result<(), AppMessage> {
    use tauri::{LogicalSize, PhysicalPosition};
    let decorated = pin.effective_decorations();
    window
        .set_decorations(decorated)
        .and_then(|_| window.set_resizable(!pin.size_locked))
        .and_then(|_| {
            window.set_always_on_top(pin.mode == WindowMode::Floating && pin.always_on_top)
        })
        .and_then(|_| window.set_size(LogicalSize::new(pin.width, pin.height)))
        .and_then(|_| match (pin.x, pin.y) {
            (Some(x), Some(y)) => window.set_position(PhysicalPosition::new(x, y)),
            _ => Ok(()),
        })
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

/// 启动桌面恢复与唯一鼠标输入线程，不再轮询 Alt + 左键。
#[cfg(windows)]
pub fn start(app: &AppHandle) {
    recovery::start(app);
    gestures::start(app);
}

/// 其他平台保留标准边框移动和托盘恢复入口。
#[cfg(not(any(windows, all(target_os = "macos", feature = "experimental-macos-drag"))))]
pub fn start(_app: &AppHandle) {}

/// 实验构建在应用主线程安装 macOS 本地鼠标事件监视器。
#[cfg(all(target_os = "macos", feature = "experimental-macos-drag"))]
pub fn start(app: &AppHandle) {
    macos_drag::start(app);
}

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
    #[cfg(all(target_os = "macos", feature = "experimental-macos-drag"))]
    macos_drag::stop();
}

#[cfg(test)]
#[path = "platform/capabilities_tests.rs"]
mod capabilities_tests;
