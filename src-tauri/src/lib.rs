// 功能：装配应用状态、本地资源协议、托盘、快捷键及会话恢复。
mod commands;
mod content;
mod data;
mod examples;
mod import;
mod models;
mod package;
mod platform;
mod shortcuts;
mod state;
mod tray;
mod windows;

use state::AppState;
use tauri::Manager;

/// 启动应用；关闭所有窗口后由托盘继续管理，显式退出时保存会话。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            windows::show_manager(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(shortcuts::plugin())
        .register_asynchronous_uri_scheme_protocol("pinpage", |context, request, responder| {
            let app = context.app_handle().clone();
            let label = context.webview_label().to_string();
            tauri::async_runtime::spawn_blocking(move || {
                responder.respond(content::respond(&app, &label, request))
            });
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::create_pin,
            commands::import_page,
            commands::export_page,
            commands::submit_page_data,
            commands::pin_action,
            commands::update_pin,
            commands::update_settings,
            commands::quit_app
        ])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            app.manage(AppState::load(path));
            tray::create(app.handle())?;
            let settings = app.state::<AppState>().settings.lock().unwrap().clone();
            if let Err(error) = shortcuts::configure(
                app.handle(),
                &settings.manager_shortcut,
                &settings.recover_shortcut,
            ) {
                state::report(app.handle(), error);
            }
            platform::start(app.handle());
            // 托盘启动适用于已有会话，也便于独立采样内容窗口的资源开销。
            if !std::env::args().any(|argument| argument == "--tray") {
                windows::show_manager(app.handle());
            }
            if settings.restore_session {
                let app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    for pin in settings.pins.into_iter().filter(|pin| pin.enabled) {
                        if let Err(error) = windows::open_pin(&app, &pin) {
                            state::report(&app, format!("恢复 {} 失败：{error}", pin.name));
                        }
                    }
                    tray::refresh(&app);
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("PinPage 启动失败")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !app
                    .state::<AppState>()
                    .exiting
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    api.prevent_exit();
                }
            }
        });
}
