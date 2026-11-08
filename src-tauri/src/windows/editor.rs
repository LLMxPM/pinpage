// 功能：局部应用窗口设置、独立保存页面内容，避免旧表单覆盖实时窗口状态。
use crate::{
    errors::AppMessage,
    models::{PinConfig, PinContentInput, WindowChange, WindowPatch},
    state::{self, AppState},
};
use tauri::{AppHandle, Manager};

/// 只覆盖输入中存在的窗口字段，并在真正变化时清除预设标记。
pub fn merge_window(previous: &PinConfig, input: &WindowPatch) -> Result<PinConfig, AppMessage> {
    let mut next = previous.clone();
    macro_rules! field {
        ($($key:ident),*) => {$(if let Some(value) = input.$key { next.$key = value; })*};
    }
    field!(
        mode,
        always_on_top,
        decorations,
        rounded,
        position_locked,
        size_locked,
        width,
        height
    );
    if let Some(value) = input.x {
        next.x = Some(value);
    }
    if let Some(value) = input.y {
        next.y = Some(value);
    }
    if !next.width.is_finite()
        || !next.height.is_finite()
        || !(160. ..=3840.).contains(&next.width)
        || !(100. ..=2160.).contains(&next.height)
    {
        return Err(AppMessage::new("errors.geometryRange")
            .arg("minWidth", 160)
            .arg("maxWidth", 3840)
            .arg("minHeight", 100)
            .arg("maxHeight", 2160));
    }
    if next.mode != previous.mode
        || next.always_on_top != previous.always_on_top
        || next.decorations != previous.decorations
        || next.rounded != previous.rounded
        || next.position_locked != previous.position_locked
        || next.size_locked != previous.size_locked
        || next.width != previous.width
        || next.height != previous.height
        || next.x != previous.x
        || next.y != previous.y
    {
        next.preset = None;
    }
    Ok(next)
}

/// 比较撤销涉及的字段；逻辑尺寸按界面整数精度比较，未涉及的字段不限制。
pub fn matches_expected(pin: &PinConfig, expected: &WindowPatch) -> bool {
    macro_rules! matches { ($($key:ident),*) => {true $(&& expected.$key.is_none_or(|value| value == pin.$key))*}; }
    matches!(
        mode,
        always_on_top,
        decorations,
        rounded,
        position_locked,
        size_locked
    ) && expected
        .width
        .is_none_or(|value| value == pin.width.round())
        && expected
            .height
            .is_none_or(|value| value == pin.height.round())
        && expected.x.is_none_or(|value| Some(value) == pin.x)
        && expected.y.is_none_or(|value| Some(value) == pin.y)
}

/// 在操作锁内获取最新页面，而非依赖前端打开弹窗时的旧快照。
fn current(state: &AppState, id: &str) -> Result<PinConfig, AppMessage> {
    state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))
}

/// 即时应用一次局部修改；保存失败时恢复原生状态和配置，并将最终快照返回前端。
pub fn update_window(
    app: &AppHandle,
    id: &str,
    input: WindowPatch,
    expected: Option<WindowPatch>,
) -> Result<WindowChange, AppMessage> {
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    let previous = current(&state, id)?;
    if expected
        .as_ref()
        .is_some_and(|expected| !matches_expected(&previous, expected))
    {
        return Err(AppMessage::new("errors.windowUndoExpired"));
    }
    let next = merge_window(&previous, &input)?;
    super::settings::apply(app, &previous, next.clone())?;
    let mut actual = current(&state, id)?;
    actual.preset = super::presets::matching_id(app, &actual);
    if let Some(pin) = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter_mut()
        .find(|pin| pin.id == id)
    {
        pin.preset = actual.preset;
    }
    if let Err(error) = state.save() {
        if let Err(rollback) = super::settings::apply(app, &next, previous) {
            state::report(app, rollback);
        }
        return Err(error);
    }
    Ok(WindowChange {
        before: previous,
        after: current(&state, id)?,
    })
}

/// 保存内容时只更新名称与来源；不重新应用窗口外观和几何。
pub fn update_content(app: &AppHandle, id: &str, input: PinContentInput) -> Result<(), AppMessage> {
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(AppMessage::new("errors.nameRange").arg("max", 80));
    }
    let previous = current(&state, id)?;
    let source = input
        .source
        .as_ref()
        .map(|value| crate::source::prepare(app, id, value))
        .transpose()?;
    let mut candidate = previous.clone();
    candidate.name = name.into();
    if let Some(source) = &source {
        candidate.source = source.clone();
    }
    let target = if source.is_some() && app.get_webview_window(id).is_some() {
        Some(super::initial_url(app, &candidate)?)
    } else {
        None
    };
    // 写入时再次取最新条目，仅覆盖内容；期间的移动、缩放回传保持有效。
    if let Some(pin) = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter_mut()
        .find(|pin| pin.id == id)
    {
        pin.name = candidate.name.clone();
        pin.source = candidate.source.clone();
    }
    if let Err(error) = state.save() {
        if let Some(pin) = state
            .settings
            .lock()
            .unwrap()
            .pins
            .iter_mut()
            .find(|pin| pin.id == id)
        {
            pin.name = previous.name;
            pin.source = previous.source;
        }
        return Err(error);
    }
    if let Some(window) = app.get_webview_window(id) {
        if let Err(error) = window.set_title(&super::window_title(app, name)) {
            state::report(
                app,
                AppMessage::new("errors.windowApplyFailed").detail(error),
            );
        }
    }
    if let Some(url) = target {
        if let Err(error) = super::navigate(app, id, url) {
            state::report(app, error);
        }
    }
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(())
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;
