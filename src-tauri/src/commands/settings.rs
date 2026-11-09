// 功能：串行保存工作台设置，协调系统自启注册、失败回滚与原生界面刷新。
use crate::{
    errors::AppMessage, i18n, models, models::WorkspaceSettings, state, state::AppState, tray,
    windows,
};
use tauri::{AppHandle, Manager, WebviewWindow};

/// 修改全局设置；自启参数为空表示未编辑开关，不覆盖系统外部的更改。
#[tauri::command]
pub async fn update_settings(
    window: WebviewWindow,
    app: AppHandle,
    input: models::SettingsInput,
    autostart: Option<bool>,
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    let candidate = WorkspaceSettings::from_input(&input)?;
    let state_ref = app.state::<AppState>();
    // 与管理窗口内的开关、样式切换共用操作锁，避免保存语言时覆盖并发窗口状态。
    let _operation = state_ref.operation.lock().unwrap();
    state_ref.ensure_mutable()?;
    save_transaction(
        &state_ref,
        &candidate,
        autostart,
        || crate::platform::autostart::is_enabled(&app),
        |enabled| crate::platform::autostart::set_enabled(&app, enabled),
        |error| state::report(&app, error),
    )?;
    // 仅保存成功后发布手势，且在操作锁内发布，避免并发保存使原生状态次序反转。
    crate::platform::set_gesture_modifier(candidate.gesture_modifier);
    drop(_operation);

    // 持久化成功后才发布新语言，回滚时界面继续显示原语言。
    state::set_locale(&app, i18n::resolve(candidate.locale));
    refresh_native_ui(&app);
    state::notify(&app);
    Ok(())
}

/// 调用者持有操作锁；系统注册与配置一起提交，任一失败都尝试恢复原值并报告回滚失败。
fn save_transaction(
    state: &AppState,
    candidate: &WorkspaceSettings,
    autostart: Option<bool>,
    read_autostart: impl FnOnce() -> Result<bool, AppMessage>,
    mut write_autostart: impl FnMut(bool) -> Result<(), AppMessage>,
    mut report: impl FnMut(AppMessage),
) -> Result<(), AppMessage> {
    state.ensure_mutable()?;
    let previous = WorkspaceSettings::from_settings(&state.settings.lock().unwrap());
    let change = autostart
        .map(|enabled| read_autostart().map(|before| (before, enabled)))
        .transpose()?
        .filter(|(before, enabled)| before != enabled);
    if let Some((before, enabled)) = change {
        if let Err(error) = write_autostart(enabled) {
            // 原生操作可能部分成功；即使返回失败也尝试恢复系统注册。
            if let Err(rollback) = write_autostart(before) {
                report(autostart_rollback_error(state, &rollback));
            }
            return Err(error);
        }
    }
    apply(state, candidate);
    if let Err(error) = state.save() {
        apply(state, &previous);
        if let Some((before, _)) = change {
            if let Err(rollback) = write_autostart(before) {
                report(autostart_rollback_error(state, &rollback));
            }
        }
        if let Err(rollback) = state.save() {
            report(i18n::caused_by(
                AppMessage::new("errors.rollbackFailed"),
                *state.locale.lock().unwrap(),
                &rollback,
            ));
        }
        return Err(error);
    }
    Ok(())
}

/// 自启回滚失败与普通保存失败分开呈现，避免误报系统注册已恢复。
fn autostart_rollback_error(state: &AppState, error: &AppMessage) -> AppMessage {
    i18n::caused_by(
        AppMessage::new("errors.autostartRollbackFailed"),
        *state.locale.lock().unwrap(),
        error,
    )
}

/// 写入可编辑设置字段，保留页面列表与来源配置。
fn apply(state: &AppState, values: &WorkspaceSettings) {
    let mut settings = state.settings.lock().unwrap();
    settings.locale = values.locale;
    settings.gesture_modifier = values.gesture_modifier;
    settings.restore_session = values.restore_session;
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

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "settings_autostart_tests.rs"]
mod autostart_tests;
