// 功能：校验单页窗口设置，实时应用并在失败时回滚原生状态。
use crate::{
    errors::AppMessage,
    models::{PinConfig, PinSettingsInput},
    state::{self, AppState},
};
use tauri::{AppHandle, Manager};

/// 校验几何输入与名称，复制旧配置以兼容已有来源和开启状态。
pub fn merge(pin: &PinConfig, input: &PinSettingsInput) -> Result<PinConfig, AppMessage> {
    if !input.width.is_finite()
        || !input.height.is_finite()
        || !(160. ..=3840.).contains(&input.width)
        || !(100. ..=2160.).contains(&input.height)
    {
        return Err(AppMessage::new("errors.geometryRange")
            .arg("minWidth", 160)
            .arg("maxWidth", 3840)
            .arg("minHeight", 100)
            .arg("maxHeight", 2160));
    }
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(AppMessage::new("errors.nameRange").arg("max", 80));
    }
    let mut next = pin.clone();
    next.name = name.into();
    next.mode = input.mode;
    next.always_on_top = input.always_on_top;
    next.decorations = input.decorations;
    next.rounded = input.rounded;
    next.position_locked = input.position_locked;
    next.size_locked = input.size_locked;
    next.width = input.width;
    next.height = input.height;
    next.x = Some(input.x);
    next.y = Some(input.y);
    Ok(next)
}

/// 保存通过校验的新设置；原生应用失败时还原原设置，避免界面与实际模式脱节。
pub(super) fn apply(
    app: &AppHandle,
    previous: &PinConfig,
    next: PinConfig,
) -> Result<(), AppMessage> {
    if let Some(window) = app.get_webview_window(&next.id) {
        if let Err(error) = crate::platform::apply(&window, &next) {
            if let Err(rollback) = crate::platform::apply(&window, previous) {
                state::report(
                    app,
                    crate::i18n::caused_by(
                        AppMessage::new("errors.rollbackFailed"),
                        state::locale(app),
                        &rollback,
                    ),
                );
            }
            return Err(error);
        }
        window
            .set_title(&super::window_title(app, &next.name))
            .map_err(|error| AppMessage::new("errors.windowApplyFailed").detail(error))?;
    }
    let state = app.state::<AppState>();
    if let Some(saved) = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter_mut()
        .find(|pin| pin.id == next.id)
    {
        *saved = next.clone();
    }
    if let Some(window) = app.get_webview_window(&next.id) {
        super::events::capture(&window);
    }
    state::schedule_save(app);
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(())
}

/// 串行处理管理界面的设置提交，防止并发开关与原生样式切换相互覆盖。
///
/// 提交里带来源时先校验外观，再兑换来源，最后才落盘并让已打开的窗口指向新地址；
/// 任一步失败都不会写入半套配置，兑换失败时页面继续沿用原来源。
pub fn update(app: &AppHandle, id: &str, input: PinSettingsInput) -> Result<(), AppMessage> {
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    let previous = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))?;
    let mut next = merge(&previous, &input)?;
    let refreshed = match &input.source {
        Some(value) => {
            next.source = crate::source::prepare(app, id, value)?;
            true
        }
        None => false,
    };
    // 先算出新来源的首屏地址：导入内容缺失等问题应在落盘前暴露，且关闭的窗口无需导航。
    let target = if refreshed && app.get_webview_window(id).is_some() {
        Some(super::initial_url(app, &next)?)
    } else {
        None
    };
    apply(app, &previous, next.clone())?;
    if let Some(url) = target {
        if let Err(error) = super::navigate(app, id, url) {
            // 配置已保存，导航失败通常是窗口刚被关闭；重新开启即可加载新来源。
            state::report(app, error);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
