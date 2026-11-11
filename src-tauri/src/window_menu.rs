// 功能：为内容窗口构建预设平铺的原生快捷菜单，提供页面设置直达入口，复用托盘的操作分发与权限边界。
use crate::{
    errors::AppMessage,
    i18n,
    state::{self, AppState},
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    AppHandle, Manager,
};

/// 按弹出时的最新配置生成菜单；预设平铺单选，外观与锁定等细节归口页面设置弹窗。
#[cfg(windows)]
pub fn show(app: &AppHandle, request: crate::platform::MenuRequest) -> Result<(), AppMessage> {
    if !request.valid() {
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
    let custom = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .window_presets
        .clone();
    let locale = state::locale(app);
    let text = |key: &str| i18n::text(locale, key, &Default::default());
    let failed = |error: tauri::Error| AppMessage::new("errors.windowMenuFailed").detail(error);
    let menu = Menu::new(app).map_err(failed)?;
    // 预设是切换重点：平铺单选勾选，切换走外观与几何的组合应用。
    let presets = crate::windows::presets::all(&custom);
    let matched = presets
        .iter()
        .any(|preset| pin.preset.as_deref() == Some(preset.id.as_str()));
    for preset in &presets {
        menu.append(
            &CheckMenuItem::with_id(
                app,
                format!("pin|{id}|preset:{}", preset.id),
                preset.name.clone(),
                true,
                pin.preset.as_deref() == Some(preset.id.as_str()),
                None::<&str>,
            )
            .map_err(failed)?,
        )
        .map_err(failed)?;
    }
    // 无匹配预设时以禁用行标明当前状态；尚无预设时引导到页面设置保存。
    if !matched {
        let key = if presets.is_empty() {
            "window.menu.noPresets"
        } else {
            "window.menu.custom"
        };
        menu.append(
            &MenuItem::with_id(
                app,
                format!("pin|{id}|presetStatus"),
                text(key),
                false,
                None::<&str>,
            )
            .map_err(failed)?,
        )
        .map_err(failed)?;
    }
    menu.append(&PredefinedMenuItem::separator(app).map_err(failed)?)
        .map_err(failed)?;
    // 页面设置直达管理界面对应弹窗；底部只保留设置弹窗没有的生命周期操作。
    for (menu_id, key) in [
        (format!("pin|{id}|settings"), "window.menu.settings"),
        (format!("pin|{id}|reload"), "tray.reload"),
        (format!("pin|{id}|hide"), "window.menu.hide"),
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
            if !request.valid() {
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
