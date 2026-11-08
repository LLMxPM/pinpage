// 功能：按目标显示器计算预设位置、全屏尺寸及原生边框占位。
use crate::models::{PinConfig, PresetFields, WindowMode};
use tauri::{AppHandle, Manager};

/// 计算目标显示器上的逻辑尺寸与物理坐标；全屏预设铺满整个显示器，其余按归一化比例定位。
pub(super) fn target_geometry(
    app: &AppHandle,
    previous: &PinConfig,
    fields: &PresetFields,
) -> Option<(f64, f64, i32, i32)> {
    let window = app.get_webview_window(&previous.id);
    let monitor = target_monitor(app, window.as_ref(), previous)?;
    let area = monitor.work_area();
    let scale = if previous.scale > 0.1 {
        previous.scale
    } else {
        1.
    };
    // 全屏尺寸按目标显示器换算，覆盖任务栏区域，并收敛到设置允许的范围内。
    let (width, height, target_scale) = if fields.fullscreen {
        let monitor_scale = monitor.scale_factor().max(0.1);
        (
            (monitor.size().width as f64 / monitor_scale)
                .round()
                .clamp(160., 3840.),
            (monitor.size().height as f64 / monitor_scale)
                .round()
                .clamp(100., 2160.),
            monitor_scale,
        )
    } else {
        (fields.width, fields.height, scale)
    };
    // 全屏以显示器全区域为边界，其他预设以工作区为边界，避开任务栏与停靠区域。
    let (origin, bounds) = if fields.fullscreen {
        (*monitor.position(), *monitor.size())
    } else {
        (area.position, area.size)
    };
    let (frame_width, frame_height) = frame_delta(&window, previous, fields, scale);
    let physical_width = (width * target_scale).round() as i64 + frame_width as i64;
    let physical_height = (height * target_scale).round() as i64 + frame_height as i64;
    let room_width = (bounds.width as i64 - physical_width).max(0);
    let room_height = (bounds.height as i64 - physical_height).max(0);
    Some((
        width,
        height,
        origin.x + (fields.fx.clamp(0., 1.) * room_width as f64).round() as i32,
        origin.y + (fields.fy.clamp(0., 1.) * room_height as f64).round() as i32,
    ))
}

/// 以窗口中心判断所在显示器；窗口未打开时使用保存的坐标，最后回退到主屏。
fn target_monitor(
    app: &AppHandle,
    window: Option<&tauri::WebviewWindow>,
    previous: &PinConfig,
) -> Option<tauri::Monitor> {
    let scale = if previous.scale > 0.1 {
        previous.scale
    } else {
        1.
    };
    let fallback = (
        (previous.width * scale).round() as i32,
        (previous.height * scale).round() as i32,
    );
    let known = match window {
        Some(window) => window.outer_position().ok().map(|point| {
            let size = window
                .outer_size()
                .map(|size| (size.width as i32, size.height as i32))
                .unwrap_or(fallback);
            (point.x, point.y, size)
        }),
        None => previous.x.zip(previous.y).map(|(x, y)| (x, y, fallback)),
    };
    known
        .and_then(|(x, y, (width, height))| {
            app.monitor_from_point(x as f64 + width as f64 / 2., y as f64 + height as f64 / 2.)
                .ok()
                .flatten()
        })
        .or_else(|| app.primary_monitor().ok().flatten())
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
