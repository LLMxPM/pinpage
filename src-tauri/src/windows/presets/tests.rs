// 功能：验证页面级自定义预设、归一化约束与跨屏相对位置换算。
use super::*;
use crate::models::{PresetFields, PresetInput, WindowMode, WindowPreset};

/// 标准窗口字段基准，替代已移除的内置预设，供各测试复用。
fn standard_fields() -> PresetFields {
    PresetFields {
        mode: WindowMode::Floating,
        always_on_top: false,
        decorations: true,
        rounded: false,
        transparent_background: false,
        position_locked: false,
        size_locked: false,
        width: 960.,
        height: 720.,
        fx: 0.5,
        fy: 0.5,
        fullscreen: false,
    }
}

#[test]
/// 包内布局跨平台回退只改变新页的外观几何，不带入包作者的 ID 或全局预设引用。
fn package_window_preset_falls_back_without_mutating_content() {
    let mut pin: PinConfig = serde_json::from_value(serde_json::json!({
        "id": "p-new", "name": "独立副本", "source": {"kind": "bundled", "entry": "index.html"},
        "width": 420, "height": 360, "x": null, "y": null, "scale": 1.,
        "alwaysOnTop": true, "decorations": false
    }))
    .unwrap();
    let mut fields = standard_fields();
    fields.mode = WindowMode::Desktop;
    fields.transparent_background = true;
    fields.position_locked = true;
    fields.size_locked = true;
    fields.width = 300.;
    fields.height = 240.;
    let mut capabilities = crate::platform::capabilities();
    capabilities.desktop_mode = false;
    capabilities.rounded = false;
    capabilities.transparent_background = false;
    capabilities.position_lock = false;
    capabilities.size_lock = false;
    capabilities.drag_modifier = None;
    initialize_fields(&mut pin, &mut fields, capabilities);
    assert_eq!(pin.id, "p-new");
    assert_eq!(pin.name, "独立副本");
    assert_eq!(pin.mode, WindowMode::Floating);
    assert!(pin.decorations);
    assert!(!pin.transparent_background && !pin.position_locked && !pin.size_locked);
    assert_eq!((pin.width, pin.height), (300., 240.));
    assert!(pin.preset.is_none() && pin.enabled);
}

/// 构造独立的编辑输入，不依赖真实显示器或用户配置。
fn editor_input(name: &str) -> PresetInput {
    PresetInput {
        name: name.into(),
        fields: standard_fields(),
    }
}

/// 构造一个只含基础字段的页面配置，预设相关字段由测试自行补齐。
fn plain_pin(id: &str) -> PinConfig {
    serde_json::from_value(serde_json::json!({
        "id": id, "name": "计时器", "source": {"kind": "bundled", "entry": "index.html"},
        "width": 960, "height": 720, "x": 100, "y": 100, "scale": 1.,
        "alwaysOnTop": false, "decorations": true
    }))
    .unwrap()
}

#[test]
/// 重命名保留匹配，修改模板参数不联动窗口，删除模板也只移除标记。
fn editing_and_deleting_presets_preserve_windows() {
    let mut pin = plain_pin("p-template");
    let id = storage::write_pin(&mut pin, None, editor_input("初始模板")).unwrap();
    pin.preset = Some(id.clone());
    let before = pin.clone();
    storage::write_pin(&mut pin, Some(&id), editor_input("重命名")).unwrap();
    assert_eq!(pin.preset.as_deref(), Some(id.as_str()));
    let mut changed = editor_input("修改尺寸");
    changed.fields.width = 500.;
    storage::write_pin(&mut pin, Some(&id), changed).unwrap();
    assert_eq!(pin.width, before.width);
    assert_eq!(pin.source, before.source);
    assert!(pin.preset.is_none());
    pin.preset = Some(id.clone());
    storage::delete_pin(&mut pin, &id).unwrap();
    assert!(pin.presets.is_empty());
    assert_eq!(pin.name, before.name);
    assert_eq!(pin.x, before.x);
    assert!(pin.preset.is_none());
}

#[test]
/// 编辑或删除不存在的模板被拒绝，非法参数被拒绝，达到数量上限后仍可编辑已有项。
fn template_permissions_validation_and_limit() {
    let mut pin = plain_pin("p-limits");
    assert!(storage::write_pin(&mut pin, Some("missing"), editor_input("不存在的模板")).is_err());
    assert!(storage::delete_pin(&mut pin, "missing").is_err());
    for (width, fx) in [
        (f64::NAN, 0.),
        (crate::windows::limits::MIN_WIDTH - 1., 0.),
        (500., -0.1),
        (500., f64::INFINITY),
    ] {
        let mut input = editor_input("非法参数");
        input.fields.width = width;
        input.fields.fx = fx;
        assert!(storage::write_pin(&mut pin, None, input).is_err());
    }
    for index in 0..MAX_CUSTOM {
        storage::write_pin(&mut pin, None, editor_input(&format!("预设{index}"))).unwrap();
    }
    assert!(storage::write_pin(&mut pin, None, editor_input("超额")).is_err());
    let id = pin.presets[0].id.clone();
    assert!(storage::write_pin(&mut pin, Some(&id), editor_input("仍可修改")).is_ok());
    // 上限按页面独立计算，另一个页面不受已满列表影响。
    let mut other = plain_pin("p-other");
    assert!(storage::write_pin(&mut other, None, editor_input("独立页面")).is_ok());
}

#[test]
/// 旧模板缺失透明字段时保留不透明外观，新模板的透明偏好可往返持久化。
fn old_presets_default_to_opaque() {
    let mut value = serde_json::to_value(editor_input("旧模板").fields).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("transparentBackground");
    let mut fields: PresetFields = serde_json::from_value(value).unwrap();
    assert!(!fields.transparent_background);
    fields.transparent_background = true;
    let restored: PresetFields =
        serde_json::from_str(&serde_json::to_string(&fields).unwrap()).unwrap();
    assert!(restored.transparent_background);
}

/// 桌面模式不允许置顶但保留圆角；浮动模式有系统边框时圆角必须失效。
#[test]
fn normalize_removes_conflicting_appearance() {
    let mut desktop = PresetFields {
        mode: WindowMode::Desktop,
        always_on_top: true,
        decorations: true,
        rounded: true,
        transparent_background: false,
        position_locked: false,
        size_locked: false,
        width: 10.,
        height: 9000.,
        fx: 2.,
        fy: -1.,
        fullscreen: false,
    };
    normalize(&mut desktop);
    assert!(!desktop.always_on_top);
    assert!(desktop.rounded, "桌面模式不画系统边框，圆角仍然有效");
    assert_eq!(desktop.width, crate::windows::limits::MIN_WIDTH);
    assert_eq!(desktop.height, crate::windows::limits::MAX_HEIGHT);
    assert_eq!((desktop.fx, desktop.fy), (1., 0.));

    let mut bordered = PresetFields {
        mode: WindowMode::Floating,
        decorations: true,
        rounded: true,
        transparent_background: false,
        ..desktop.clone()
    };
    normalize(&mut bordered);
    assert!(!bordered.rounded, "有系统边框时圆角不生效");
}

/// 自定义预设保留用户名称并按保存顺序输出；不同页面可保存同名模板互不影响。
#[test]
fn custom_presets_keep_user_names() {
    assert!(normalized(&[]).is_empty(), "不再提供内置预设");
    let custom = vec![WindowPreset {
        id: "custom-1".into(),
        name: "我的挂件".into(),
        fields: PresetFields {
            mode: WindowMode::Floating,
            always_on_top: false,
            decorations: false,
            rounded: true,
            transparent_background: false,
            position_locked: false,
            size_locked: false,
            width: 300.,
            height: 300.,
            fx: 0.,
            fy: 0.,
            fullscreen: false,
        },
    }];
    let views = normalized(&custom);
    assert_eq!(views.len(), 1);
    let only = &views[0];
    assert_eq!(only.id, "custom-1");
    assert_eq!(only.name, "我的挂件");
}

#[test]
/// v1 迁移语义：页内查找只认自己的列表，引用其他页面模板的标记收敛为“自定义”。
fn find_resolves_within_own_page_only() {
    let mut pin = plain_pin("p-own");
    let id = storage::write_pin(&mut pin, None, editor_input("我的模板")).unwrap();
    assert!(find(&pin, &id).is_some());
    let mut other = plain_pin("p-other");
    other.preset = Some(id.clone());
    assert!(find(&other, &id).is_none(), "不应看到其他页面的模板");
    other.presets.push(WindowPreset {
        id: id.clone(),
        name: "同名不同页".into(),
        fields: standard_fields(),
    });
    let found = find(&other, &id).unwrap();
    assert_eq!(found.name, "同名不同页");
}

/// 归一化比例与屏幕坐标必须能互相还原，覆盖四个角与居中。
#[test]
fn position_round_trip() {
    let area = (100, 50, 1920, 1040);
    let width = 400;
    let height = 300;
    let room = (area.2 - width, area.3 - height);
    for (fx, fy) in [(0., 0.), (1., 0.), (0., 1.), (1., 1.), (0.5, 0.5)] {
        let x = area.0 + (fx * room.0 as f64).round() as i32;
        let y = area.1 + (fy * room.1 as f64).round() as i32;
        let back_x = (x - area.0) as f64 / room.0 as f64;
        let back_y = (y - area.1) as f64 / room.1 as f64;
        assert!((back_x - fx).abs() < 0.001);
        assert!((back_y - fy).abs() < 0.001);
    }
}
