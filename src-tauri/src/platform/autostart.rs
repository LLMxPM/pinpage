// 功能：封装当前用户的 Windows 登录自启注册，系统状态是开关的唯一来源。
use crate::errors::AppMessage;
use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub supported: bool,
    pub enabled: bool,
    pub error: Option<AppMessage>,
}

/// 显式限定当前用户；系统启动项不会随配置迁移，验证实例使用独立名称。
#[cfg(windows)]
fn registration(app: &AppHandle) -> Result<auto_launch::AutoLaunch, AppMessage> {
    let name = if app.config().identifier == "com.pinpage.desktop" {
        "PinPage"
    } else {
        &app.config().identifier
    };
    let executable = std::env::current_exe()
        .map_err(|error| AppMessage::new("errors.autostartReadFailed").detail(error))?;
    // auto-launch 在 Windows 中直接拼接命令行，必须自行引用含空格或中文的完整路径。
    Ok(auto_launch::AutoLaunch::new(
        name,
        &format!("\"{}\"", executable.display()),
        auto_launch::WindowsEnableMode::CurrentUser,
        &["--tray"],
    ))
}

/// 读取失败仅禁用自启开关，不妨碍工作台读取页面和其他设置。
pub fn status(app: &AppHandle) -> Status {
    match is_enabled(app) {
        Ok(enabled) => Status {
            supported: cfg!(windows),
            enabled,
            error: None,
        },
        Err(error) => Status {
            supported: cfg!(windows),
            enabled: false,
            error: Some(error),
        },
    }
}

/// 每次保存前读取系统注册，避免使用配置副本覆盖系统外部的修改。
pub fn is_enabled(app: &AppHandle) -> Result<bool, AppMessage> {
    #[cfg(windows)]
    return registration(app)?
        .is_enabled()
        .map_err(|error| AppMessage::new("errors.autostartReadFailed").detail(error));
    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(false)
    }
}

/// 修改当前用户的自启项；关闭已关闭的项目不重复删除，不需要管理员权限。
pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), AppMessage> {
    #[cfg(windows)]
    {
        let manager = registration(app)?;
        let result = if enabled {
            manager.enable()
        } else if is_enabled(app)? {
            manager.disable()
        } else {
            return Ok(());
        };
        result.map_err(|error| AppMessage::new("errors.autostartWriteFailed").detail(error))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        if enabled {
            Err(AppMessage::new("errors.autostartUnsupported"))
        } else {
            Ok(())
        }
    }
}
