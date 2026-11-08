// 功能：监听窗口与导航生命周期，更新运行态并防抖保存窗口几何信息。
use crate::{
    errors::AppMessage,
    state::{self, AppState},
};
use std::sync::atomic::Ordering;
use tauri::{webview::PageLoadEvent, AppHandle, Manager, WebviewWindow, WindowEvent};

/// 记录单窗加载错误，管理界面可展示失败原因并提供重试。
pub fn set_error(app: &AppHandle, id: &str, error: AppMessage) {
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
            set_error(&app, &id, AppMessage::new("errors.loadSlow"));
        }
    });
}

/// 保存物理位置与逻辑内容尺寸，防止切换 DPI 后尺寸被重复放大。
pub(super) fn capture(window: &WebviewWindow) {
    let (Ok(size), Ok(position), Ok(scale)) = (
        window.inner_size(),
        crate::platform::position(window),
        window.scale_factor(),
    ) else {
        return;
    };
    if size.width == 0 || size.height == 0 || window.is_minimized().unwrap_or(false) {
        return;
    }
    let state = window.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    // 安装前的最终快照之后不再吸收窗口事件，避免覆盖刚保存的会话。
    if state.installing.load(Ordering::SeqCst) || state.read_only {
        return;
    }
    let mut changed = false;
    if let Some(pin) = settings.pins.iter_mut().find(|p| p.id == window.label()) {
        changed = pin.width != size.width as f64 / scale
            || pin.height != size.height as f64 / scale
            || pin.x != Some(position.x)
            || pin.y != Some(position.y)
            || pin.scale != scale;
        pin.width = size.width as f64 / scale;
        pin.height = size.height as f64 / scale;
        pin.x = Some(position.x);
        pin.y = Some(position.y);
        pin.scale = scale;
    }
    drop(settings);
    if changed {
        state::schedule_save(window.app_handle());
        state::notify(window.app_handle());
    }
}

/// 用户关窗只关闭页面，保留记录；异常销毁也保留配置用于重新开启。
pub fn attach(window: &WebviewWindow) {
    let handle = window.clone();
    window.on_window_event(move |event| {
        let app = handle.app_handle();
        match event {
            WindowEvent::CloseRequested { api, .. } => {
                if !app.state::<AppState>().exiting.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let app = app.clone();
                    let id = handle.label().to_string();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = super::action(&app, &id, "close") {
                            state::report(&app, error);
                        }
                    });
                }
            }
            WindowEvent::Moved(_)
            | WindowEvent::Resized(_)
            | WindowEvent::ScaleFactorChanged { .. } => capture(&handle),
            WindowEvent::Destroyed => {
                let state = app.state::<AppState>();
                let expected = state
                    .settings
                    .lock()
                    .unwrap()
                    .pins
                    .iter()
                    .find(|p| p.id == handle.label())
                    .is_some_and(|p| !p.enabled);
                if let Some(runtime) = state.runtime.lock().unwrap().get_mut(handle.label()) {
                    runtime.open = false;
                    runtime.visible = false;
                    runtime.loading = false;
                    if !expected && !state.exiting.load(Ordering::SeqCst) {
                        runtime.error = Some(AppMessage::new("errors.windowClosed"));
                    }
                }
                state::schedule_save(app);
                state::notify(app);
                crate::tray::refresh_or_report(app);
            }
            _ => {}
        }
    });
    capture(window);
}
