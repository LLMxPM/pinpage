// 功能：装配应用状态、本地资源协议、托盘及会话恢复。
mod commands;
mod content;
mod data;
mod errors;
mod examples;
mod i18n;
mod import;
mod models;
mod package;
mod platform;
mod source;
mod state;
mod tray;
mod updates;
#[cfg(windows)]
mod window_menu;
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
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            commands::session::import_page,
            commands::session::export_page,
            commands::session::submit_page_data,
            commands::pin_action,
            commands::update_pin,
            commands::editor::update_window,
            commands::editor::update_content,
            commands::editor::write_preset,
            commands::save_preset,
            commands::delete_preset,
            commands::settings::update_settings,
            commands::updates::updates_snapshot,
            commands::updates::update_action,
            commands::quit_app
        ])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            app.manage(AppState::load(path));
            let update_path = app.path().app_config_dir()?.join("updates.json");
            app.manage(updates::UpdateState::load(app.handle(), update_path));
            updates::start_background(app.handle());
            // 托盘在管理界面之前创建，必须先具备正确的原生翻译。
            tray::create(app.handle())?;
            let settings = app.state::<AppState>().settings.lock().unwrap().clone();
            platform::start(app.handle());
            // 托盘启动适用于已有会话，也便于独立采样内容窗口的资源开销。
            if !std::env::args().any(|argument| argument == "--tray") {
                windows::show_manager(app.handle());
            }
            if settings.restore_session {
                let app = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    for pin in settings.pins.into_iter().filter(|pin| pin.enabled) {
                        let state = app.state::<AppState>();
                        let _operation = state.operation.lock().unwrap();
                        if let Err(error) = windows::open_pin(&app, &pin) {
                            // 原因按当前语言渲染为参数，页面名称始终保留用户原值。
                            let reason = i18n::render(state::locale(&app), &error);
                            state::report(
                                &app,
                                errors::AppMessage::new("errors.restoreFailed")
                                    .arg("name", &pin.name)
                                    .arg("reason", reason),
                            );
                        }
                    }
                    tray::refresh_or_report(&app);
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("PinPage 启动失败")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                platform::stop();
            }
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
