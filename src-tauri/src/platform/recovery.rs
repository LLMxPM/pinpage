// 功能：在 Explorer 桌面宿主恢复后，重新开启意外关闭的桌面页面。
use crate::{models::WindowMode, state::AppState};
use tauri::{AppHandle, Manager};

/// 仅重试本次运行中已失败的桌面窗口，尊重手动关闭及关闭启动恢复的设置。
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(3));
        let state = app.state::<AppState>();
        if state.exiting.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        let pins = state.settings.lock().unwrap().pins.clone();
        for pin in pins
            .into_iter()
            .filter(|pin| pin.enabled && pin.mode == WindowMode::Desktop)
        {
            let failed = state
                .runtime
                .lock()
                .unwrap()
                .get(&pin.id)
                .is_some_and(|runtime| !runtime.open && runtime.error.is_some());
            if !failed
                || app.get_webview_window(&pin.id).is_some()
                || super::desktop::host().is_err()
            {
                continue;
            }
            let Ok(_operation) = state.operation.try_lock() else {
                continue;
            };
            // 取得操作锁后重读开启状态，避免与用户的关闭动作竞争。
            let enabled = state
                .settings
                .lock()
                .unwrap()
                .pins
                .iter()
                .find(|saved| saved.id == pin.id)
                .is_some_and(|saved| saved.enabled);
            if enabled {
                let _ = crate::windows::open_pin(&app, &pin);
            }
        }
    });
}
