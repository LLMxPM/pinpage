// 功能：串行保存工作台设置，处理语言解析、快捷键回滚与原生界面刷新。
use crate::{
    errors::AppMessage, i18n, models, models::WorkspaceSettings, shortcuts, state, state::AppState,
    tray, windows,
};
use tauri::{AppHandle, Manager, WebviewWindow};

/// 修改语言、会话恢复与快捷键；持久化失败时整体回滚，不留下半套设置。
#[tauri::command]
pub async fn update_settings(
    window: WebviewWindow,
    app: AppHandle,
    input: models::SettingsInput,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    let candidate = WorkspaceSettings::from_input(&input)?;
    let state_ref = app.state::<AppState>();
    // 与管理窗口内的开关、样式切换共用操作锁，避免保存语言时覆盖并发窗口状态。
    let _operation = state_ref.operation.lock().unwrap();
    let previous = WorkspaceSettings::from_settings(&state_ref.settings.lock().unwrap());

    // 先注册新快捷键：冲突时原有设置与原有组合都不发生变化。
    if candidate.manager_shortcut != previous.manager_shortcut
        || candidate.recover_shortcut != previous.recover_shortcut
    {
        shortcuts::configure(
            &app,
            &candidate.manager_shortcut,
            &candidate.recover_shortcut,
        )?;
    }

    apply(&state_ref, &candidate);
    if let Err(error) = state_ref.save() {
        apply(&state_ref, &previous);
        let locale = state::locale(&app);
        if let Err(rollback) = state_ref.save() {
            state::report(
                &app,
                i18n::caused_by(AppMessage::new("errors.rollbackFailed"), locale, &rollback),
            );
        }
        if let Err(rollback) =
            shortcuts::configure(&app, &previous.manager_shortcut, &previous.recover_shortcut)
        {
            state::report(
                &app,
                i18n::caused_by(AppMessage::new("errors.rollbackFailed"), locale, &rollback),
            );
        }
        return Err(error);
    }
    drop(_operation);

    // 持久化成功后才发布新语言，回滚时界面继续显示原语言。
    state::set_locale(&app, i18n::resolve(candidate.locale));
    refresh_native_ui(&app);
    state::notify(&app);
    Ok(())
}

/// 写入可编辑设置字段，保留页面列表与来源配置。
fn apply(state: &AppState, values: &WorkspaceSettings) {
    let mut settings = state.settings.lock().unwrap();
    settings.locale = values.locale;
    settings.restore_session = values.restore_session;
    settings.manager_shortcut = values.manager_shortcut.clone();
    settings.recover_shortcut = values.recover_shortcut.clone();
}

/// 按新的有效语言刷新托盘和管理窗口标题；操作锁已释放，菜单重建不会与状态读取相互等待。
fn refresh_native_ui(app: &AppHandle) {
    windows::refresh_manager_title(app);
    if let Err(error) = tray::refresh(app) {
        // 区分「设置已保存」和「原生显示未刷新」，避免把整次保存误报为失败。
        state::report(
            app,
            i18n::caused_by(
                AppMessage::new("errors.settingsSavedTrayFailed"),
                state::locale(app),
                &error,
            ),
        );
    }
}
