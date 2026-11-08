// 功能：验证旧配置兼容、开关持久化及单页设置的几何约束。
use super::*;
use crate::models::SourceInput;

/// 模拟缺少新增字段的配置，验证旧页面不会被重置。
fn old_pin() -> PinConfig {
    serde_json::from_value(serde_json::json!({
        "id": "p-test", "name": "计时器",
        "source": {"kind": "bundled", "entry": "index.html"},
        "width": 420, "height": 360, "x": 10, "y": 20, "scale": 1.25,
        "alwaysOnTop": true, "decorations": true
    }))
    .unwrap()
}

/// 提供含桌面模式与独立锁定的合法设置输入。
fn input() -> PinSettingsInput {
    serde_json::from_value(serde_json::json!({
        "name": " 桌面时钟 ", "mode": "desktop", "alwaysOnTop": true,
        "decorations": false, "rounded": true, "positionLocked": true,
        "sizeLocked": false, "width": 600, "height": 240, "x": -300, "y": 100
    }))
    .unwrap()
}

#[test]
/// 旧配置默认开启且保持浮动，关闭状态可往返序列化。
fn old_settings_and_closed_state_survive() {
    let mut pin = old_pin();
    assert!(pin.enabled);
    assert_eq!(pin.mode, crate::models::WindowMode::Floating);
    assert!(!pin.rounded && !pin.position_locked && !pin.size_locked);
    pin.enabled = false;
    let saved = serde_json::to_string(&pin).unwrap();
    let restored: PinConfig = serde_json::from_str(&saved).unwrap();
    assert!(!restored.enabled);
    assert_eq!(restored.id, pin.id);
}

#[test]
/// 设置保留来源和开启状态，位置与尺寸锁互不依赖。
fn updates_preserve_identity_and_independent_locks() {
    let mut previous = old_pin();
    previous.enabled = false;
    let next = merge(&previous, &input()).unwrap();
    assert_eq!(next.id, previous.id);
    assert!(!next.enabled);
    assert_eq!(next.name, "桌面时钟");
    assert_eq!(next.x, Some(-300));
    assert!(next.position_locked && !next.size_locked && next.rounded);
    assert!(!next.effective_decorations());
    let mut changed = input();
    changed.position_locked = false;
    changed.size_locked = true;
    let next = merge(&previous, &changed).unwrap();
    assert!(!next.position_locked && next.size_locked);
}

#[test]
/// 非法尺寸和空名称在任何原生调用之前拒绝。
fn rejects_invalid_geometry() {
    for width in [f64::NAN, f64::INFINITY, 0., 159., 3841.] {
        let mut value = input();
        value.width = width;
        assert!(merge(&old_pin(), &value).is_err());
    }
    let mut value = input();
    value.height = 99.;
    assert!(merge(&old_pin(), &value).is_err());
    let mut value = input();
    value.name = "  ".into();
    assert!(merge(&old_pin(), &value).is_err());
}

#[test]
/// 来源缺失字段按“沿用原来源”解析，外观提交不会改写已有来源。
fn missing_source_field_keeps_existing_source() {
    let value = input();
    assert!(value.source.is_none());
    let next = merge(&old_pin(), &value).unwrap();
    assert_eq!(next.source, old_pin().source);
}

#[test]
/// 来源兑换属于更新流程，合并阶段只校验外观，不会把输入里的来源写入配置。
fn merge_never_applies_the_submitted_source() {
    let mut value = input();
    value.source = Some(SourceInput::Url {
        address: "example.com".into(),
    });
    let next = merge(&old_pin(), &value).unwrap();
    assert_eq!(next.source, old_pin().source);
    assert_eq!(next.name, "桌面时钟");
}

#[test]
/// 预设标记随表单提交落库，缺省时视为自定义，不影响其他字段。
fn merge_carries_preset_marker() {
    let value = serde_json::json!({
        "name": "计时器", "mode": "floating", "alwaysOnTop": false,
        "decorations": false, "rounded": true, "positionLocked": false,
        "sizeLocked": false, "width": 420, "height": 360, "x": 10, "y": 10,
        "preset": "minimal"
    });
    let tagged: PinSettingsInput = serde_json::from_value(value.clone()).unwrap();
    let next = merge(&old_pin(), &tagged).unwrap();
    assert_eq!(next.preset.as_deref(), Some("minimal"));
    assert_eq!(next.width, 420.);

    let mut untagged = value;
    untagged.as_object_mut().unwrap().remove("preset");
    let plain: PinSettingsInput = serde_json::from_value(untagged).unwrap();
    assert!(merge(&old_pin(), &plain).unwrap().preset.is_none());
}
