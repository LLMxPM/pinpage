// 功能：定义内置窗口预设，校验自定义预设，并在归一化位置与屏幕物理坐标之间换算。
use crate::{
    errors::AppMessage,
    i18n,
    models::{PinConfig, PresetFields, PresetView, WindowMode, WindowPreset},
    state::{self, AppState},
};
use tauri::{AppHandle, Manager};

/// 自定义预设数量上限，避免原生菜单与卡片菜单被撑得过长。
pub const MAX_CUSTOM: usize = 20;
/// 预设名称长度上限，与页面名称保持同等可读范围。
pub const MAX_NAME: usize = 40;

/// 内置预设的 ID 与外观组合；顺序即菜单顺序，名称由 `window.presets.<id>` 提供。
///
/// 位置使用归一化比例：0 贴左或贴上、1 贴右或贴下，因此同一预设在任意
/// 分辨率与缩放下都能落在相同的相对位置。
fn builtin_table() -> [(&'static str, PresetFields); 4] {
    [
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
            },
        ),
        (
            "minimal",
            PresetFields {
                mode: WindowMode::Floating,
                always_on_top: true,
                decorations: false,
                rounded: true,
                position_locked: true,
                size_locked: false,
                width: 420.,
                height: 360.,
                fx: 1.,
                fy: 0.,
            },
        ),
        (
            "sticker",
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
            },
        ),
        (
            "board",
            PresetFields {
                mode: WindowMode::Floating,
                always_on_top: false,
                decorations: false,
                rounded: true,
                position_locked: true,
                size_locked: true,
                width: 520.,
                height: 640.,
                fx: 1.,
                fy: 0.5,
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

/// 按 ID 查找预设，内置项优先。
pub fn find(app: &AppHandle, id: &str) -> Option<PresetView> {
    let state = app.state::<AppState>();
    let presets = state.settings.lock().unwrap().window_presets.clone();
    all(&presets).into_iter().find(|preset| preset.id == id)
}

/// 预设显示名：内置项按当前语言翻译，自定义项使用用户输入的名称。
pub fn label(app: &AppHandle, preset: &PresetView) -> String {
    match &preset.name {
        Some(name) => name.clone(),
        None => i18n::text(
            state::locale(app),
            &format!("window.presets.{}", preset.id),
            &Default::default(),
        ),
    }
}

/// 应用预设：只合并外观与几何字段，名称、来源和开关状态保持不变。
pub fn apply(app: &AppHandle, previous: &PinConfig, id: &str) -> Result<(), AppMessage> {
    let preset = find(app, id).ok_or_else(|| AppMessage::new("errors.presetNotFound"))?;
    let mut next = previous.clone();
    next.mode = preset.fields.mode;
    next.always_on_top = preset.fields.always_on_top;
    next.decorations = preset.fields.decorations;
    next.rounded = preset.fields.rounded;
    next.position_locked = preset.fields.position_locked;
    next.size_locked = preset.fields.size_locked;
    next.width = preset.fields.width;
    next.height = preset.fields.height;
    // 位置必须在原生应用前算好：位置锁在应用过程中同步生效，之后再移动会被规则拦回。
    if let Some((x, y)) = target_position(app, previous, &preset.fields) {
        next.x = Some(x);
        next.y = Some(y);
    }
    next.preset = Some(preset.id);
    super::settings::apply(app, previous, next)
}

/// 按归一化比例换算屏幕坐标；同一边框状态下用真实外框差值抵消边框占位。
fn target_position(
    app: &AppHandle,
    previous: &PinConfig,
    fields: &PresetFields,
) -> Option<(i32, i32)> {
    let scale = if previous.scale > 0.1 {
        previous.scale
    } else {
        1.
    };
    let window = app.get_webview_window(&previous.id);
    let (frame_width, frame_height) = frame_delta(&window, previous, fields, scale);
    let width = (fields.width * scale).round() as i64 + frame_width as i64;
    let height = (fields.height * scale).round() as i64 + frame_height as i64;
    // 窗口未打开或尚未定位时没有可用坐标，此时直接按主屏工作区换算。
    let known = match &window {
        Some(window) => window.outer_position().ok().map(|point| (point.x, point.y)),
        None => previous.x.zip(previous.y),
    };
    // 以窗口中心判断所在显示器，窗口跨屏时仍归属面积更大的那一块。
    let monitor = known
        .and_then(|(x, y)| {
            app.monitor_from_point(x as f64 + width as f64 / 2., y as f64 + height as f64 / 2.)
                .ok()
                .flatten()
        })
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let area = monitor.work_area();
    let room_width = (area.size.width as i64 - width).max(0);
    let room_height = (area.size.height as i64 - height).max(0);
    Some((
        area.position.x + (fields.fx.clamp(0., 1.) * room_width as f64).round() as i32,
        area.position.y + (fields.fy.clamp(0., 1.) * room_height as f64).round() as i32,
    ))
}

/// 边框占位：仅在边框状态不变时可从当前窗口量出，切换边框时无法预估，按无边框处理。
fn frame_delta(
    window: &Option<tauri::WebviewWindow>,
    previous: &PinConfig,
    fields: &PresetFields,
    scale: f64,
) -> (u32, u32) {
    let Some(window) = window else {
        return (0, 0);
    };
    let decorated = fields.mode == WindowMode::Floating && fields.decorations;
    if previous.effective_decorations() != decorated {
        return (0, 0);
    }
    let Ok(outer) = window.outer_size() else {
        return (0, 0);
    };
    let inner_width = (previous.width * scale).round().max(0.) as u32;
    let inner_height = (previous.height * scale).round().max(0.) as u32;
    (
        outer.width.saturating_sub(inner_width),
        outer.height.saturating_sub(inner_height),
    )
}

/// 把页面当前外观与几何存为自定义预设，返回新预设 ID。
pub fn save(app: &AppHandle, pin: &PinConfig, name: &str) -> Result<String, AppMessage> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(AppMessage::new("errors.presetNameRange").arg("max", MAX_NAME));
    }
    let fields = snapshot_fields(app, pin)?;
    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    if settings.window_presets.len() >= MAX_CUSTOM {
        return Err(AppMessage::new("errors.presetLimit").arg("max", MAX_CUSTOM));
    }
    let id = format!("custom-{}", uuid::Uuid::new_v4().simple());
    settings.window_presets.push(WindowPreset {
        id: id.clone(),
        name: name.into(),
        fields,
    });
    drop(settings);
    state::schedule_save(app);
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(id)
}

/// 读取页面当前状态并折算为归一化位置；页面关闭时按保存的几何与缩放估算。
fn snapshot_fields(app: &AppHandle, pin: &PinConfig) -> Result<PresetFields, AppMessage> {
    let scale = if pin.scale > 0.1 { pin.scale } else { 1. };
    let (x, y, width, height) = match app.get_webview_window(&pin.id) {
        Some(window) => {
            let position = crate::platform::position(&window)?;
            let size = window
                .outer_size()
                .map_err(|error| AppMessage::new("errors.windowRestoreFailed").detail(error))?;
            (
                position.x,
                position.y,
                size.width as i32,
                size.height as i32,
            )
        }
        None => (
            pin.x.unwrap_or(0),
            pin.y.unwrap_or(0),
            (pin.width * scale).round() as i32,
            (pin.height * scale).round() as i32,
        ),
    };
    let monitor = app
        .monitor_from_point(x as f64 + width as f64 / 2., y as f64 + height as f64 / 2.)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| AppMessage::new("errors.windowRestoreFailed"))?;
    let area = monitor.work_area();
    let room_width = (area.size.width as i32 - width).max(1);
    let room_height = (area.size.height as i32 - height).max(1);
    Ok(PresetFields {
        mode: pin.mode,
        always_on_top: pin.always_on_top,
        decorations: pin.decorations,
        rounded: pin.rounded,
        position_locked: pin.position_locked,
        size_locked: pin.size_locked,
        width: pin.width.round(),
        height: pin.height.round(),
        fx: ((x - area.position.x) as f64 / room_width as f64).clamp(0., 1.),
        fy: ((y - area.position.y) as f64 / room_height as f64).clamp(0., 1.),
    })
}

/// 删除自定义预设，并清空仍引用它的页面标记；内置预设不可删除。
pub fn delete(app: &AppHandle, id: &str) -> Result<(), AppMessage> {
    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    let before = settings.window_presets.len();
    settings.window_presets.retain(|preset| preset.id != id);
    if settings.window_presets.len() == before {
        return Err(AppMessage::new("errors.presetNotFound"));
    }
    for pin in settings.pins.iter_mut() {
        if pin.preset.as_deref() == Some(id) {
            pin.preset = None;
        }
    }
    drop(settings);
    state::schedule_save(app);
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 内置预设对外只暴露 ID 与字段，名称留给 i18n。
    #[test]
    fn builtins_are_named_by_i18n() {
        let views = all(&[]);
        assert_eq!(views.len(), 4);
        assert!(views
            .iter()
            .all(|preset| preset.builtin && preset.name.is_none()));
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
}
