// 功能：校验单页窗口设置，实时应用并在失败时回滚原生状态。
use crate::{
    models::{PinConfig, PinSettingsInput},
    state::{self, AppState},
};
use tauri::{AppHandle, Manager};

/// 校验几何输入与名称，复制旧配置以兼容已有来源和开启状态。
pub fn merge(pin: &PinConfig, input: PinSettingsInput) -> Result<PinConfig, String> {
    if !input.width.is_finite()
        || !input.height.is_finite()
        || !(160. ..=3840.).contains(&input.width)
        || !(100. ..=2160.).contains(&input.height)
    {
        return Err("宽度应为 160～3840，高度应为 100～2160".into());
    }
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("页面名称应为 1～80 个字符".into());
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
pub(super) fn apply(app: &AppHandle, previous: &PinConfig, next: PinConfig) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&next.id) {
        if let Err(error) = crate::platform::apply(&window, &next) {
            if let Err(rollback) = crate::platform::apply(&window, previous) {
                state::report(app, format!("窗口设置回滚失败：{rollback}"));
            }
            return Err(error);
        }
        window
            .set_title(&format!("{} · PinPage", next.name))
            .map_err(|e| e.to_string())?;
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
    crate::tray::refresh(app);
    Ok(())
}

/// 串行处理管理界面的设置提交，防止并发开关与原生样式切换相互覆盖。
pub fn update(app: &AppHandle, id: &str, input: PinSettingsInput) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    let previous = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .cloned()
        .ok_or("页面记录不存在")?;
    let next = merge(&previous, input)?;
    apply(app, &previous, next)
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
