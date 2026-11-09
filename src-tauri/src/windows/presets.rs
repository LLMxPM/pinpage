// 功能：定义内置窗口预设，校验自定义预设，并在归一化位置与屏幕物理坐标之间换算。
use crate::{
    errors::AppMessage,
    i18n,
    models::{PinConfig, PresetView},
    state::{self, AppState},
};
use tauri::{AppHandle, Manager};

/// 自定义预设数量上限，避免原生菜单与卡片菜单被撑得过长。
pub const MAX_CUSTOM: usize = 20;
/// 预设名称长度上限，与页面名称保持同等可读范围。
pub const MAX_NAME: usize = 40;

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
    next.transparent_background = preset.fields.transparent_background;
    next.position_locked = preset.fields.position_locked;
    next.size_locked = preset.fields.size_locked;
    next.width = preset.fields.width;
    next.height = preset.fields.height;
    // 位置与全屏尺寸必须在原生应用前算好：位置锁在应用过程中同步生效，之后再移动会被规则拦回。
    if let Some((width, height, x, y)) = target_geometry(app, previous, &preset.fields) {
        next.width = width;
        next.height = height;
        next.x = Some(x);
        next.y = Some(y);
    }
    next.preset = Some(preset.id);
    super::settings::apply(app, previous, next.clone())?;
    let actual = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == next.id)
        .cloned();
    if let Some(actual) = actual {
        let marker = matching_id(app, &actual);
        if let Some(pin) = app
            .state::<AppState>()
            .settings
            .lock()
            .unwrap()
            .pins
            .iter_mut()
            .find(|pin| pin.id == next.id)
        {
            pin.preset = marker;
        }
    }
    if let Err(error) = app.state::<AppState>().save() {
        if let Err(rollback) = super::settings::apply(app, &next, previous.clone()) {
            state::report(app, rollback);
        }
        return Err(error);
    }
    Ok(())
}

/// 按实际外观和目标显示器几何匹配模板，允许系统像素取整；撤销也可恢复匹配状态。
pub fn matching_id(app: &AppHandle, pin: &PinConfig) -> Option<String> {
    let custom = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .window_presets
        .clone();
    let mut presets = all(&custom);
    presets.sort_by_key(|preset| pin.preset.as_deref() != Some(preset.id.as_str()));
    presets
        .into_iter()
        .find(|preset| {
            let fields = &preset.fields;
            let floating = pin.mode == crate::models::WindowMode::Floating;
            let decorations = floating && fields.decorations;
            if fields.mode != pin.mode
                || (floating && fields.always_on_top) != (floating && pin.always_on_top)
                || decorations != pin.effective_decorations()
                || fields.transparent_background != pin.transparent_background
                || (!decorations && fields.rounded) != (!pin.effective_decorations() && pin.rounded)
                || fields.position_locked != pin.position_locked
                || fields.size_locked != pin.size_locked
            {
                return false;
            }
            target_geometry(app, pin, fields).is_some_and(|(width, height, x, y)| {
                (width - pin.width).abs() <= 1.
                    && (height - pin.height).abs() <= 1.
                    && pin
                        .x
                        .is_some_and(|actual| (actual as i64 - x as i64).abs() <= 1)
                    && pin
                        .y
                        .is_some_and(|actual| (actual as i64 - y as i64).abs() <= 1)
            })
        })
        .map(|preset| preset.id)
}

mod builtins;
mod geometry;
mod storage;
pub use builtins::{all, normalize};
use geometry::target_geometry;
pub use storage::{delete, save, snapshot_fields, write};

/// 随包默认布局只初始化新页面，不写入全局预设库；平台回退后再按主屏计算归一化位置。
pub fn initialize(app: &AppHandle, pin: &mut PinConfig, mut fields: crate::models::PresetFields) {
    initialize_fields(pin, &mut fields, crate::platform::capabilities());
    if let Some(monitor) = app.primary_monitor().ok().flatten() {
        pin.scale = monitor.scale_factor();
    }
    if let Some((width, height, x, y)) = target_geometry(app, pin, &fields) {
        pin.width = width;
        pin.height = height;
        pin.x = Some(x);
        pin.y = Some(y);
    }
}

/// 把已校验包内布局合并为初始状态；不支持的能力安全回退，没有手势时保留系统边框。
fn initialize_fields(
    pin: &mut PinConfig,
    fields: &mut crate::models::PresetFields,
    capabilities: crate::models::PlatformCapabilities,
) {
    normalize(fields);
    pin.mode = fields.mode;
    pin.always_on_top = fields.always_on_top;
    pin.decorations = fields.decorations;
    pin.rounded = fields.rounded;
    pin.transparent_background = fields.transparent_background;
    pin.position_locked = fields.position_locked;
    pin.size_locked = fields.size_locked;
    pin.width = fields.width;
    pin.height = fields.height;
    pin.preset = None;
    crate::platform::fallback_pin(capabilities, pin);
    if capabilities.drag_modifier.is_none() {
        pin.decorations = true;
    }
    fields.mode = pin.mode;
    fields.decorations = pin.decorations;
    fields.rounded = pin.rounded;
    fields.transparent_background = pin.transparent_background;
    fields.position_locked = pin.position_locked;
    fields.size_locked = pin.size_locked;
}

#[cfg(test)]
#[path = "presets/tests.rs"]
mod tests;
