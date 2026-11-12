// 功能：限制自动化方法和严格输入，在入口鉴权后分派公共业务服务。
use super::{operations, AutomationState};
use crate::{services, state::AppState};
use pinpage_protocol::{Failure, Request};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Id {
    id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    id: String,
    patch: crate::models::WindowPatch,
}
/// 只将有限变更方法送入操作表，内容页不能提交任意方法名。
pub async fn request(app: &AppHandle, request: Request) -> Result<Value, Failure> {
    match request.method.as_str() {
        "doctor" => Ok(
            json!({"appId":app.config().identifier,"appVersion":app.package_info().version.to_string(),"protocolVersion":pinpage_protocol::VERSION,"instanceId":app.state::<AutomationState>().instance_id,"connected":true,"capabilities":pinpage_protocol::capabilities(),"sessionId":request.session_id}),
        ),
        "pages.list" => {
            let pins = app
                .state::<AppState>()
                .settings
                .lock()
                .unwrap()
                .pins
                .clone();
            let pages = pins
                .iter()
                .map(|p| super::inspect(app, &p.id))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"pages":pages}))
        }
        "pages.inspect" => super::inspect(app, &decode::<Id>(request.params)?.id),
        "operations.inspect" => operations::inspect(app, &decode::<Id>(request.params)?.id),
        "capture.target" => super::geometry::target(app, &decode::<Id>(request.params)?.id),
        "debug.start" => super::diagnostics::start(app, request.params).await,
        "debug.logs" => super::diagnostics::logs(app, request.params),
        "debug.stop" => super::diagnostics::stop(app, &decode::<Id>(request.params)?.id),
        "pages.install"
        | "pages.update"
        | "pages.open"
        | "pages.close"
        | "pages.focus"
        | "pages.reload"
        | "pages.wait"
        | pinpage_protocol::WINDOW_SET => {
            if request.method == "pages.update" && request.expected_instance.is_none() {
                return Err(Failure::new(
                    "invalidArgument",
                    "更新必须携带 expectedInstance",
                ));
            }
            operations::accept(
                app,
                request.method,
                request.params,
                request.expected_instance,
            )
        }
        _ => Err(Failure::new("unsupported", "不支持的方法")),
    }
}
/// 对来源和窗口变更做业务层复核，内部工作线程避免同步 UI 创建阻塞。
pub fn mutate(
    app: &AppHandle,
    method: &str,
    params: Value,
    expected: Option<&str>,
    operation: &str,
) -> Result<Value, Failure> {
    match method {
        "pages.install" => services::pages::install(app, decode(params)?, operation),
        "pages.update" => services::pages::update(app, decode(params)?, expected, operation),
        pinpage_protocol::WINDOW_SET => {
            let input: Window = decode(params)?;
            let previous = super::pin(app, &input.id)?;
            let next = crate::windows::editor::merge_window(&previous, &input.patch)
                .map_err(services::source::native)?;
            if next.mode == crate::models::WindowMode::Desktop
                && (next.always_on_top || input.patch.decorations == Some(true))
            {
                return Err(Failure::new(
                    "invalidArgument",
                    "桌面模式不接受置顶或系统边框",
                ));
            }
            crate::windows::editor::update_window(app, &input.id, input.patch, None)
                .map_err(services::source::native)?;
            services::pages::outcome(app, &input.id)
        }
        "pages.open" | "pages.close" | "pages.focus" | "pages.reload" => {
            let id = decode::<Id>(params)?.id;
            let action = method.strip_prefix("pages.").unwrap();
            crate::windows::action(app, &id, action).map_err(services::source::native)?;
            let mut result = services::pages::outcome(app, &id)?;
            if matches!(action, "focus" | "close") {
                result["loadId"] = Value::Null;
                result["loadState"] = json!("notRequested");
            }
            Ok(result)
        }
        _ => Err(Failure::new("unsupported", "不支持的变更")),
    }
}
/// deny_unknown_fields 防止尝试写入 folderRoot、页面 ID 或未来字段。
fn decode<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, Failure> {
    serde_json::from_value(params).map_err(|e| Failure::new("invalidArgument", e))
}
