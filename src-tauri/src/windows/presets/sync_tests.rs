// 功能：验证稳定 ID 的幂等写入、包列表同步和失败时的窗口状态保留。
use super::*;
use crate::windows::presets::validation::{self, SyncPolicy};

#[test]
/// 同一 ID 的重复创建和更新只保留一项；同名不同 ID 可以共存，关闭页保持关闭。
fn stable_ids_are_idempotent_and_do_not_apply_the_preset() {
    let mut pin = plain_pin("p-stable");
    pin.enabled = false;
    let before = serde_json::to_value(&pin).unwrap();
    storage::put_pin(&mut pin, "compact", editor_input("精简")).unwrap();
    storage::put_pin(&mut pin, "compact", editor_input("精简")).unwrap();
    assert_eq!(pin.presets.len(), 1);
    let mut changed = editor_input("精简");
    changed.fields.width = 320.;
    storage::put_pin(&mut pin, "compact", changed).unwrap();
    storage::put_pin(&mut pin, "other", editor_input("精简")).unwrap();
    assert_eq!(pin.presets.len(), 2);
    let mut after = serde_json::to_value(&pin).unwrap();
    after["presets"] = serde_json::json!([]);
    assert_eq!(after, before, "编辑模板不能改变窗口、标记、来源或开关");
}

#[test]
/// 默认保留用户模板；合并追加新 ID，同 ID 冲突和超量都不产生部分写入。
fn keep_and_merge_preserve_user_edits_and_fail_atomically() {
    let mut pin = plain_pin("p-sync");
    storage::put_pin(&mut pin, "mine", editor_input("我的布局")).unwrap();
    let mut incoming = pin.presets.clone();
    incoming[0].fields.width = 320.;
    let before = serde_json::to_value(&pin).unwrap();
    validation::sync(&mut pin, &incoming, SyncPolicy::Keep).unwrap();
    assert_eq!(serde_json::to_value(&pin).unwrap(), before);
    assert_eq!(
        validation::sync(&mut pin, &incoming, SyncPolicy::Merge)
            .unwrap_err()
            .code,
        "errors.presetConflict"
    );
    assert_eq!(serde_json::to_value(&pin).unwrap(), before);
    incoming[0].id = "new".into();
    validation::sync(&mut pin, &incoming, SyncPolicy::Merge).unwrap();
    validation::sync(&mut pin, &incoming, SyncPolicy::Merge).unwrap();
    assert_eq!(pin.presets.len(), 2);
    let before = serde_json::to_value(&pin).unwrap();
    let incoming: Vec<_> = (0..MAX_CUSTOM)
        .map(|i| WindowPreset {
            id: format!("extra-{i}"),
            name: "新增".into(),
            fields: standard_fields(),
        })
        .collect();
    assert!(validation::sync(&mut pin, &incoming, SyncPolicy::Merge).is_err());
    assert_eq!(serde_json::to_value(&pin).unwrap(), before);
}

#[test]
/// 替换移除或改动匹配项时清除标记，但当前窗口、来源、开关全部保留。
fn replacing_presets_only_changes_library_and_invalid_marker() {
    let mut pin = plain_pin("p-replace");
    storage::put_pin(&mut pin, "mine", editor_input("我的布局")).unwrap();
    pin.preset = Some("mine".into());
    pin.enabled = false;
    let mut before = serde_json::to_value(&pin).unwrap();
    validation::sync(&mut pin, &[], SyncPolicy::Replace).unwrap();
    before["presets"] = serde_json::json!([]);
    before["preset"] = serde_json::Value::Null;
    assert_eq!(serde_json::to_value(&pin).unwrap(), before);
}
