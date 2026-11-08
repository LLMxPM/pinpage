// 功能：为内容窗口构建带状态勾选的原生快捷菜单，复用托盘的操作分发与权限边界。
use crate::{
    errors::AppMessage,
    i18n,
    models::WindowMode,
    state::{self, AppState},
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    AppHandle, Manager,
};

/// 按弹出时的最新配置生成菜单；不包含删除操作，关闭只释放 WebView 并保留配置。
#[cfg(windows)]
pub fn show(app: &AppHandle, request: crate::platform::MenuRequest) -> Result<(), AppMessage> {
    if !request.ready() {
        return Ok(());
    }
    let id = &request.id;
    let hwnd = request.hwnd;
    let Some(window) = app.get_webview_window(id) else {
        return Ok(());
    };
    if window.hwnd().map_err(crate::errors::unknown)?.0 as usize != hwnd {
        return Ok(());
    }
    let Some(pin) = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == *id)
        .cloned()
    else {
        return Ok(());
    };
    let locale = state::locale(app);
    let text = |key: &str| i18n::text(locale, key, &Default::default());
    let failed = |error: tauri::Error| AppMessage::new("errors.windowMenuFailed").detail(error);
    let menu = Menu::new(app).map_err(failed)?;
    let desktop = pin.mode == WindowMode::Desktop;
    for (action, key, enabled, checked) in [
        (
            "top",
            "window.menu.top",
            !desktop,
            !desktop && pin.always_on_top,
        ),
        ("desktop", "window.menu.desktop", true, desktop),
        (
            "border",
            "window.menu.border",
            !desktop,
            pin.effective_decorations(),
        ),
        (
            "rounded",
            "window.menu.rounded",
            !pin.effective_decorations(),
            pin.rounded,
        ),
        (
            "positionLock",
            "window.menu.positionLock",
            true,
            pin.position_locked,
        ),
        ("sizeLock", "window.menu.sizeLock", true, pin.size_locked),
    ] {
        menu.append(
            &CheckMenuItem::with_id(
                app,
                format!("pin|{id}|{action}"),
                text(key),
                enabled,
                checked,
                None::<&str>,
            )
            .map_err(failed)?,
        )
        .map_err(failed)?;
    }
    menu.append(&PredefinedMenuItem::separator(app).map_err(failed)?)
        .map_err(failed)?;
    for (menu_id, key) in [
        (format!("pin|{id}|reload"), "tray.reload"),
        (format!("pin|{id}|hide"), "window.menu.hide"),
        ("manager".into(), "tray.openManager"),
        (format!("pin|{id}|close"), "tray.close"),
    ] {
        menu.append(
            &MenuItem::with_id(app, menu_id, text(key), true, None::<&str>).map_err(failed)?,
        )
        .map_err(failed)?;
    }
    // 原生菜单 API 使用客户区坐标；到 UI 线程再换算，兼容 DPI、负坐标和桌面父窗口。
    let app = app.clone();
    let ui_window = window.clone();
    window
        .run_on_main_thread(move || {
            use windows_sys::Win32::{
                Foundation::POINT,
                Graphics::Gdi::ScreenToClient,
                UI::WindowsAndMessaging::{IsWindow, IsWindowVisible, PostMessageW, WM_NULL},
            };
            if !request.ready() {
                return;
            }
            let handle = hwnd as windows_sys::Win32::Foundation::HWND;
            let mut point = POINT {
                x: request.x,
                y: request.y,
            };
            if unsafe {
                IsWindow(handle) == 0
                    || IsWindowVisible(handle) == 0
                    || ScreenToClient(handle, &mut point) == 0
            } {
                return;
            }
            let result =
                ui_window.popup_menu_at(&menu, tauri::PhysicalPosition::new(point.x, point.y));
            // 前台窗口第二次弹菜单时，系统可能让菜单出现后立即消失；按 TrackPopupMenu
            // 文档在菜单返回后投递 WM_NULL，强制完成一次任务切换。
            unsafe {
                if IsWindow(handle) != 0 {
                    PostMessageW(handle, WM_NULL, 0, 0);
                }
            }
            if let Err(error) = result {
                state::report(
                    &app,
                    AppMessage::new("errors.windowMenuFailed").detail(error),
                );
            }
        })
        .map_err(failed)
}
