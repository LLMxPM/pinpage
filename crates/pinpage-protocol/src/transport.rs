// 功能：读写长度前缀消息，并定位当前会话内唯一的应用实例。
use crate::{Failure, Instance, MAX_MESSAGE};
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// 在分配内存前检查长度，read_exact 可正确处理分包与连续消息。
pub async fn read<T: DeserializeOwned>(
    stream: &mut (impl AsyncRead + Unpin),
) -> Result<T, Failure> {
    let length = stream.read_u32_le().await.map_err(connection)? as usize;
    if length == 0 || length > MAX_MESSAGE {
        return Err(Failure::new("invalidArgument", "消息长度超出限制"));
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await.map_err(connection)?;
    serde_json::from_slice(&bytes).map_err(|e| Failure::new("invalidArgument", e))
}
/// 编码后检查预算，再发送完整消息。
pub async fn write(
    stream: &mut (impl AsyncWrite + Unpin),
    value: &impl Serialize,
) -> Result<(), Failure> {
    let bytes = serde_json::to_vec(value).map_err(connection)?;
    if bytes.len() > MAX_MESSAGE {
        return Err(Failure::new("responseTooLarge", "响应超过 1 MiB"));
    }
    stream
        .write_u32_le(bytes.len() as u32)
        .await
        .map_err(connection)?;
    stream.write_all(&bytes).await.map_err(connection)?;
    stream.flush().await.map_err(connection)
}
/// 保留传输失败，不将断开误报成业务回滚。
fn connection(error: impl ToString) -> Failure {
    Failure::new("connectionFailed", error)
}

/// 限制标识字符，禁止用实例参数拼接任意路径。
pub fn descriptor_path(app_id: &str) -> Result<PathBuf, Failure> {
    if app_id.is_empty()
        || app_id.len() > 160
        || !app_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c))
    {
        return Err(Failure::new("invalidArgument", "应用标识不合法"));
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| Failure::new("unsupported", "缺少用户本地目录"))?;
    #[cfg(windows)]
    let session = crate::security::session(std::process::id())?;
    #[cfg(not(windows))]
    let session = 0;
    Ok(PathBuf::from(base)
        .join("PinPage/control")
        .join(app_id)
        .join(format!("{session}/instance.json")))
}
/// 发现描述后仍需在发送凭据前核对管道进程，描述文件本身不证明身份。
pub fn discover(app_id: &str) -> Result<Instance, Failure> {
    let path = descriptor_path(app_id)?;
    let bytes =
        std::fs::read(path).map_err(|_| Failure::new("appNotRunning", "目标 PinPage 未运行"))?;
    if bytes.len() > 16384 {
        return Err(Failure::new("authenticationFailed", "实例描述过大"));
    }
    let value: Instance = serde_json::from_slice(&bytes)
        .map_err(|_| Failure::new("authenticationFailed", "实例描述不合法"))?;
    let expected_pipe = format!(r"\\.\pipe\PinPage-{}", value.instance_id);
    if value.app_id != app_id
        || value.pipe != expected_pipe
        || uuid::Uuid::parse_str(&value.instance_id).is_err()
    {
        return Err(Failure::new("authenticationFailed", "实例描述不匹配"));
    }
    #[cfg(windows)]
    if crate::security::session(std::process::id())? != value.session_id
        || crate::security::session(value.pid).ok() != Some(value.session_id)
    {
        return Err(Failure::new("appNotRunning", "实例进程或登录会话已失效"));
    }
    if !value.enabled {
        return Err(Failure::new("controlDisabled", "PinPage 本机控制已关闭"));
    }
    if value.protocol_version != crate::VERSION {
        return Err(Failure::new(
            "protocolMismatch",
            "请更新 PinPage 以匹配协议",
        ));
    }
    Ok(value)
}
