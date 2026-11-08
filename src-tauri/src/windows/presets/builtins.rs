// 功能：统一定义内置预设、外观约束与提供给前端的预设列表。
use crate::models::{PresetFields, PresetView, WindowMode, WindowPreset};

/// 内置预设的 ID 与外观组合；顺序即菜单顺序，名称由 `window.presets.<id>` 提供。
///
/// 位置使用归一化比例：0 贴左或贴上、1 贴右或贴下，因此同一预设在任意
/// 分辨率与缩放下都能落在相同的相对位置。
fn builtin_table() -> [(&'static str, PresetFields); 4] {
    [
        (
            "fullscreen",
            PresetFields {
                mode: WindowMode::Floating,
                always_on_top: false,
                decorations: false,
                rounded: true,
                position_locked: false,
                size_locked: false,
                width: 960.,
                height: 720.,
                fx: 0.,
                fy: 0.,
                fullscreen: true,
            },
        ),
        (
            "standard",
            PresetFields {
                mode: WindowMode::Floating,
                always_on_top: false,
                decorations: true,
                rounded: false,
                position_locked: false,
                size_locked: false,
                width: 960.,
                height: 720.,
                fx: 0.5,
                fy: 0.5,
                fullscreen: false,
            },
        ),
        (
            "topRight",
            PresetFields {
                mode: WindowMode::Floating,
                always_on_top: true,
                decorations: false,
                rounded: true,
                position_locked: true,
                size_locked: false,
                width: 320.,
                height: 480.,
                fx: 1.,
                fy: 0.,
                fullscreen: false,
            },
        ),
        (
            "bottomRight",
            PresetFields {
                mode: WindowMode::Desktop,
                always_on_top: false,
                decorations: false,
                rounded: true,
                position_locked: true,
                size_locked: true,
                width: 320.,
                height: 480.,
                fx: 1.,
                fy: 1.,
                fullscreen: false,
            },
        ),
    ]
}

/// 归一化外观组合：桌面模式不置顶，有系统边框时圆角不生效，尺寸与比例限制在合法范围。
pub fn normalize(fields: &mut PresetFields) {
    if fields.mode == WindowMode::Desktop {
        fields.always_on_top = false;
    }
    if fields.mode == WindowMode::Floating && fields.decorations {
        fields.rounded = false;
    }
    fields.width = fields.width.clamp(160., 3840.);
    fields.height = fields.height.clamp(100., 2160.);
    fields.fx = fields.fx.clamp(0., 1.);
    fields.fy = fields.fy.clamp(0., 1.);
}

/// 组装内置预设视图，字段在输出前统一归一化。
fn builtin_views() -> Vec<PresetView> {
    builtin_table()
        .into_iter()
        .map(|(id, mut fields)| {
            normalize(&mut fields);
            PresetView {
                id: id.into(),
                name: None,
                builtin: true,
                fields,
            }
        })
        .collect()
}

/// 合并内置与自定义预设；自定义项排在后面，与菜单展示顺序一致。
pub fn all(custom: &[WindowPreset]) -> Vec<PresetView> {
    let mut list = builtin_views();
    list.extend(custom.iter().map(|preset| {
        let mut fields = preset.fields.clone();
        normalize(&mut fields);
        PresetView {
            id: preset.id.clone(),
            name: Some(preset.name.clone()),
            builtin: false,
            fields,
        }
    }));
    list
}
