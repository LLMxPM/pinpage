// 功能：创建管理窗口与网页窗口，统一执行原生窗口操作。
mod actions;
pub(crate) mod editor;
mod events;
mod geometry;
pub(crate) mod limits;
pub(crate) mod presets;
mod settings;
pub use actions::action;
pub use settings::update;

use crate::{
    content,
    errors::AppMessage,
    i18n,
    models::{PageSource, PinConfig, PinRuntime},
    state::{self, AppState},
};
use std::{collections::BTreeMap, sync::atomic::Ordering};
use tauri::{
    webview::NewWindowResponse, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

/// 只接受受信任管理窗口；同时验证当前来源，避免仅凭标签放行。
pub fn ensure_manager(window: &WebviewWindow) -> Result<(), AppMessage> {
    let url = window
        .url()
        .map_err(|error| AppMessage::new("errors.windowNotAuthorized").detail(error))?;
    if window.label() != "main" || !manager_navigation(&url) {
        return Err(AppMessage::new("errors.notManager"));
    }
    Ok(())
}

/// 内容窗口与管理窗口的系统标题：用户名称或应用标题按当前语言加后缀。
pub fn window_title(app: &AppHandle, title: &str) -> String {
    let args = BTreeMap::from([("title".to_string(), title.to_string())]);
    i18n::text(state::locale(app), "window.titleSuffix", &args)
}

/// 管理窗口的系统标题。
fn manager_title(app: &AppHandle) -> String {
    i18n::text(
        state::locale(app),
        "window.managerTitle",
        &Default::default(),
    )
}

/// 语言切换后更新已存在管理窗口的系统标题。
pub fn refresh_manager_title(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let title = manager_title(app);
        let _ = window.set_title(&title);
    }
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
        let title = manager_title(&app);
        let result = WebviewWindowBuilder::new(&app, "main", WebviewUrl::App("index.html".into()))
            .title(&title)
            .inner_size(1080., 800.)
            .min_inner_size(760., 620.)
            .center()
            // 原生标题栏会与自绘顶栏叠成两层，管理窗口只保留界面内的一条。
            .decorations(false)
            .shadow(false)
            .on_navigation(manager_navigation)
            .on_new_window(|_, _| NewWindowResponse::Deny)
            .build();
        if let Err(error) = result {
            if app.get_webview_window("main").is_none() {
                state::report(
                    &app,
                    AppMessage::new("errors.openManagerFailed").detail(error),
                );
            }
        }
    });
}

/// 按当前配置校验导航目标；来源可以随时被修改，因此不能沿用建窗时的快照。
/// 页面记录已不存在时一律拒绝，宁可不加载也不给已删除的窗口留白名单。
fn navigation_allowed_now(app: &AppHandle, id: &str, url: &url::Url) -> bool {
    app.state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .is_some_and(|pin| content::navigation_allowed(id, &pin.source, url))
}

/// 计算内容窗口的首屏地址，导入页与随包数据的规则在此只保留一处实现。
/// 更新来源后重新调用，可复用同一入口而不必销毁重建窗口。
pub fn initial_url(app: &AppHandle, pin: &PinConfig) -> Result<url::Url, AppMessage> {
    let url = content::source_url(&pin.id, &pin.source)?;
    let PageSource::Bundled { entry } = &pin.source else {
        return Ok(url);
    };
    // 导入页内容在应用数据目录，缺失时给出比导航失败更明确的恢复提示。
    if !crate::import::site_root(app, &pin.id)?
        .join(entry)
        .is_file()
    {
        return Err(AppMessage::new("errors.importContentLost"));
    }
    // 存在随包数据时先加载引导页，回填完成后再跳转到真正的入口。
    let pending = crate::import::pages_root(app)?
        .join(&pin.id)
        .join(crate::import::PENDING_DATA);
    if !pending.is_file() {
        return Ok(url);
    }
    let mut target = url::Url::parse(&format!(
        "pinpage://{}{}",
        pin.id,
        crate::data::RESTORE_PAGE
    ))
    .map_err(|error| AppMessage::new("errors.localUrlInvalid").detail(error))?;
    target.query_pairs_mut().append_pair("entry", entry);
    Ok(target)
}

/// 把已打开窗口指向新的首屏地址，避免销毁重建带来的窗口抖动与几何丢失。
pub fn navigate(app: &AppHandle, id: &str, url: url::Url) -> Result<(), AppMessage> {
    let window = app
        .get_webview_window(id)
        .ok_or_else(|| AppMessage::new("errors.pageClosed"))?;
    window
        .navigate(url)
        .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))?;
    events::watch_load(app, id);
    Ok(())
}

/// 创建一个内容 WebView，来源校验与资源授权在窗口创建前完成。
pub fn open_pin(app: &AppHandle, pin: &PinConfig) -> Result<(), AppMessage> {
    let state = app.state::<AppState>();
    state.ensure_mutable()?;
    crate::platform::validate_pin(crate::platform::capabilities(), pin)?;
    if let Some(window) = app.get_webview_window(&pin.id) {
        window
            .show()
            .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))?;
        if pin.mode == crate::models::WindowMode::Floating {
            window
                .set_focus()
                .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))?;
        }
        return Ok(());
    }
    let url = initial_url(app, pin)?;
    let navigation_id = pin.id.clone();
    let navigation_app = app.clone();
    let popup_app = app.clone();
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
    let (width, height) = limits::clamp_size(pin.width, pin.height);
    let builder = WebviewWindowBuilder::new(app, &pin.id, WebviewUrl::External(url))
        .title(window_title(app, &pin.name))
        .inner_size(width, height)
        .min_inner_size(limits::MIN_WIDTH, limits::MIN_HEIGHT)
        .resizable(!pin.size_locked)
        // 悬浮页通过托盘和管理界面访问，避免多个页面占用任务栏。
        .skip_taskbar(true)
        .always_on_top(pin.mode == crate::models::WindowMode::Floating && pin.always_on_top)
        .decorations(pin.effective_decorations())
        .shadow(pin.effective_decorations())
        .disable_drag_drop_handler()
        .on_navigation(move |url| {
            let allowed = navigation_allowed_now(&navigation_app, &navigation_id, url);
            if !allowed {
                events::set_error(
                    &navigation_app,
                    &navigation_id,
                    AppMessage::new("errors.navigationBlocked"),
                );
            }
            allowed
        })
        .on_new_window(move |url, _| {
            if matches!(url.scheme(), "http" | "https")
                && !url.host_str().unwrap_or("").starts_with("pinpage.")
            {
                if let Err(error) = popup_app.opener().open_url(url.as_str(), None::<&str>) {
                    state::report(
                        &popup_app,
                        AppMessage::new("errors.openExternalFailed").detail(error),
                    );
                }
            }
            NewWindowResponse::Deny
        })
        .on_page_load(events::page_load)
        .on_document_title_changed(move |window, title| {
            // 网页标题只影响系统标题栏，用户设置的名称保持稳定。
            let text: String = title.chars().take(100).collect();
            let title = window_title(window.app_handle(), &text);
            let _ = window.set_title(&title);
        });
    let window = match builder.build() {
        Ok(window) => window,
        Err(error) => {
            let message = AppMessage::new("errors.createWindowFailed").detail(error);
            events::set_error(app, &pin.id, message.clone());
            if let Some(runtime) = state.runtime.lock().unwrap().get_mut(&pin.id) {
                runtime.open = false;
            }
            return Err(message);
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
    let state = app.state::<AppState>();
    if let Err(error) = if state.read_only {
        Ok(())
    } else {
        state.save()
    } {
        state::report(app, error);
        show_manager(app);
        return;
    }
    app.state::<AppState>()
        .exiting
        .store(true, Ordering::SeqCst);
    app.exit(0);
}
