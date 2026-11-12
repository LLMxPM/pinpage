// 功能：每次请求重新核对实例与管道服务端进程，不缓存凭据或管理配置。
use crate::args::Cli;
use pinpage_protocol::{Failure, Request, Response};
use serde_json::Value;

/// 传输有总截止时间，断连不意味着已受理操作被撤销。
#[cfg(windows)]
pub async fn call(cli: &Cli, method: &str, params: Value) -> Result<Response, Failure> {
    use pinpage_protocol::{security, transport};
    use std::os::windows::io::AsRawHandle;
    let instance = transport::discover(&cli.instance)?;
    if cli
        .expected_instance
        .as_ref()
        .is_some_and(|id| *id != instance.instance_id)
    {
        return Err(Failure::new("revisionConflict", "目标应用实例已变化"));
    }
    let timeout = std::time::Duration::from_millis(cli.timeout_ms);
    tokio::time::timeout(timeout, async {
        let mut stream = loop {
            match security::connect(&instance.pipe) {
                Ok(stream) => break stream,
                Err(e) if e.raw_os_error() == Some(231) => {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await
                }
                Err(e) => return Err(Failure::new("appNotRunning", e)),
            }
        };
        if unsafe { security::server_pid(stream.as_raw_handle()) }? != instance.pid {
            return Err(Failure::new("authenticationFailed", "管道服务端进程不匹配"));
        }
        let request = Request {
            protocol_version: pinpage_protocol::VERSION,
            request_id: uuid::Uuid::new_v4().to_string(),
            credential: instance.credential,
            session_id: security::session(std::process::id())?,
            expected_instance: cli.expected_instance.clone(),
            method: method.into(),
            params,
        };
        transport::write(&mut stream, &request).await?;
        let response: Response = transport::read(&mut stream).await?;
        if response.request_id != request.request_id
            || response.instance_id.as_ref() != Some(&instance.instance_id)
            || response.protocol_version != pinpage_protocol::VERSION
        {
            return Err(Failure::new("authenticationFailed", "响应身份不匹配"));
        }
        transport::write(
            &mut stream,
            &serde_json::json!({"requestId":request.request_id,"received":true}),
        )
        .await?;
        // 等服务端确认读完响应后断开，防止新连接撞上同一管道实例的复用边界。
        use tokio::io::AsyncReadExt;
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), stream.read_u8()).await;
        Ok(response)
    })
    .await
    .map_err(|_| Failure::new("timeout", "连接等待超时；请查询操作状态"))?
}
/// 非 Windows 构建保留帮助与离线 skill 功能。
#[cfg(not(windows))]
pub async fn call(_: &Cli, _: &str, _: Value) -> Result<Response, Failure> {
    Err(Failure::new("unsupported", "当前平台不支持本机控制"))
}

/// 内部组合流程继续传播服务端稳定错误。
pub async fn data(cli: &Cli, method: &str, params: Value) -> Result<Value, Failure> {
    let response = call(cli, method, params).await?;
    if let Some(error) = response.error {
        Err(error.context(response.data))
    } else {
        Ok(response.data)
    }
}
