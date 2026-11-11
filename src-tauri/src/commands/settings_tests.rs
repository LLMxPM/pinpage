// 功能：验证全局设置的字段保留、自启注册与持久化失败回滚。
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
        (GestureModifier::Ctrl, "shift"),
        (GestureModifier::Shift, "disabled"),
        (GestureModifier::Disabled, "ctrl"),
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
