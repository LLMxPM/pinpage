// 功能：监听窗口与导航生命周期，更新运行态并防抖保存窗口几何信息。
use crate::state::{self, AppState};
use std::sync::atomic::Ordering;
use tauri::{webview::PageLoadEvent, AppHandle, Manager, WebviewWindow, WindowEvent};

/// 记录单窗加载错误，管理界面可展示失败原因并提供重试。
pub fn set_error(app: &AppHandle, id: &str, error: String) {
    let state = app.state::<AppState>();
    let mut runtime = state.runtime.lock().unwrap();
    let item = runtime.entry(id.into()).or_default();
    item.loading = false;
    item.error = Some(error);
    drop(runtime);
    state::notify(app);
}

/// 页面加载事件仅更新运行态，不把跳转后的敏感地址写入会话配置。
pub fn page_load(window: WebviewWindow, payload: tauri::webview::PageLoadPayload<'_>) {
    let state = window.state::<AppState>();
    let mut runtime = state.runtime.lock().unwrap();
    let item = runtime.entry(window.label().into()).or_default();
    item.loading = matches!(payload.event(), PageLoadEvent::Started);
    item.current_url = payload.url().to_string();
    item.error = None;
    drop(runtime);
    state::notify(window.app_handle());
}

/// 长时间未收到加载完成事件时提示检查地址，同时允许页面继续加载。
pub fn watch_load(app: &AppHandle, id: &str) {
    let app = app.clone();
    let id = id.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(25));
        let loading = app
            .state::<AppState>()
            .runtime
            .lock()
            .unwrap()
            .get(&id)
            .is_some_and(|r| r.loading);
        if loading {
            set_error(
                &app,
                &id,
                "加载耗时较长，请检查地址或网络，也可尝试刷新".into(),
            );
        }
    });
}

/// 保存物理位置与逻辑内容尺寸，防止切换 DPI 后尺寸被重复放大。
fn capture(window: &WebviewWindow) {
    let (Ok(size), Ok(position), Ok(scale)) = (
        window.inner_size(),
        window.outer_position(),
        window.scale_factor(),
    ) else {
        return;
    };
    if size.width == 0 || size.height == 0 || window.is_minimized().unwrap_or(false) {
        return;
    }
    let state = window.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    if let Some(pin) = settings.pins.iter_mut().find(|p| p.id == window.label()) {
        pin.width = size.width as f64 / scale;
        pin.height = size.height as f64 / scale;
        pin.x = Some(position.x);
        pin.y = Some(position.y);
        pin.scale = scale;
    }
    drop(settings);
    state::schedule_save(window.app_handle());
}

/// 注册生命周期事件，用户关窗删除记录，应用退出时保留会话。
pub fn attach(window: &WebviewWindow) {
    let handle = window.clone();
    window.on_window_event(move |event| {
        let app = handle.app_handle();
        match event {
            WindowEvent::Moved(_)
            | WindowEvent::Resized(_)
            | WindowEvent::ScaleFactorChanged { .. } => capture(&handle),
            WindowEvent::Destroyed => {
                let state = app.state::<AppState>();
                state.runtime.lock().unwrap().remove(handle.label());
                if !state.exiting.load(Ordering::SeqCst) {
                    state
                        .settings
                        .lock()
                        .unwrap()
                        .pins
                        .retain(|p| p.id != handle.label());
                    state::schedule_save(app);
                }
                state::notify(app);
                crate::tray::refresh(app);
            }
            _ => {}
        }
    });
    capture(window);
}
