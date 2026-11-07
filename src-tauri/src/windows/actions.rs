// 功能：管理页面的开启、关闭、删除与常见原生窗口操作。
use crate::{
    errors::AppMessage,
    models::{PageSource, WindowMode},
    state::{self, AppState},
};
use tauri::{AppHandle, Manager};

/// Tauri 窗口 API 的失败统一归入窗口设置说明，原始诊断保留在详情。
fn window_failed(error: impl std::fmt::Display) -> AppMessage {
    AppMessage::new("errors.windowApplyFailed").detail(error)
}

/// 统一串行化生命周期与外观操作；关闭保留配置，删除才移除记录。
pub fn action(app: &AppHandle, id: &str, action: &str) -> Result<(), AppMessage> {
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    let previous = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))?;
    match action {
        "open" | "reopen" => {
            super::open_pin(app, &previous)?;
        }
        "close" | "delete" => {
            if let Some(pin) = state
                .settings
                .lock()
                .unwrap()
                .pins
                .iter_mut()
                .find(|pin| pin.id == id)
            {
                pin.enabled = false;
            }
            if let Some(window) = app.get_webview_window(id) {
                if let Err(error) = window.destroy() {
                    if let Some(pin) = state
                        .settings
                        .lock()
                        .unwrap()
                        .pins
                        .iter_mut()
                        .find(|pin| pin.id == id)
                    {
                        pin.enabled = previous.enabled;
                    }
                    return Err(AppMessage::new("errors.windowApplyFailed").detail(error));
                }
            }
            if action == "delete" {
                state
                    .settings
                    .lock()
                    .unwrap()
                    .pins
                    .retain(|pin| pin.id != id);
                state.runtime.lock().unwrap().remove(id);
                // 删除记录时一并清理导入的站点副本；外部引用来源没有副本可删。
                if matches!(&previous.source, PageSource::Bundled { .. }) {
                    if let Err(error) = crate::import::remove_page(app, id) {
                        state::report(app, error);
                    }
                }
            } else if let Some(runtime) = state.runtime.lock().unwrap().get_mut(id) {
                runtime.open = false;
                runtime.visible = false;
                runtime.loading = false;
                runtime.error = None;
            }
        }
        "top" | "border" | "desktop" | "recover" => {
            let mut next = previous.clone();
            match action {
                "top" => {
                    if next.mode == WindowMode::Desktop {
                        return Err(AppMessage::new("errors.desktopTopMode"));
                    }
                    next.always_on_top = !next.always_on_top;
                }
                "border" => {
                    next.decorations = !next.decorations;
                }
                "desktop" => {
                    next.mode = if next.mode == WindowMode::Desktop {
                        WindowMode::Floating
                    } else {
                        WindowMode::Desktop
                    };
                }
                "recover" => {
                    next.mode = WindowMode::Floating;
                    next.decorations = true;
                    next.position_locked = false;
                }
                _ => unreachable!(),
            }
            super::settings::apply(app, &previous, next)?;
            if action == "recover" {
                if let Some(window) = app.get_webview_window(id) {
                    super::geometry::recover(&window)?;
                    super::events::capture(&window);
                }
            }
        }
        _ => {
            let window = app
                .get_webview_window(id)
                .ok_or_else(|| AppMessage::new("errors.pageClosed"))?;
            match action {
                "focus" => {
                    window
                        .unminimize()
                        .and_then(|_| window.show())
                        .map_err(window_failed)?;
                    if previous.mode == WindowMode::Floating {
                        window.set_focus().map_err(window_failed)?;
                    }
                }
                "visibility" => {
                    if window.is_visible().map_err(window_failed)? {
                        window.hide()
                    } else {
                        window.show()
                    }
                    .map_err(window_failed)?;
                }
                "minimize" => window.minimize().map_err(window_failed)?,
                "reload" => {
                    window.reload().map_err(window_failed)?;
                    super::events::watch_load(app, id);
                }
                _ => return Err(AppMessage::new("errors.unknownAction")),
            }
            if let Some(runtime) = state.runtime.lock().unwrap().get_mut(id) {
                runtime.visible = window.is_visible().unwrap_or(true);
            }
        }
    }
    state::schedule_save(app);
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(())
}
