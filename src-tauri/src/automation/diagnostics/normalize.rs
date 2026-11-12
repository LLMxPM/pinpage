// 功能：把内部 CDP 事件归一化为有界浅层日志，并去除 URL 的敏感分量。
use serde_json::{json, Value};
/// URL 不保留用户名、密码、查询串与片段。
pub fn url(text: &str) -> String {
    if let Ok(mut value) = url::Url::parse(text) {
        let _ = value.set_username("");
        let _ = value.set_password(None);
        value.set_query(None);
        value.set_fragment(None);
        value.to_string().chars().take(2048).collect()
    } else {
        "[invalid URL]".into()
    }
}
/// 只摘取固定字段，不展开对象、不执行 getter，也不保留远端 objectId。
pub fn event(kind: &str, value: &Value) -> Option<Value> {
    match kind {
        "Runtime.consoleAPICalled" => {
            let level = match value["type"].as_str().unwrap_or("log") {
                "error" | "assert" => "error",
                "warning" => "warning",
                "debug" => "debug",
                _ => "info",
            };
            let message = value["args"]
                .as_array()
                .map(|args| {
                    args.iter()
                        .take(20)
                        .map(|a| {
                            a.get("value")
                                .map(|v| {
                                    v.as_str()
                                        .map(str::to_owned)
                                        .unwrap_or_else(|| v.to_string())
                                })
                                .or_else(|| a["description"].as_str().map(str::to_owned))
                                .unwrap_or_else(|| "[object]".into())
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            Some(json!({"source":"console","level":level,"message":message}))
        }
        "Runtime.exceptionThrown" => {
            let details = &value["exceptionDetails"];
            let message = details["exception"]["description"]
                .as_str()
                .or(details["text"].as_str())
                .unwrap_or("脚本异常");
            Some(
                json!({"source":"exception","level":"error","message":redact_stack(message),"url":url(details["url"].as_str().unwrap_or("")),"line":details["lineNumber"],"column":details["columnNumber"]}),
            )
        }
        "Network.loadingFailed" => Some(
            json!({"source":"resource","level":"error","message":value["errorText"].as_str().unwrap_or("资源请求失败"),"cancelled":value["canceled"]}),
        ),
        "Network.responseReceived" if value["response"]["status"].as_u64().unwrap_or(0) >= 400 => {
            Some(
                json!({"source":"resource","level":"error","message":format!("HTTP {}",value["response"]["status"]),"url":url(value["response"]["url"].as_str().unwrap_or(""))}),
            )
        }
        _ => None,
    }
}
/// 堆栈保留行列，URL 中的查询串和凭据逐段移除。
fn redact_stack(message: &str) -> String {
    message
        .split_whitespace()
        .map(|part| {
            if part.starts_with("http:") || part.starts_with("https:") {
                url(part)
            } else {
                part.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secrets_are_removed() {
        assert_eq!(
            url("https://user:pass@example.com/a?token=secret#x"),
            "https://example.com/a"
        );
        let v=event("Runtime.consoleAPICalled",&json!({"type":"log","args":[{"type":"object","objectId":"secret","description":"Object"}]})).unwrap();
        assert!(!v.to_string().contains("secret"));
    }
}
