// 功能：验证命名预设的完整包往返、旧包兼容与外部列表的严格校验。
use super::*;
use crate::models::WindowPreset;

/// 同一名称的不同 ID 合法，顺序和每个布局字段都应保留。
fn named_presets() -> Vec<WindowPreset> {
    let compact = WindowPreset {
        id: "compact".into(),
        name: "专注布局".into(),
        fields: standard_fields(),
    };
    let mut expanded = compact.clone();
    expanded.id = "expanded".into();
    expanded.fields.width = 1200.;
    expanded.fields.fx = 1.;
    expanded.fields.transparent_background = true;
    vec![compact, expanded]
}

#[test]
/// 导出后磁盘和内存导入都保留列表；再次导出不会丢失 ID、名称、顺序和参数。
fn named_presets_round_trip_twice_independently_of_default_layout() {
    let fixture = Fixture::new();
    let original = named_presets();
    let package = fixture.0.join("out/presets.pinpage");
    write_dir(
        &fixture.0.join("site"),
        "模板页",
        "index.html",
        None,
        None,
        &original,
        &package,
    )
    .unwrap();
    let manifest = read_manifest(&package).unwrap();
    assert!(manifest.window_preset.is_none());
    assert_eq!(
        serde_json::to_value(&manifest.window_presets).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    let restored = fixture.0.join("out/restored");
    let extracted = extract(&package, &restored).unwrap();
    let memory = extract_bytes(
        &std::fs::read(&package).unwrap(),
        &fixture.0.join("out/memory"),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&memory.window_presets).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    let again = fixture.0.join("out/again.pinpage");
    write_dir(
        &restored,
        "模板页",
        &extracted.entry,
        None,
        None,
        &extracted.window_presets,
        &again,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(read_manifest(&again).unwrap().window_presets).unwrap(),
        serde_json::to_value(original).unwrap()
    );
}

#[test]
/// 畸形模板、重复 ID、危险菜单标识、超额列表和非法名称在创建目标目录之前拒绝。
fn invalid_named_presets_are_rejected_before_extraction() {
    let fixture = Fixture::new();
    let valid = serde_json::to_value(named_presets()).unwrap();
    let mut cases = vec![];
    for (key, value) in [
        ("id", serde_json::json!("bad|preset:other")),
        ("name", serde_json::json!("  ")),
        ("name", serde_json::json!("名".repeat(41))),
        ("width", serde_json::json!(0)),
        ("fy", serde_json::json!(1.1)),
    ] {
        let mut list = valid.clone();
        list[0][key] = value;
        cases.push(list);
    }
    let mut duplicate = valid.clone();
    duplicate[1]["id"] = duplicate[0]["id"].clone();
    cases.push(duplicate);
    let mut missing = valid.clone();
    missing[0].as_object_mut().unwrap().remove("width");
    cases.push(missing);
    cases.push(serde_json::json!((0..21)
        .map(|i| {
            let mut p = valid[0].clone();
            p["id"] = serde_json::json!(format!("p-{i}"));
            p
        })
        .collect::<Vec<_>>()));
    for (i, list) in cases.into_iter().enumerate() {
        let manifest = serde_json::json!({"schemaVersion":1,"name":"无效模板","entry":"index.html","exportedAt":0,"files":1,"windowPresets":list}).to_string();
        let path = fixture.0.join(format!("out/invalid-{i}.pinpage"));
        write_zip(
            &path,
            &[
                ("manifest.json", manifest.as_bytes()),
                ("site/index.html", b"<html></html>"),
            ],
        );
        let target = fixture.0.join(format!("out/invalid-{i}"));
        assert!(extract(&path, &target).is_err(), "case {i}");
        assert!(!target.exists());
    }
}

#[test]
/// 旧包缺失命名列表时恢复为空，不凭默认布局偷偷增加一个用户模板。
fn legacy_package_does_not_create_a_named_preset() {
    let fixture = Fixture::new();
    let path = fixture.0.join("out/legacy.pinpage");
    let manifest = serde_json::json!({"schemaVersion":1,"name":"旧包","entry":"index.html","exportedAt":0,"files":1,"windowPreset":standard_fields()}).to_string();
    write_zip(
        &path,
        &[
            ("manifest.json", manifest.as_bytes()),
            ("site/index.html", b"<html></html>"),
        ],
    );
    let extracted = extract(&path, &fixture.0.join("out/legacy")).unwrap();
    assert!(extracted.window_presets.is_empty());
    assert!(extracted.window_preset.is_some());
}
