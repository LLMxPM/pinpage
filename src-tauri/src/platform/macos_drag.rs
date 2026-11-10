// 功能：在实验构建中为 macOS 无边框页面提供 Option 拖动，并管理主线程事件监视器。
// 基于 loikein 的 PR #1：https://github.com/LLMxPM/pinpage/pull/1。
use block2::RcBlock;
use objc2::{rc::Retained, runtime::AnyObject, MainThreadMarker};
use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags};
use std::{cell::RefCell, ptr::NonNull};
use tauri::{AppHandle, Manager};

thread_local! {
    // AppKit 监视器只在主线程访问，退出时显式注销，避免遗留持有 AppHandle 的闭包。
    static MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

/// 从应用 setup 主线程安装一次本地监视器；只吞掉真正触发拖动的左键事件。
pub fn start(app: &AppHandle) {
    if MainThreadMarker::new().is_none() {
        crate::state::report(
            app,
            super::AppMessage::new("errors.windowApplyFailed")
                .detail("macOS 拖动监视器必须在主线程安装"),
        );
        return;
    }
    if MONITOR.with(|monitor| monitor.borrow().is_some()) {
        return;
    }
    let event_app = app.clone();
    let handler: RcBlock<dyn Fn(NonNull<NSEvent>) -> *mut NSEvent> =
        RcBlock::new(move |event: NonNull<NSEvent>| {
            // AppKit 在回调期间保证事件有效；返回原事件或空指针符合监视器契约。
            if should_drag(&event_app, unsafe { event.as_ref() }) {
                std::ptr::null_mut()
            } else {
                event.as_ptr()
            }
        });
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::LeftMouseDown, &handler)
    };
    if monitor.is_none() {
        crate::state::report(
            app,
            super::AppMessage::new("errors.windowApplyFailed")
                .detail("无法安装 macOS 原生拖动监视器"),
        );
    }
    MONITOR.with(|stored| *stored.borrow_mut() = monitor);
}

/// 从应用退出事件的主线程注销自身创建的监视器；重复调用无副作用。
pub fn stop() {
    if MainThreadMarker::new().is_none() {
        return;
    }
    MONITOR.with(|stored| {
        if let Some(monitor) = stored.borrow_mut().take() {
            // 句柄只来自 addLocalMonitor，且一直保留到注销结束。
            unsafe { NSEvent::removeMonitor(&monitor) };
        }
    });
}

/// 仅让已登记、未锁定位置的无边框浮动页面响应 Option + 左键。
fn should_drag(app: &AppHandle, event: &NSEvent) -> bool {
    let flags = event.modifierFlags() & NSEventModifierFlags::DeviceIndependentFlagsMask;
    if flags != NSEventModifierFlags::Option {
        return false;
    }
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let Some(target) = event.window(mtm) else {
        return false;
    };
    let target_ptr = Retained::as_ptr(&target) as *mut std::ffi::c_void;
    for (label, window) in app.webview_windows() {
        if label == "main" || window.ns_window().ok() != Some(target_ptr) {
            continue;
        }
        let can_drag = app
            .state::<crate::state::AppState>()
            .settings
            .lock()
            .unwrap()
            .pins
            .iter()
            .find(|pin| pin.id == label)
            .is_some_and(|pin| {
                pin.enabled
                    && pin.mode == crate::models::WindowMode::Floating
                    && !pin.decorations
                    && !pin.position_locked
            });
        if !can_drag || window.is_decorated().unwrap_or(true) {
            return false;
        }
        // 原生拖动会进入事件循环，必须先释放 settings 锁，允许位置回传正常保存。
        target.performWindowDragWithEvent(event);
        return true;
    }
    false
}
