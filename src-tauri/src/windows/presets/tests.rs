// 功能：验证内置预设、归一化约束与跨屏相对位置换算。
use super::*;
use crate::models::{PresetFields, PresetInput, Settings, WindowMode, WindowPreset};

/// 构造独立的编辑输入，不依赖真实显示器或用户配置。
fn editor_input(name: &str) -> PresetInput {
    PresetInput {
        name: name.into(),
        fields: all(&[])[1].fields.clone(),
    }
}

#[test]
/// 重命名保留匹配，修改模板参数不联动窗口，删除模板也只移除标记。
fn editing_and_deleting_presets_preserve_windows() {
    let mut settings = Settings::default();
    let id = storage::write_settings(&mut settings, None, editor_input("初始模板")).unwrap();
    let pin: PinConfig = serde_json::from_value(serde_json::json!({
        "id": "p-template", "name": "计时器", "source": {"kind": "bundled", "entry": "index.html"},
        "width": 960, "height": 720, "x": 100, "y": 100, "scale": 1.,
        "alwaysOnTop": false, "decorations": true, "preset": id
    }))
    .unwrap();
    settings.pins.push(pin.clone());
    storage::write_settings(&mut settings, Some(&id), editor_input("重命名")).unwrap();
    assert_eq!(settings.pins[0].preset.as_deref(), Some(id.as_str()));
    let mut changed = editor_input("修改尺寸");
    changed.fields.width = 500.;
    storage::write_settings(&mut settings, Some(&id), changed).unwrap();
    assert_eq!(settings.pins[0].width, pin.width);
    assert_eq!(settings.pins[0].source, pin.source);
    assert!(settings.pins[0].preset.is_none());
    settings.pins[0].preset = Some(id.clone());
    storage::delete_settings(&mut settings, &id).unwrap();
    assert!(settings.window_presets.is_empty());
    assert_eq!(settings.pins[0].name, pin.name);
    assert_eq!(settings.pins[0].x, pin.x);
    assert!(settings.pins[0].preset.is_none());
}

#[test]
/// 内置项不可写删，非法参数被拒绝，达到数量上限后仍可编辑已有项。
fn template_permissions_validation_and_limit() {
    let mut settings = Settings::default();
    assert!(
        storage::write_settings(&mut settings, Some("standard"), editor_input("替换内置")).is_err()
    );
    assert!(storage::delete_settings(&mut settings, "standard").is_err());
    for (width, fx) in [
        (f64::NAN, 0.),
        (159., 0.),
        (500., -0.1),
        (500., f64::INFINITY),
    ] {
        let mut input = editor_input("非法参数");
        input.fields.width = width;
        input.fields.fx = fx;
        assert!(storage::write_settings(&mut settings, None, input).is_err());
    }
    for index in 0..MAX_CUSTOM {
        storage::write_settings(&mut settings, None, editor_input(&format!("预设{index}")))
            .unwrap();
    }
    assert!(storage::write_settings(&mut settings, None, editor_input("超额")).is_err());
    let id = settings.window_presets[0].id.clone();
    assert!(storage::write_settings(&mut settings, Some(&id), editor_input("仍可修改")).is_ok());
}

/// 内置预设对外只暴露 ID 与字段，名称留给 i18n。
#[test]
fn builtins_are_named_by_i18n() {
    let views = all(&[]);
    assert_eq!(views.len(), 4);
    assert!(views
        .iter()
        .all(|preset| preset.builtin && preset.name.is_none()));
}

/// 新内置预设顺序固定：全屏、标准窗口、右上角、右下角；两角统一 320×480。
#[test]
fn builtin_order_and_geometry() {
    let views = all(&[]);
    let ids: Vec<&str> = views.iter().map(|preset| preset.id.as_str()).collect();
    assert_eq!(ids, ["fullscreen", "standard", "topRight", "bottomRight"]);
    let full = &views[0].fields;
    assert!(full.fullscreen);
    assert!(!full.always_on_top);
    let standard = &views[1].fields;
    assert_eq!((standard.width, standard.height), (960., 720.));
    assert_eq!((standard.fx, standard.fy), (0.5, 0.5));
    let top_right = &views[2].fields;
    assert_eq!(
        (
            top_right.width,
            top_right.height,
            top_right.fx,
            top_right.fy
        ),
        (320., 480., 1., 0.)
    );
    assert!(top_right.always_on_top);
    let bottom_right = &views[3].fields;
    assert_eq!(
        (
            bottom_right.width,
            bottom_right.height,
            bottom_right.fx,
            bottom_right.fy
        ),
        (320., 480., 1., 1.)
    );
    assert_eq!(bottom_right.mode, WindowMode::Desktop);
    assert!(!bottom_right.always_on_top);
    assert!(!bottom_right.fullscreen);
}

/// 桌面模式不允许置顶但保留圆角；浮动模式有系统边框时圆角必须失效。
#[test]
fn normalize_removes_conflicting_appearance() {
    let mut desktop = PresetFields {
        mode: WindowMode::Desktop,
        always_on_top: true,
        decorations: true,
        rounded: true,
        position_locked: false,
        size_locked: false,
        width: 100.,
        height: 5000.,
        fx: 2.,
        fy: -1.,
        fullscreen: false,
    };
    normalize(&mut desktop);
    assert!(!desktop.always_on_top);
    assert!(desktop.rounded, "桌面模式不画系统边框，圆角仍然有效");
    assert_eq!(desktop.width, 160.);
    assert_eq!(desktop.height, 2160.);
    assert_eq!((desktop.fx, desktop.fy), (1., 0.));

    let mut bordered = PresetFields {
        mode: WindowMode::Floating,
        decorations: true,
        rounded: true,
        ..desktop.clone()
    };
    normalize(&mut bordered);
    assert!(!bordered.rounded, "有系统边框时圆角不生效");
}

/// 自定义预设追加在内置之后，并保留用户名称。
#[test]
fn custom_presets_follow_builtins() {
    let custom = vec![WindowPreset {
        id: "custom-1".into(),
        name: "我的挂件".into(),
        fields: PresetFields {
            mode: WindowMode::Floating,
            always_on_top: false,
            decorations: false,
            rounded: true,
            position_locked: false,
            size_locked: false,
            width: 300.,
            height: 300.,
            fx: 0.,
            fy: 0.,
            fullscreen: false,
        },
    }];
    let views = all(&custom);
    assert_eq!(views.len(), 5);
    let last = views.last().unwrap();
    assert_eq!(last.id, "custom-1");
    assert_eq!(last.name.as_deref(), Some("我的挂件"));
    assert!(!last.builtin);
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
