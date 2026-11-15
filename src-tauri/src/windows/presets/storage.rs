// 功能：校验、创建、编辑和删除页面自己的预设，独立保存模板并在失败时回滚。
use super::{validation, MAX_CUSTOM};
use crate::{
    errors::AppMessage,
    models::{PinConfig, PresetFields, PresetInput, WindowPreset},
    state::{self, AppState},
};
use tauri::{AppHandle, Manager};

/// 严格校验 IPC 参数后才归一化外观冲突，不悄悄截断非法尺寸或位置。
fn validate(input: &mut PresetInput) -> Result<(), AppMessage> {
    validation::validate_input(input)?;
    crate::platform::validate_preset(crate::platform::capabilities(), &input.fields)?;
    Ok(())
}

/// 创建或更新页面自己的模板；窗口只清除失效匹配标记，实际外观与几何不会联动修改。
pub(super) fn write_pin(
    pin: &mut PinConfig,
    id: Option<&str>,
    mut input: PresetInput,
) -> Result<String, AppMessage> {
    validate(&mut input)?;
    if let Some(id) = id {
        let preset = pin
            .presets
            .iter_mut()
            .find(|preset| preset.id == id)
            .ok_or_else(|| AppMessage::new("errors.presetNotFound"))?;
        let changed = preset.fields != input.fields;
        preset.name = input.name;
        preset.fields = input.fields;
        if changed && pin.preset.as_deref() == Some(id) {
            pin.preset = None;
        }
        Ok(id.into())
    } else {
        if pin.presets.len() >= MAX_CUSTOM {
            return Err(AppMessage::new("errors.presetLimit").arg("max", MAX_CUSTOM));
        }
        let id = format!("custom-{}", uuid::Uuid::new_v4().simple());
        pin.presets.push(WindowPreset {
            id: id.clone(),
            name: input.name,
            fields: input.fields,
        });
        Ok(id)
    }
}

/// 移除页面自己的模板；删除后窗口内容、开关、外观与几何全部保留。
pub(super) fn delete_pin(pin: &mut PinConfig, id: &str) -> Result<(), AppMessage> {
    let index = pin
        .presets
        .iter()
        .position(|preset| preset.id == id)
        .ok_or_else(|| AppMessage::new("errors.presetNotFound"))?;
    pin.presets.remove(index);
    if pin.preset.as_deref() == Some(id) {
        pin.preset = None;
    }
    Ok(())
}

/// 调用者持有操作锁；落盘失败仅还原目标页面的预设列表与匹配标记。
fn transaction<T>(
    app: &AppHandle,
    pin_id: &str,
    operation: impl FnOnce(&mut PinConfig) -> Result<T, AppMessage>,
) -> Result<T, AppMessage> {
    let state = app.state::<AppState>();
    let before = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == pin_id)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))?;
    let result = operation(
        state
            .settings
            .lock()
            .unwrap()
            .pins
            .iter_mut()
            .find(|pin| pin.id == pin_id)
            .expect("before 已确认页面存在"),
    )?;
    if let Err(error) = state.save() {
        if let Some(pin) = state
            .settings
            .lock()
            .unwrap()
            .pins
            .iter_mut()
            .find(|pin| pin.id == pin_id)
        {
            pin.presets = before.presets;
            pin.preset = before.preset;
        }
        return Err(error);
    }
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(result)
}

/// 在指定页面的模板列表中编辑模板，不把模板变化应用到正在运行的窗口。
pub fn write(
    app: &AppHandle,
    pin_id: &str,
    id: Option<&str>,
    input: PresetInput,
) -> Result<String, AppMessage> {
    transaction(app, pin_id, |pin| write_pin(pin, id, input))
}

/// 指定稳定 ID 创建或更新模板，重试相同请求不会增加重复项，窗口不会随模板编辑而变化。
pub(super) fn put_pin(
    pin: &mut PinConfig,
    id: &str,
    mut input: PresetInput,
) -> Result<String, AppMessage> {
    validation::validate_id(id)?;
    validate(&mut input)?;
    if pin.presets.iter().any(|p| p.id == id) {
        return write_pin(pin, Some(id), input);
    }
    if pin.presets.len() >= MAX_CUSTOM {
        return Err(AppMessage::new("errors.presetLimit").arg("max", MAX_CUSTOM));
    }
    pin.presets.push(WindowPreset {
        id: id.into(),
        name: input.name,
        fields: input.fields,
    });
    Ok(id.into())
}

/// 调用者持有操作锁，在同一个保存事务内提交稳定 ID 的模板。
pub fn put(
    app: &AppHandle,
    pin_id: &str,
    id: &str,
    input: PresetInput,
) -> Result<String, AppMessage> {
    transaction(app, pin_id, |pin| put_pin(pin, id, input))
}

/// 把已生效窗口状态另存或覆盖页面自己的模板，成功后标记当前窗口匹配该模板。
pub fn capture(
    app: &AppHandle,
    pin: &PinConfig,
    name: &str,
    id: &str,
) -> Result<String, AppMessage> {
    let fields = snapshot_fields(app, pin)?;
    transaction(app, &pin.id, |saved| {
        let id = put_pin(
            saved,
            id,
            PresetInput {
                name: name.into(),
                fields,
            },
        )?;
        saved.preset = Some(id.clone());
        Ok(id)
    })
}

/// 把已生效窗口状态另存或覆盖页面自己的模板，成功后标记当前窗口匹配该模板。
pub fn save(
    app: &AppHandle,
    pin: &PinConfig,
    name: &str,
    id: Option<&str>,
) -> Result<String, AppMessage> {
    let fields = snapshot_fields(app, pin)?;
    let pin_id = pin.id.clone();
    transaction(app, &pin_id, |saved| {
        let id = write_pin(
            saved,
            id,
            PresetInput {
                name: name.into(),
                fields,
            },
        )?;
        saved.preset = Some(id.clone());
        Ok(id)
    })
}

/// 仅移除指定页面的模板，失败时配置与界面保持原状。
pub fn delete(app: &AppHandle, pin_id: &str, id: &str) -> Result<(), AppMessage> {
    transaction(app, pin_id, |pin| delete_pin(pin, id))
}

/// 读取页面当前状态并折算为归一化位置；页面关闭时按保存的几何与缩放估算。
pub fn snapshot_fields(app: &AppHandle, pin: &PinConfig) -> Result<PresetFields, AppMessage> {
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
        transparent_background: pin.transparent_background,
        position_locked: pin.position_locked,
        size_locked: pin.size_locked,
        width: logical_width,
        height: logical_height,
        fx: ((x - area.position.x) as f64 / room_width as f64).clamp(0., 1.),
        fy: ((y - area.position.y) as f64 / room_height as f64).clamp(0., 1.),
        fullscreen: pin
            .preset
            .as_deref()
            .and_then(|id| super::find(pin, id))
            .is_some_and(|preset| preset.fields.fullscreen),
    })
}
