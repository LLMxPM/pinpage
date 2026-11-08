// 功能：验证语言与窗口手势的配置容错、快照契约与消息序列化形状。
use super::*;
use crate::models::{GestureModifier, LocalePreference, Settings};

/// 读取指定 JSON 文本中的配置，模拟不同版本的磁盘文件。
fn settings_from(json: &str) -> Settings {
    serde_json::from_str(json).expect("配置应能解析")
}

/// 构造带一个页面和已废弃快捷键字段的旧版配置文本。
fn legacy_config() -> String {
    serde_json::to_string(&serde_json::json!({
        "schemaVersion": 1,
        "restoreSession": false,
        "managerShortcut": "Ctrl+Alt+J",
        "recoverShortcut": "Ctrl+Alt+K",
        "pins": [{
            "id": "p-old", "name": "计时器", "source": {"kind": "bundled", "entry": "index.html"},
            "width": 420, "height": 360, "x": 10, "y": 20, "scale": 1.0,
            "alwaysOnTop": true, "decorations": true, "enabled": false
        }]
    }))
    .unwrap()
}

#[test]
/// 缺少语言字段的旧配置只回退中文，页面与会话保持不变。
fn missing_locale_keeps_other_settings() {
    let settings = settings_from(&legacy_config());
    assert_eq!(settings.locale, LocalePreference::ZhCn);
    assert!(!settings.restore_session);
    assert_eq!(settings.pins.len(), 1);
    assert!(!settings.pins[0].enabled, "关闭状态不应被语言回退重置");
}

#[test]
/// 未知语言与非字符串语言都只让该字段回退中文。
fn invalid_locale_values_fall_back_to_chinese() {
    for value in ["\"fr-FR\"", "123", "null", "{\"a\":1}"] {
        let json = format!(
            "{{\"schemaVersion\":1,\"locale\":{value},\"pins\":[{{\"id\":\"p-1\",\"name\":\"n\",\
             \"source\":{{\"kind\":\"bundled\",\"entry\":\"i.html\"}},\"width\":420,\"height\":360,\
             \"x\":null,\"y\":null,\"scale\":1,\"alwaysOnTop\":true,\"decorations\":true}}]}}"
        );
        let settings = settings_from(&json);
        assert_eq!(
            settings.locale,
            LocalePreference::ZhCn,
            "取值 {value} 应回退中文"
        );
        assert_eq!(settings.pins.len(), 1, "取值 {value} 不应丢掉页面记录");
    }
}

#[test]
/// 语言偏好按约定字面量读写，往返后不改变取值。
fn locale_round_trips_as_bcp47() {
    let settings = Settings {
        locale: LocalePreference::System,
        ..Settings::default()
    };
    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("\"locale\":\"system\""), "实际结果：{json}");
    let restored: Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.locale, LocalePreference::System);
}

#[test]
/// 新配置与空配置均使用 Alt，序列化字段保持前后端约定的驼峰命名。
fn gesture_modifier_defaults_to_alt() {
    assert_eq!(GestureModifier::default(), GestureModifier::Alt);
    for settings in [Settings::default(), settings_from("{}")] {
        assert_eq!(settings.gesture_modifier, GestureModifier::Alt);
        assert_eq!(
            serde_json::to_value(settings).unwrap()["gestureModifier"],
            "alt"
        );
    }
}

#[test]
/// 缺少手势字段的旧配置只补默认值，不改变页面、语言与会话。
fn missing_gesture_modifier_keeps_legacy_settings() {
    let mut legacy: serde_json::Value = serde_json::from_str(&legacy_config()).unwrap();
    legacy["locale"] = serde_json::json!("en-US");
    let settings: Settings = serde_json::from_value(legacy).unwrap();
    assert_eq!(settings.gesture_modifier, GestureModifier::Alt);
    assert_eq!(settings.locale, LocalePreference::EnUs);
    assert!(!settings.restore_session);
    assert_eq!(settings.pins.len(), 1);
    assert_eq!(settings.pins[0].id, "p-old");
    assert!(!settings.pins[0].enabled);
}

#[test]
/// 坏手势值只回退该字段，全部页面属性及其他全局设置保持原样。
fn invalid_gesture_modifier_keeps_other_settings() {
    let mut legacy: serde_json::Value = serde_json::from_str(&legacy_config()).unwrap();
    legacy["locale"] = serde_json::json!("en-US");
    let expected: Settings = serde_json::from_value(legacy.clone()).unwrap();
    let expected = serde_json::to_value(expected).unwrap();
    for value in [
        "\"meta\"",
        "\"Alt\"",
        "\" ctrl\"",
        "123",
        "null",
        "true",
        "[]",
        "{}",
    ] {
        legacy["gestureModifier"] = serde_json::from_str(value).unwrap();
        let settings: Settings = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(settings.gesture_modifier, GestureModifier::Alt);
        assert_eq!(
            serde_json::to_value(settings).unwrap(),
            expected,
            "取值 {value}"
        );
    }
}

#[test]
/// 四个修饰键值都按稳定字面量在配置中往返，包括显式关闭。
fn gesture_modifiers_round_trip() {
    for (modifier, text) in [
        (GestureModifier::Alt, "alt"),
        (GestureModifier::Ctrl, "ctrl"),
        (GestureModifier::Shift, "shift"),
        (GestureModifier::Disabled, "disabled"),
    ] {
        let settings = Settings {
            gesture_modifier: modifier,
            ..Settings::default()
        };
        let value = serde_json::to_value(settings).unwrap();
        assert_eq!(value["gestureModifier"], text);
        let restored: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(restored.gesture_modifier, modifier);
        assert_eq!(serde_json::to_value(modifier).unwrap(), text);
    }
}

#[test]
/// 消息只带必要字段，空参数与缺失详情不参与序列化。
fn message_serializes_minimally() {
    let plain = AppMessage::new("errors.invalidUrl");
    assert_eq!(
        serde_json::to_string(&plain).unwrap(),
        r#"{"code":"errors.invalidUrl"}"#
    );
    let with_detail = AppMessage::new("errors.saveConfigFailed")
        .arg("max", 4)
        .detail("拒绝访问");
    let value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&with_detail).unwrap()).unwrap();
    assert_eq!(value["args"]["max"], "4");
    assert_eq!(value["detail"], "拒绝访问");
    assert!(value.get("argsFrom").is_none());
}

#[test]
/// 空详情不会污染消息结构，避免界面出现孤立的分隔符。
fn empty_detail_is_dropped() {
    let message = AppMessage::new("errors.unknown").detail("   ");
    assert_eq!(message.detail, None);
}

#[test]
/// 配置损坏时备份原文件并给出可渲染的通知，而不是直接重置。
fn broken_config_is_backed_up_with_notice() {
    let directory = std::env::temp_dir().join(format!("pinpage-i18n-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("settings.json");
    std::fs::write(&path, "{ not json").unwrap();
    let state = AppState::load(path.clone());
    let notices = state.notices.lock().unwrap();
    assert_eq!(notices.len(), 1, "应产生一条备份通知");
    assert_eq!(notices[0].code, "errors.configBackedUp");
    assert!(notices[0].args.contains_key("path"));
    assert_eq!(*state.locale.lock().unwrap(), AppLocale::ZhCn);
    drop(notices);
    let backups = std::fs::read_dir(&directory)
        .unwrap()
        .filter_map(|item| item.ok())
        .filter(|item| {
            item.file_name()
                .to_string_lossy()
                .starts_with("settings.backup-")
        })
        .count();
    assert_eq!(backups, 1, "原配置应被保留为一份备份");
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
/// 快照携带语言偏好、有效语言与手势修饰键，管理界面重建后可直接恢复。
fn snapshot_reports_locale_and_gesture_modifier() {
    let directory = std::env::temp_dir().join(format!("pinpage-i18n-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("settings.json");
    let settings = Settings {
        gesture_modifier: GestureModifier::Shift,
        ..Settings::default()
    };
    std::fs::write(&path, serde_json::to_string(&settings).unwrap()).unwrap();
    let state = AppState::load(path.clone());
    let value: serde_json::Value = serde_json::to_value(state.snapshot()).unwrap();
    assert_eq!(value["settings"]["locale"], "zh-CN");
    assert_eq!(value["settings"]["gestureModifier"], "shift");
    assert_eq!(value["resolvedLocale"], "zh-CN");
    let _ = std::fs::remove_dir_all(&directory);
}
