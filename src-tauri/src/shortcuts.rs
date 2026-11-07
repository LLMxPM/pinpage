// 功能：注册可配置的全局控制快捷键，并在冲突时回滚。
use crate::{errors::AppMessage, state::AppState, windows};
use std::str::FromStr;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// 解析两组快捷键，要求不重复且至少包含一个组合修饰键。
fn parse(manager: &str, recover: &str) -> Result<[Shortcut; 2], AppMessage> {
    let manager = Shortcut::from_str(manager)
        .map_err(|error| AppMessage::new("errors.managerShortcutInvalid").detail(error))?;
    let recover = Shortcut::from_str(recover)
        .map_err(|error| AppMessage::new("errors.recoverShortcutInvalid").detail(error))?;
    if manager == recover {
        return Err(AppMessage::new("errors.shortcutDuplicate"));
    }
    if manager.mods.is_empty() || recover.mods.is_empty() {
        return Err(AppMessage::new("errors.shortcutModifier"));
    }
    Ok([manager, recover])
}

/// 替换已注册快捷键；注册新组合失败时恢复原有组合。
pub fn configure(app: &AppHandle, manager: &str, recover: &str) -> Result<(), AppMessage> {
    let next = parse(manager, recover)?;
    let old = app.state::<AppState>().settings.lock().unwrap().clone();
    let previous = parse(&old.manager_shortcut, &old.recover_shortcut).ok();
    app.global_shortcut()
        .unregister_all()
        .map_err(|error| AppMessage::new("errors.shortcutTaken").detail(error))?;
    if let Err(error) = app.global_shortcut().register_multiple(next) {
        let _ = app.global_shortcut().unregister_all();
        if let Some(previous) = previous {
            let _ = app.global_shortcut().register_multiple(previous);
        }
        return Err(AppMessage::new("errors.shortcutTaken").detail(error));
    }
    Ok(())
}

/// 构造插件事件处理器，只响应按下事件，原生窗口操作交给异步任务。
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let settings = app.state::<AppState>().settings.lock().unwrap().clone();
            let is_manager =
                Shortcut::from_str(&settings.manager_shortcut).is_ok_and(|key| key == *shortcut);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if is_manager {
                    windows::show_manager(&app);
                } else {
                    windows::recover_all(&app);
                }
            });
        })
        .build()
}
