// 功能：创建管理窗口与网页窗口，统一执行原生窗口操作。
mod actions;
mod events;
mod geometry;
mod settings;
pub use actions::action;
pub use settings::update;

use crate::{
    content,
    models::{PinConfig, PinRuntime},
    state::{self, AppState},
};
use std::sync::atomic::Ordering;
use tauri::{
    webview::NewWindowResponse, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

/// 只接受受信任管理窗口；同时验证当前来源，避免仅凭标签放行。
pub fn ensure_manager(window: &WebviewWindow) -> Result<(), String> {
    let url = window.url().map_err(|e| e.to_string())?;
    if window.label() != "main" || !manager_navigation(&url) {
        return Err("此页面无权管理应用".into());
    }
    Ok(())
}

/// 限制管理窗口为打包应用来源，开发时额外放行本项目开发服务器。
fn manager_navigation(url: &url::Url) -> bool {
    (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost"))
        || (cfg!(debug_assertions)
            && url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(1420))
}

/// 已有管理窗口时激活，否则从异步任务创建，关闭后不保留隐藏 WebView。
pub fn show_manager(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = WebviewWindowBuilder::new(&app, "main", WebviewUrl::App("index.html".into()))
            .title("PinPage · 网页悬浮工作台")
            .inner_size(1080., 800.)
            .min_inner_size(760., 620.)
            .center()
            .on_navigation(manager_navigation)
            .on_new_window(|_, _| NewWindowResponse::Deny)
            .build();
        if let Err(error) = result {
            if app.get_webview_window("main").is_none() {
                state::report(&app, format!("无法打开管理界面：{error}"));
            }
        }
    });
}

/// 创建一个内容 WebView，来源校验与资源授权在窗口创建前完成。
pub fn open_pin(app: &AppHandle, pin: &PinConfig) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&pin.id) {
        window.show().map_err(|e| e.to_string())?;
        if pin.mode == crate::models::WindowMode::Floating {
            window.set_focus().map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let url = content::source_url(&pin.id, &pin.source)?;
    let navigation_id = pin.id.clone();
    let navigation_source = pin.source.clone();
    let navigation_app = app.clone();
    let popup_app = app.clone();
    let state = app.state::<AppState>();
    state.runtime.lock().unwrap().insert(
        pin.id.clone(),
        PinRuntime {
            open: true,
            visible: true,
            loading: true,
            current_url: url.to_string(),
            error: None,
        },
    );
    let builder = WebviewWindowBuilder::new(app, &pin.id, WebviewUrl::External(url))
        .title(&pin.name)
        .inner_size(pin.width.clamp(160., 3840.), pin.height.clamp(100., 2160.))
        .min_inner_size(160., 100.)
        .resizable(!pin.size_locked)
        // 悬浮页通过托盘和管理界面访问，避免多个页面占用任务栏。
        .skip_taskbar(true)
        .always_on_top(pin.mode == crate::models::WindowMode::Floating && pin.always_on_top)
        .decorations(pin.effective_decorations())
        .shadow(pin.effective_decorations())
        .disable_drag_drop_handler()
        .on_navigation(move |url| {
            let allowed = content::navigation_allowed(&navigation_id, &navigation_source, url);
            if !allowed {
                events::set_error(
                    &navigation_app,
                    &navigation_id,
                    "已阻止不支持的导航协议或本地来源".into(),
                );
            }
            allowed
        })
        .on_new_window(move |url, _| {
            if matches!(url.scheme(), "http" | "https")
                && !url.host_str().unwrap_or("").starts_with("pinpage.")
            {
                if let Err(error) = popup_app.opener().open_url(url.as_str(), None::<&str>) {
                    state::report(&popup_app, format!("无法打开外部链接：{error}"));
                }
            }
            NewWindowResponse::Deny
        })
        .on_page_load(events::page_load)
        .on_document_title_changed(move |window, title| {
            // 网页标题只影响系统标题栏，用户设置的名称保持稳定。
            let _ = window.set_title(&format!(
                "{} · PinPage",
                title.chars().take(100).collect::<String>()
            ));
        });
    let window = match builder.build() {
        Ok(window) => window,
        Err(error) => {
            events::set_error(app, &pin.id, format!("创建窗口失败：{error}"));
            if let Some(runtime) = state.runtime.lock().unwrap().get_mut(&pin.id) {
                runtime.open = false;
            }
            return Err(error.to_string());
        }
    };
    geometry::restore(&window, pin)?;
    let mut positioned = pin.clone();
    let position = crate::platform::position(&window)?;
    positioned.x = Some(position.x);
    positioned.y = Some(position.y);
    if let Err(error) = crate::platform::apply(&window, &positioned) {
        let _ = window.destroy();
        events::set_error(app, &pin.id, error.clone());
        if let Some(runtime) = state.runtime.lock().unwrap().get_mut(&pin.id) {
            runtime.open = false;
        }
        return Err(error);
    }
    if let Some(saved) = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter_mut()
        .find(|p| p.id == pin.id)
    {
        saved.enabled = true;
    }
    events::attach(&window);
    events::watch_load(app, &pin.id);
    state::notify(app);
    Ok(())
}

/// 恢复全部窗口的边框与可见位置，作为无边框模式的可靠找回入口。
pub fn recover_all(app: &AppHandle) {
    let ids: Vec<_> = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .map(|p| p.id.clone())
        .collect();
    for id in ids {
        if app.get_webview_window(&id).is_some() {
            if let Err(error) = action(app, &id, "recover") {
                state::report(app, error);
            }
        }
    }
}

/// 退出前保存会话，并防止窗口销毁事件误删需要恢复的记录。
pub fn quit(app: &AppHandle) {
    if let Err(error) = app.state::<AppState>().save() {
        state::report(app, error);
        show_manager(app);
        return;
    }
    app.state::<AppState>()
        .exiting
        .store(true, Ordering::SeqCst);
    app.exit(0);
}
