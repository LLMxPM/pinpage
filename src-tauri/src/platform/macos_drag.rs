// 功能：为 macOS 无边框网页窗口提供 Option + 左键原生拖动。
use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags};
use std::ptr::NonNull;
use tauri::{AppHandle, Manager};

/// 安装应用内鼠标事件监视器；命中可拖动窗口时交给系统拖动并吞掉该次点击，
/// 网页不会收到按下事件，也无需向页面注入脚本。必须在主线程调用。
pub fn start(app: &AppHandle) {
    let app = app.clone();
    let handler: RcBlock<dyn Fn(NonNull<NSEvent>) -> *mut NSEvent> =
        RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
            let event_ref = unsafe { event.as_ref() };
            if should_drag(&app, event_ref) {
                return std::ptr::null_mut();
            }
            event.as_ptr()
        });
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::LeftMouseDown, &handler)
    };
    // 监视器需与应用同寿命，主动泄漏以免被释放后失效。
    std::mem::forget(monitor);
}

/// 仅在按住 Option、点中未锁定位置的无边框页面窗口时开始拖动。
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
    let target_ptr = objc2::rc::Retained::as_ptr(&target) as *mut std::ffi::c_void;
    for (label, window) in app.webview_windows() {
        if label == "main" || window.is_decorated().unwrap_or(true) {
            continue;
        }
        if window.ns_window().ok() != Some(target_ptr) {
            continue;
        }
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
            return false;
        }
        target.performWindowDragWithEvent(event);
        return true;
    }
    false
}
