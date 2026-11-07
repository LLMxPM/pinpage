// 功能：在 UI 线程统一应用桌面模式、边框、几何锁和圆角，避免跨线程子类化。
use super::{corners, desktop, rules};
use crate::{
    errors::{self, AppMessage},
    models::{PinConfig, WindowMode},
};
use tauri::{LogicalSize, WebviewWindow};

/// 在一次原生配置事务内应用设置；期间放行几何变化，完成后冻结新位置及尺寸。
pub fn apply(window: &WebviewWindow, pin: &PinConfig) -> Result<(), AppMessage> {
    let window_for_ui = window.clone();
    let pin = pin.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    window
        .run_on_main_thread(move || {
            let _ = sender.send(apply_on_ui(&window_for_ui, &pin));
        })
        .map_err(errors::unknown)?;
    receiver
        .recv()
        .map_err(|_| AppMessage::new("errors.taskInterrupted"))?
}

/// 所有对窗口样式、父窗口和消息规则的修改均发生在所属主线程。
fn apply_on_ui(window: &WebviewWindow, pin: &PinConfig) -> Result<(), AppMessage> {
    let hwnd = window.hwnd().map_err(errors::unknown)?.0;
    let desired_parent = if pin.mode == WindowMode::Desktop {
        desktop::host()?
    } else {
        std::ptr::null_mut()
    };
    let rules = unsafe {
        &*rules::install(hwnd)
            .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))?
    };
    rules.configuring.set(true);
    let result = (|| {
        // 切回浮动层后再恢复 Tauri 的顶层窗口样式。
        if desired_parent.is_null() {
            desktop::set_parent(hwnd, desired_parent)?;
        }
        let decorated = pin.effective_decorations();
        window
            .set_decorations(decorated)
            .and_then(|_| window.set_shadow(decorated))
            .and_then(|_| window.set_resizable(!pin.size_locked))
            .and_then(|_| window.set_maximizable(!pin.size_locked && !pin.position_locked))
            .and_then(|_| {
                window.set_always_on_top(pin.mode == WindowMode::Floating && pin.always_on_top)
            })
            .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))?;
        if !desired_parent.is_null() {
            desktop::set_parent(hwnd, desired_parent)?;
        }
        window
            .set_size(LogicalSize::new(pin.width, pin.height))
            .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))?;
        let position = super::position(window)?;
        desktop::place(
            hwnd,
            pin.x.unwrap_or(position.x),
            pin.y.unwrap_or(position.y),
        )?;
        let scale = window
            .scale_factor()
            .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))?;
        rules.position_locked.set(pin.position_locked);
        rules.size_locked.set(pin.size_locked);
        rules.rounded.set(pin.rounded);
        rules.decorated.set(decorated);
        rules.scale.set(scale);
        corners::apply(hwnd, pin.rounded, decorated, scale)?;
        rules.anchor.set(unsafe { desktop::parent_rect(hwnd) });
        Ok(())
    })();
    rules.configuring.set(false);
    result
}
