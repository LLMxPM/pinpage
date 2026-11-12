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
    state.ensure_mutable()?;
    let previous = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))?;
    // 预设是一次组合提交：外观与几何一起应用，保存与回滚由 apply 统一处理。
    if let Some(preset) = action.strip_prefix("preset:") {
        return super::presets::apply(app, &previous, preset);
    }
    match action {
        "open" | "reopen" => {
            super::open_pin(app, &previous)?;
            if let Err(error) = state.save() {
                if !previous.enabled {
                    if let Some(pin) = state
                        .settings
                        .lock()
                        .unwrap()
                        .pins
                        .iter_mut()
                        .find(|p| p.id == id)
                    {
                        pin.enabled = false;
                    }
                    if let Some(window) = app.get_webview_window(id) {
                        let _ = window.destroy();
                    }
                }
                return Err(error);
            }
        }
        "close" | "delete" => {
            let before = state.settings.lock().unwrap().clone();
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
            if let Err(error) = state.save() {
                *state.settings.lock().unwrap() = before;
                return Err(error);
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
                    if let Err(rollback) = state.save() {
                        state::report(app, rollback);
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
                state
                    .settings
                    .lock()
                    .unwrap()
                    .automation_clients
                    .retain(|_, value| value["pageId"] != id);
                state.settings.lock().unwrap().site_commits.remove(id);
                state.save()?;
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
        "top" | "border" | "desktop" | "recover" | "frameSystem" | "frameBorderless"
        | "frameRounded" | "positionLock" | "sizeLock" => {
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
                // 边框样式三选一：桌面模式不写系统边框偏好，切回浮动时沿用原有选择。
                "frameSystem" | "frameBorderless" | "frameRounded" => {
                    if next.mode == WindowMode::Floating {
                        next.decorations = action == "frameSystem";
                    }
                    next.rounded = action == "frameRounded";
                }
                "positionLock" => next.position_locked = !next.position_locked,
                "sizeLock" => next.size_locked = !next.size_locked,
                "recover" => {
                    next.mode = WindowMode::Floating;
                    next.decorations = true;
                    next.transparent_background = false;
                    next.position_locked = false;
                }
                _ => unreachable!(),
            }
            // 单字段操作会偏离预设组合，标记退化为“自定义”而不是继续声称基于某个预设。
            next.preset = None;
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
                    if previous.mode == WindowMode::Floating {
                        super::geometry::ensure_on_screen(&window)?;
                    }
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
                "hide" => window.hide().map_err(window_failed)?,
                "reload" => {
                    crate::automation::begin_load(app, id);
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
    if matches!(action, "close" | "delete") {
        crate::automation::load_error(
            app,
            id,
            pinpage_protocol::Failure::new("pageClosed", "页面已关闭"),
        );
        crate::automation::diagnostics::close_page(app, id);
    }
    state::schedule_save(app);
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(())
}
