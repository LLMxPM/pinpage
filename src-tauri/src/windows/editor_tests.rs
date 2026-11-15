// 功能：验证局部窗口设置的状态隔离、输入边界及严格内容契约。
use super::*;
use crate::windows::limits;
use serde_json::json;

/// 构造已关闭、带预设的页面，确保编辑不改变生命周期。
fn pin() -> PinConfig {
    serde_json::from_value(json!({
        "id": "p-editor", "name": "原名", "source": {"kind": "url", "url": "https://example.com/"},
        "width": 420, "height": 360, "x": -800, "y": 30, "scale": 1.25,
        "alwaysOnTop": false, "decorations": false, "enabled": false, "preset": "custom-test"
    }))
    .unwrap()
}

#[test]
/// 只修改置顶时沿用最新位置、内容和开启状态，不夹带旧表单几何。
fn patch_preserves_unrelated_state() {
    let previous = pin();
    let patch = serde_json::from_value(json!({"alwaysOnTop": true})).unwrap();
    let next = merge_window(&previous, &patch).unwrap();
    assert!(next.always_on_top);
    assert_eq!(
        (next.x, next.y, next.width, next.height),
        (previous.x, previous.y, previous.width, previous.height)
    );
    assert_eq!(next.name, previous.name);
    assert_eq!(next.source, previous.source);
    assert!(!next.enabled);
    assert!(next.preset.is_none());
}

#[test]
/// 右键开关属于页面行为而非预设字段，切换后保留匹配标记，撤销比较覆盖该字段。
fn page_context_menu_patch_keeps_preset() {
    let previous = pin();
    let patch: WindowPatch = serde_json::from_value(json!({"pageContextMenu": true})).unwrap();
    let next = merge_window(&previous, &patch).unwrap();
    assert!(next.page_context_menu);
    assert_eq!(next.preset, previous.preset);
    assert!(matches_expected(&next, &patch));
    let stale: WindowPatch = serde_json::from_value(json!({"pageContextMenu": false})).unwrap();
    assert!(!matches_expected(&next, &stale));
}

#[test]
/// 未发生变化的字段保留匹配标记，位置锁与大小锁可以独立更新。
fn no_op_and_independent_locks() {
    let previous = pin();
    assert_eq!(
        merge_window(&previous, &WindowPatch::default())
            .unwrap()
            .preset,
        previous.preset
    );
    let patch = serde_json::from_value(json!({"positionLocked": true, "x": -600})).unwrap();
    let next = merge_window(&previous, &patch).unwrap();
    assert!(next.position_locked);
    assert!(!next.size_locked);
    assert_eq!(next.x, Some(-600));
}

#[test]
/// 非法尺寸和混入的身份、内容字段不能通过局部窗口接口写入。
fn rejects_invalid_patch_and_cross_scope_fields() {
    for value in [
        json!({"width": limits::MIN_WIDTH - 1.}),
        json!({"height": limits::MAX_HEIGHT + 1.}),
    ] {
        let patch = serde_json::from_value(value).unwrap();
        assert!(merge_window(&pin(), &patch).is_err());
    }
    for value in [
        json!({"name": "错误入口"}),
        json!({"enabled": true}),
        json!({"source": {}}),
        json!({"x": 0.5}),
    ] {
        assert!(serde_json::from_value::<WindowPatch>(value).is_err());
    }
    assert!(
        serde_json::from_value::<PinContentInput>(json!({"name": "新名", "width": 600})).is_err()
    );
    let content: PinContentInput = serde_json::from_value(json!({"name": "新名"})).unwrap();
    assert!(content.source.is_none());
}

#[test]
/// 撤销在原生层校验受影响字段，窗口随后移动不能被旧的几何撤销覆盖。
fn undo_checks_latest_native_fields() {
    let mut actual = pin();
    let expected: WindowPatch = serde_json::from_value(json!({"x": -800, "width": 420})).unwrap();
    assert!(matches_expected(&actual, &expected));
    actual.x = Some(-600);
    assert!(!matches_expected(&actual, &expected));
    let only_top = serde_json::from_value(json!({"alwaysOnTop": false})).unwrap();
    assert!(matches_expected(&actual, &only_top));
}

#[test]
/// 透明开关保留关闭状态与几何，参与预设失配、往返存储及过期撤销校验。
fn transparency_is_independent_and_undo_checks_it() {
    let previous = pin();
    assert!(!previous.transparent_background);
    let patch = serde_json::from_value(json!({"transparentBackground": true})).unwrap();
    let next = merge_window(&previous, &patch).unwrap();
    assert!(next.transparent_background);
    assert!(!next.enabled);
    assert_eq!(
        (next.x, next.y, next.width, next.height),
        (previous.x, previous.y, previous.width, previous.height)
    );
    assert_eq!(next.source, previous.source);
    assert!(next.preset.is_none());
    assert!(matches_expected(&next, &patch));
    assert!(!matches_expected(&previous, &patch));
    let restored: PinConfig = serde_json::from_str(&serde_json::to_string(&next).unwrap()).unwrap();
    assert!(restored.transparent_background);
}
