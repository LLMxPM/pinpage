// 功能：构建系统托盘及各悬浮窗的原生控制菜单。
use crate::{
    state::{self, AppState},
    windows,
};
use tauri::{
    menu::{Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

/// 依据当前窗口列表创建菜单，失效会话记录同样提供重新打开和删除入口。
fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "manager",
        "打开管理界面",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "recover",
        "找回全部窗口（恢复边框）",
        true,
        None::<&str>,
    )?)?;
    let pins = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .pins
        .clone();
    for pin in pins {
        let submenu = Submenu::new(app, &pin.name, true)?;
        for (action, name) in [
            ("focus", "显示并激活"),
            ("reopen", "重新打开"),
            ("visibility", "显示 / 隐藏"),
            ("top", "切换置顶"),
            ("border", "切换边框"),
            ("reload", "刷新"),
            ("recover", "恢复边框并居中"),
            ("close", "关闭并删除"),
        ] {
            submenu.append(&MenuItem::with_id(
                app,
                format!("pin|{}|{action}", pin.id),
                name,
                true,
                None::<&str>,
            )?)?;
        }
        menu.append(&submenu)?;
    }
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "退出 PinPage",
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

/// 创建常驻托盘；左键打开管理界面，右键显示原生菜单。
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let icon = app.default_window_icon().expect("应用图标缺失").clone();
    TrayIconBuilder::with_id("pinpage")
        .icon(icon)
        .tooltip("PinPage · 网页悬浮工具")
        .menu(&menu(app)?)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                windows::show_manager(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref().to_string();
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                match id.as_str() {
                    "manager" => windows::show_manager(&app),
                    "recover" => windows::recover_all(&app),
                    "quit" => windows::quit(&app),
                    _ => {
                        let parts: Vec<_> = id.split('|').collect();
                        if parts.len() == 3 && parts[0] == "pin" {
                            if let Err(error) = windows::action(&app, parts[1], parts[2]) {
                                state::report(&app, error);
                                windows::show_manager(&app);
                            }
                        }
                    }
                }
            });
        })
        .build(app)?;
    Ok(())
}

/// 在窗口列表或状态变化后更新托盘菜单，失败保留现有菜单并记录。
pub fn refresh(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("pinpage") {
        if let Err(error) = menu(app).and_then(|menu| tray.set_menu(Some(menu))) {
            state::report(app, format!("托盘菜单更新失败：{error}"));
        }
    }
}
