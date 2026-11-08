// 功能：构建系统托盘及各悬浮窗的原生控制菜单。
use crate::{errors::AppMessage, i18n, models::WindowMode, state, state::AppState, windows};
use tauri::{
    menu::{Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

/// 托盘菜单项消息键，顺序即菜单顺序。
const PIN_ACTIONS: [(&str, &str); 10] = [
    ("focus", "tray.focus"),
    ("open", "tray.open"),
    ("close", "tray.close"),
    ("visibility", "tray.visibility"),
    ("top", "tray.top"),
    ("border", "tray.border"),
    ("desktop", "tray.desktop"),
    ("reload", "tray.reload"),
    ("recover", "tray.recover"),
    ("delete", "tray.delete"),
];

/// 依据当前窗口列表创建菜单，失效会话记录同样提供重新打开和删除入口。
fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let locale = state::locale(app);
    let entry = |key: &str| i18n::text(locale, key, &Default::default());
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "manager",
        entry("tray.openManager"),
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "recover",
        entry("tray.recoverAll"),
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
        for (action, key) in PIN_ACTIONS {
            // 桌面模式是 Explorer 的子窗口，无法抬到普通窗口前面。
            if action == "focus" && pin.mode == WindowMode::Desktop {
                continue;
            }
            submenu.append(&MenuItem::with_id(
                app,
                format!("pin|{}|{action}", pin.id),
                entry(key),
                true,
                None::<&str>,
            )?)?;
        }
        menu.append(&submenu)?;
    }
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        entry("tray.quit"),
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

/// 创建常驻托盘；左键打开管理界面，右键显示原生菜单。
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let icon = app.default_window_icon().expect("应用图标缺失").clone();
    let tooltip = i18n::text(state::locale(app), "tray.tooltip", &Default::default());
    TrayIconBuilder::with_id("pinpage")
        .icon(icon)
        .tooltip(tooltip)
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

/// 在窗口列表或语言变化后重建托盘菜单和提示文字，失败时保留原有可操作菜单并返回原因。
pub fn refresh(app: &AppHandle) -> Result<(), AppMessage> {
    let Some(tray) = app.tray_by_id("pinpage") else {
        return Ok(());
    };
    let failed = |error: tauri::Error| AppMessage::new("errors.trayRefreshFailed").detail(error);
    let menu = menu(app).map_err(failed)?;
    tray.set_menu(Some(menu)).map_err(failed)?;
    let tooltip = i18n::text(state::locale(app), "tray.tooltip", &Default::default());
    tray.set_tooltip(Some(&tooltip)).map_err(failed)
}

/// 常规路径的托盘刷新：失败记录为可见通知，菜单保持可用。
pub fn refresh_or_report(app: &AppHandle) {
    if let Err(error) = refresh(app) {
        state::report(app, error);
    }
}
