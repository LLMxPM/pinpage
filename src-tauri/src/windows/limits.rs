// 功能：集中定义页面窗口尺寸的合法范围，供设置提交、窗口创建与预设共用。
use crate::errors::AppMessage;

pub const MIN_WIDTH: f64 = 80.;
pub const MAX_WIDTH: f64 = 8192.;
pub const MIN_HEIGHT: f64 = 60.;
pub const MAX_HEIGHT: f64 = 8192.;

/// 尺寸必须是有限值且落在统一范围内；具体错误码由调用方决定。
pub fn contains(width: f64, height: f64) -> bool {
    width.is_finite()
        && height.is_finite()
        && (MIN_WIDTH..=MAX_WIDTH).contains(&width)
        && (MIN_HEIGHT..=MAX_HEIGHT).contains(&height)
}

/// 校验窗口设置提交的尺寸，越界时返回带上下限的几何错误。
pub fn check(width: f64, height: f64) -> Result<(), AppMessage> {
    if contains(width, height) {
        return Ok(());
    }
    Err(AppMessage::new("errors.geometryRange")
        .arg("minWidth", MIN_WIDTH)
        .arg("maxWidth", MAX_WIDTH)
        .arg("minHeight", MIN_HEIGHT)
        .arg("maxHeight", MAX_HEIGHT))
}

/// 将尺寸夹到合法范围，用于窗口创建和预设归一化。
pub fn clamp_size(width: f64, height: f64) -> (f64, f64) {
    (
        width.clamp(MIN_WIDTH, MAX_WIDTH),
        height.clamp(MIN_HEIGHT, MAX_HEIGHT),
    )
}
