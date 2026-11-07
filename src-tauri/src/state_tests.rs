// 功能：验证语言偏好的配置容错、快照契约与消息序列化形状。
use super::*;
use crate::models::{LocalePreference, Settings};

/// 读取指定 JSON 文本中的配置，模拟不同版本的磁盘文件。
fn settings_from(json: &str) -> Settings {
    serde_json::from_str(json).expect("配置应能解析")
}

/// 构造带一个页面和自定义快捷键的旧版配置文本。
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
/// 缺少语言字段的旧配置只回退中文，页面与快捷键保持不变。
fn missing_locale_keeps_other_settings() {
    let settings = settings_from(&legacy_config());
    assert_eq!(settings.locale, LocalePreference::ZhCn);
    assert!(!settings.restore_session);
    assert_eq!(settings.manager_shortcut, "Ctrl+Alt+J");
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
/// 快照同时携带偏好与有效语言，管理界面重建后可直接恢复。
fn snapshot_reports_preferred_and_effective_locale() {
    let directory = std::env::temp_dir().join(format!("pinpage-i18n-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("settings.json");
    std::fs::write(&path, serde_json::to_string(&Settings::default()).unwrap()).unwrap();
    let state = AppState::load(path.clone());
    let value: serde_json::Value = serde_json::to_value(state.snapshot()).unwrap();
    assert_eq!(value["settings"]["locale"], "zh-CN");
    assert_eq!(value["resolvedLocale"], "zh-CN");
    let _ = std::fs::remove_dir_all(&directory);
}
