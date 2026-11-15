// 功能：定义 CLI 与应用共享的有界消息、结果及稳定错误域。
#[cfg(windows)]
pub mod security;
pub mod transport;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const VERSION: u32 = 1;
pub const MAX_MESSAGE: usize = 1024 * 1024;
pub const DEFAULT_APP: &str = "com.pinpage.desktop";
pub const WINDOW_SET: &str = "window.set";
#[cfg(test)]
mod tests;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub protocol_version: u32,
    pub request_id: String,
    pub credential: String,
    pub session_id: u32,
    pub expected_instance: Option<String>,
    pub method: String,
    pub params: Value,
}
impl Request {
    /// 对端会话来自内核；正文会话、凭据、协议与目标期望全部一致后才允许业务分发。
    pub fn authorize(
        &self,
        instance: &Instance,
        actual_session: u32,
        enabled: bool,
    ) -> Result<(), Failure> {
        if actual_session != instance.session_id
            || self.session_id != actual_session
            || self.credential != instance.credential
        {
            return Err(Failure::new("authenticationFailed", "本机身份校验失败"));
        }
        if self.protocol_version != VERSION {
            return Err(Failure::new("protocolMismatch", "协议主版本不匹配"));
        }
        if !enabled {
            return Err(Failure::new("controlDisabled", "PinPage 本机控制已关闭"));
        }
        if self
            .expected_instance
            .as_ref()
            .is_some_and(|id| *id != instance.instance_id)
        {
            return Err(Failure::new("revisionConflict", "应用实例已变化"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}
impl Failure {
    /// 构造机器可判定的错误；本地化只改变 message。
    pub fn new(code: &str, message: impl ToString) -> Self {
        Self {
            code: code.into(),
            message: message.to_string(),
            retryable: false,
            details: None,
        }
    }
    /// 保留原生原因、冲突信息或已提交的操作状态。
    pub fn detail(mut self, value: impl Serialize) -> Self {
        self.details = serde_json::to_value(value).ok();
        self
    }
    /// 追加操作上下文时保留原生原因，避免提交状态覆盖 code/args/detail。
    pub fn context(mut self, value: impl Serialize) -> Self {
        if let Ok(mut context) = serde_json::to_value(value) {
            if self.details.as_ref() == Some(&context) {
                return self;
            }
            if !context.is_object() {
                context = serde_json::json!({"result":context});
            }
            if let Some(cause) = self.details.take() {
                context["cause"] = cause;
            }
            self.details = Some(context);
        }
        self
    }
    /// 将稳定错误域转换为进程退出码。
    pub fn exit_code(&self) -> i32 {
        match self.code.as_str() {
            "invalidArgument" | "invalidSource" => 2,
            "appNotRunning" | "connectionFailed" | "protocolMismatch" => 3,
            "authenticationFailed" | "controlDisabled" => 4,
            "revisionConflict"
            | "clientKeyConflict"
            | "skillTargetConflict"
            | "skillModified"
            | "presetConflict" => 5,
            "timeout" => 6,
            "unsupported" | "captureUnavailable" | "skillIncompatible" => 7,
            "cancelled" => 130,
            _ => 8,
        }
    }
}
impl std::fmt::Display for Failure {
    /// 输出可读信息，不包含握手凭据。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Failure {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub protocol_version: u32,
    pub request_id: String,
    pub instance_id: Option<String>,
    pub ok: bool,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Failure>,
    pub warnings: Vec<String>,
}
impl Response {
    /// 成功与失败使用同一信封，提交后错误仍可携带页面与操作信息。
    pub fn result(
        request_id: String,
        instance_id: Option<String>,
        result: Result<Value, Failure>,
    ) -> Self {
        let (ok, data, error) = match result {
            Ok(data) => (true, data, None),
            Err(error) => (
                false,
                error.details.clone().unwrap_or(Value::Null),
                Some(error),
            ),
        };
        Self {
            protocol_version: VERSION,
            request_id,
            instance_id,
            ok,
            data,
            error,
            warnings: vec![],
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Instance {
    pub app_id: String,
    pub pid: u32,
    pub session_id: u32,
    pub instance_id: String,
    pub pipe: String,
    pub protocol_version: u32,
    pub credential: String,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}
/// 兼容首版描述字段缺省，关闭状态由新版显式发布。
fn enabled_by_default() -> bool {
    true
}

/// 能力与内容页 host_info 分离；只公布实际实现的接口。
pub fn capabilities() -> BTreeMap<String, u32> {
    [
        "pageInstall",
        "pageUpdate",
        "windowControl",
        "windowPresets",
        "pageExport",
        "pageDiagnostics",
        "loadWait",
        "operations",
    ]
    .into_iter()
    .map(|s| (s.into(), 1))
    .collect()
}

/// 统一使用毫秒 UNIX 时间，不依赖用户时区。
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
