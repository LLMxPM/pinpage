// 功能：登记有界操作并在提交锁外等待导航，保留提交事实与错误。
use super::AutomationState;
use pinpage_protocol::Failure;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct Operation {
    pub id: String,
    pub at: u64,
    pub data: Value,
    pub error: Option<Failure>,
}
/// 过期与容量淘汰只处理已完成项，运行中操作不丢失可查询性。
pub fn accept(
    app: &AppHandle,
    method: String,
    params: Value,
    expected: Option<String>,
) -> Result<Value, Failure> {
    let id = uuid::Uuid::new_v4().to_string();
    let state = app.state::<AutomationState>();
    let mut ops = state.operations.lock().unwrap();
    ops.retain(|op| {
        op.data["done"] != true || pinpage_protocol::now().saturating_sub(op.at) < 15 * 60 * 1000
    });
    while ops.len() >= 100 {
        if let Some(i) = ops.iter().position(|op| op.data["done"] == true) {
            ops.remove(i);
        } else {
            return Err(Failure::new("busy", "操作队列已满"));
        }
    }
    let data = json!({"operationId":id,"phase":"accepted","committed":false,"done":false});
    ops.push(Operation {
        id: id.clone(),
        at: pinpage_protocol::now(),
        data: data.clone(),
        error: None,
    });
    drop(ops);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        set(&app, &id, json!({"phase":"preparing"}), None);
        let execute_app = app.clone();
        let execute_id = id.clone();
        let result = if method == "pages.wait" {
            let page = params["id"].as_str().unwrap_or("");
            let load = params["loadId"].as_str().unwrap_or("");
            super::wait(
                &app,
                page,
                load,
                params["settleMs"].as_u64().unwrap_or(0).min(30000),
            )
            .await
        } else {
            tauri::async_runtime::spawn_blocking(move || {
                super::dispatch::mutate(
                    &execute_app,
                    &method,
                    params,
                    expected.as_deref(),
                    &execute_id,
                )
            })
            .await
            .map_err(|e| Failure::new("executionFailed", e))
            .and_then(|r| r)
        };
        match result {
            Ok(mut result) => {
                result["operationId"] = json!(id);
                set(&app, &id, result.clone(), None);
                let wait = match (result["pageId"].as_str(), result["loadId"].as_str()) {
                    (Some(page), Some(load)) if result["loadState"] == "loading" => {
                        Some((page.to_owned(), load.to_owned()))
                    }
                    _ => None,
                };
                if let Some((page, load)) = wait {
                    match super::wait(&app, &page, &load, 0).await {
                        Ok(_) => {
                            result["loadState"] = json!("loaded");
                            result["phase"] = json!("loaded");
                        }
                        Err(error) => {
                            result["loadState"] = json!("failed");
                            result["done"] = json!(true);
                            set(&app, &id, result, Some(error));
                            return;
                        }
                    }
                }
                if result["loadState"] == "loaded" {
                    result["phase"] = json!("loaded");
                }
                result["done"] = json!(true);
                set(&app, &id, result, None);
            }
            Err(error) => set(
                &app,
                &id,
                json!({"done":true,"phase":"failed"}),
                Some(error),
            ),
        }
    });
    Ok(data)
}
/// 合并每个阶段，失败前已记录的 committed、pageId、revision 保留。
fn set(app: &AppHandle, id: &str, data: Value, error: Option<Failure>) {
    if let Some(op) = app
        .state::<AutomationState>()
        .operations
        .lock()
        .unwrap()
        .iter_mut()
        .find(|op| op.id == id)
    {
        if let Some(fields) = data.as_object() {
            for (k, v) in fields {
                op.data[k] = v.clone();
            }
        }
        op.error = error;
    }
}
/// 过期的 ID 明确返回 operationExpired，不映射成空结果。
pub fn inspect(app: &AppHandle, id: &str) -> Result<Value, Failure> {
    let state = app.state::<AutomationState>();
    let items = state.operations.lock().unwrap();
    let op = items
        .iter()
        .find(|op| op.id == id && pinpage_protocol::now().saturating_sub(op.at) < 15 * 60 * 1000)
        .ok_or_else(|| Failure::new("operationExpired", "操作不存在或已过期"))?;
    match &op.error {
        Some(error) => Err(error.clone().context(op.data.clone())),
        None => Ok(op.data.clone()),
    }
}
/// 在导航前记录持久提交结果，创建窗口失败也能查询到页面身份。
pub fn committed(app: &AppHandle, id: &str, data: Value) {
    set(app, id, data, None);
}
