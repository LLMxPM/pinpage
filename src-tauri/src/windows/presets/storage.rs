// 功能：校验、创建、编辑和删除自定义预设，独立保存模板并在失败时回滚。
use super::{normalize, MAX_CUSTOM, MAX_NAME};
use crate::{
    errors::AppMessage,
    models::{PinConfig, PresetFields, PresetInput, Settings, WindowPreset},
    state::{self, AppState},
};
use tauri::{AppHandle, Manager};

/// 严格校验 IPC 参数后才归一化外观冲突，不悄悄截断非法尺寸或位置。
fn validate(input: &mut PresetInput) -> Result<(), AppMessage> {
    input.name = input.name.trim().into();
    if input.name.is_empty() || input.name.chars().count() > MAX_NAME {
        return Err(AppMessage::new("errors.presetNameRange").arg("max", MAX_NAME));
    }
    let fields = &mut input.fields;
    if !crate::windows::limits::contains(fields.width, fields.height)
        || !fields.fx.is_finite()
        || !fields.fy.is_finite()
        || !(0. ..=1.).contains(&fields.fx)
        || !(0. ..=1.).contains(&fields.fy)
    {
        return Err(AppMessage::new("errors.presetFieldsInvalid"));
    }
    normalize(fields);
    Ok(())
}

/// 创建或更新模板；窗口只清除失效匹配标记，实际外观与几何不会联动修改。
pub(super) fn write_settings(
    settings: &mut Settings,
    id: Option<&str>,
    mut input: PresetInput,
) -> Result<String, AppMessage> {
    validate(&mut input)?;
    if let Some(id) = id {
        if super::all(&[]).iter().any(|preset| preset.id == id) {
            return Err(AppMessage::new("errors.presetReadonly"));
        }
        let preset = settings
            .window_presets
            .iter_mut()
            .find(|preset| preset.id == id)
            .ok_or_else(|| AppMessage::new("errors.presetNotFound"))?;
        let changed = preset.fields != input.fields;
        preset.name = input.name;
        preset.fields = input.fields;
        if changed {
            clear_markers(settings, id);
        }
        Ok(id.into())
    } else {
        if settings.window_presets.len() >= MAX_CUSTOM {
            return Err(AppMessage::new("errors.presetLimit").arg("max", MAX_CUSTOM));
        }
        let id = format!("custom-{}", uuid::Uuid::new_v4().simple());
        settings.window_presets.push(WindowPreset {
            id: id.clone(),
            name: input.name,
            fields: input.fields,
        });
        Ok(id)
    }
}

/// 移除模板前校验身份；删除后保留窗口内容、开关、外观与几何。
pub(super) fn delete_settings(settings: &mut Settings, id: &str) -> Result<(), AppMessage> {
    if super::all(&[]).iter().any(|preset| preset.id == id) {
        return Err(AppMessage::new("errors.presetReadonly"));
    }
    let index = settings
        .window_presets
        .iter()
        .position(|preset| preset.id == id)
        .ok_or_else(|| AppMessage::new("errors.presetNotFound"))?;
    settings.window_presets.remove(index);
    clear_markers(settings, id);
    Ok(())
}

/// 只有模板参数变化或删除才清除标记，重命名不会影响匹配关系。
fn clear_markers(settings: &mut Settings, id: &str) {
    for pin in &mut settings.pins {
        if pin.preset.as_deref() == Some(id) {
            pin.preset = None;
        }
    }
}

/// 调用者持有操作锁；落盘失败仅还原预设与未发生几何变化的匹配标记。
fn transaction<T>(
    app: &AppHandle,
    operation: impl FnOnce(&mut Settings) -> Result<T, AppMessage>,
) -> Result<T, AppMessage> {
    let state = app.state::<AppState>();
    let before = state.settings.lock().unwrap().clone();
    let result = operation(&mut state.settings.lock().unwrap())?;
    if let Err(error) = state.save() {
        let mut settings = state.settings.lock().unwrap();
        settings.window_presets = before.window_presets;
        for pin in &mut settings.pins {
            if let Some(old) = before.pins.iter().find(|old| old.id == pin.id) {
                pin.preset = if (pin.width, pin.height, pin.x, pin.y)
                    == (old.width, old.height, old.x, old.y)
                {
                    old.preset.clone()
                } else {
                    None
                };
            }
        }
        return Err(error);
    }
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(result)
}

/// 从管理器编辑模板，不把模板变化应用到任何正在运行的页面。
pub fn write(app: &AppHandle, id: Option<&str>, input: PresetInput) -> Result<String, AppMessage> {
    transaction(app, |settings| write_settings(settings, id, input))
}

/// 把已生效窗口状态另存或覆盖自定义模板，成功后标记当前窗口匹配该模板。
pub fn save(
    app: &AppHandle,
    pin: &PinConfig,
    name: &str,
    id: Option<&str>,
) -> Result<String, AppMessage> {
    let mut fields = snapshot_fields(app, pin)?;
    fields.fullscreen = pin
        .preset
        .as_deref()
        .and_then(|id| super::find(app, id))
        .is_some_and(|preset| preset.fields.fullscreen);
    transaction(app, |settings| {
        let id = write_settings(
            settings,
            id,
            PresetInput {
                name: name.into(),
                fields,
            },
        )?;
        if let Some(saved) = settings.pins.iter_mut().find(|saved| saved.id == pin.id) {
            saved.preset = Some(id.clone());
        }
        Ok(id)
    })
}

/// 删除仅移除模板，失败时配置与界面保持原状。
pub fn delete(app: &AppHandle, id: &str) -> Result<(), AppMessage> {
    transaction(app, |settings| delete_settings(settings, id))
}

/// 读取页面当前状态并折算为归一化位置；页面关闭时按保存的几何与缩放估算。
fn snapshot_fields(app: &AppHandle, pin: &PinConfig) -> Result<PresetFields, AppMessage> {
    let scale = if pin.scale > 0.1 { pin.scale } else { 1. };
    let (x, y, width, height, logical_width, logical_height) = match app.get_webview_window(&pin.id)
    {
        Some(window) => {
            let position = crate::platform::position(&window)?;
            let size = window
                .outer_size()
                .map_err(|error| AppMessage::new("errors.windowRestoreFailed").detail(error))?;
            let inner = window
                .inner_size()
                .map_err(|error| AppMessage::new("errors.windowRestoreFailed").detail(error))?;
            let current_scale = window
                .scale_factor()
                .map_err(|error| AppMessage::new("errors.windowRestoreFailed").detail(error))?
                .max(0.1);
            (
                position.x,
                position.y,
                size.width as i32,
                size.height as i32,
                (inner.width as f64 / current_scale).round(),
                (inner.height as f64 / current_scale).round(),
            )
        }
        None => (
            pin.x.unwrap_or(0),
            pin.y.unwrap_or(0),
            (pin.width * scale).round() as i32,
            (pin.height * scale).round() as i32,
            pin.width.round(),
            pin.height.round(),
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
        width: logical_width,
        height: logical_height,
        fx: ((x - area.position.x) as f64 / room_width as f64).clamp(0., 1.),
        fy: ((y - area.position.y) as f64 / room_height as f64).clamp(0., 1.),
        fullscreen: false,
    })
}
