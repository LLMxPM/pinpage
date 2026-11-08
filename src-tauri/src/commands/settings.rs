// 功能：串行保存工作台设置，处理手势发布、语言解析与原生界面刷新。
use crate::{
    errors::AppMessage, i18n, models, models::WorkspaceSettings, state, state::AppState, tray,
    windows,
};
use tauri::{AppHandle, Manager, WebviewWindow};

/// 修改语言、窗口手势与会话恢复；持久化失败时整体回滚，不留下半套设置。
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
    state_ref.ensure_mutable()?;
    let previous = WorkspaceSettings::from_settings(&state_ref.settings.lock().unwrap());

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
        return Err(error);
    }
    // 仅保存成功后发布手势，且在操作锁内发布，避免并发保存使原生状态次序反转。
    crate::platform::set_gesture_modifier(candidate.gesture_modifier);
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
mod tests {
    use super::*;
    use crate::models::{GestureModifier, Settings, SettingsInput};
    use serde_json::json;

    #[test]
    /// 编辑快照、应用及失败回滚覆盖全部设置字段，且始终保留页面配置。
    fn apply_and_rollback_preserve_all_workspace_fields() {
        let path = std::env::temp_dir()
            .join(format!("pinpage-settings-{}", uuid::Uuid::new_v4()))
            .join("settings.json");
        let state = AppState::load(path);
        for (before, after) in [
            (GestureModifier::Alt, "ctrl"),
            (GestureModifier::Ctrl, "shift"),
            (GestureModifier::Shift, "disabled"),
            (GestureModifier::Disabled, "alt"),
        ] {
            // 旧配置中的快捷键字段已经废弃，反序列化时应被忽略。
            let original: Settings = serde_json::from_value(json!({
                "locale": "system", "gestureModifier": before, "restoreSession": true,
                "managerShortcut": "Ctrl+Alt+P", "recoverShortcut": "Ctrl+Alt+B",
                "pins": [{
                    "id": "p-keep", "name": "保留页面",
                    "source": {"kind": "bundled", "entry": "index.html"},
                    "width": 420, "height": 360, "x": 10, "y": 20, "scale": 1.0,
                    "alwaysOnTop": true, "decorations": false, "enabled": false
                }]
            }))
            .unwrap();
            let baseline = serde_json::to_value(&original).unwrap();
            *state.settings.lock().unwrap() = original;
            let previous = WorkspaceSettings::from_settings(&state.settings.lock().unwrap());
            assert_eq!(previous.gesture_modifier, before);

            let input = SettingsInput {
                locale: "en-US".into(),
                gesture_modifier: after.into(),
                restore_session: false,
            };
            let candidate = WorkspaceSettings::from_input(&input).unwrap();
            apply(&state, &candidate);
            let mut expected = baseline.clone();
            expected["locale"] = json!("en-US");
            expected["gestureModifier"] = json!(after);
            expected["restoreSession"] = json!(false);
            assert_eq!(
                serde_json::to_value(state.snapshot().settings).unwrap(),
                expected
            );

            // 与保存失败分支使用同一个 apply 入口，回滚后包括手势在内的字段必须完全复原。
            apply(&state, &previous);
            assert_eq!(
                serde_json::to_value(state.snapshot().settings).unwrap(),
                baseline
            );
        }
    }
}
