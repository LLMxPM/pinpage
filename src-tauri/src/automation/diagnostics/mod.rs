// 功能：管理按页诊断会话和有界日志，不持久化正文或向内容页开放读取。
#[cfg(windows)]
mod native;
mod normalize;
use super::AutomationState;
use pinpage_protocol::Failure;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use tauri::{AppHandle, Manager};

pub struct Session {
    pub id: String,
    pub page: String,
    pub at: u64,
    pub expires: u64,
    pub records: VecDeque<Value>,
    pub bytes: usize,
    pub next: u64,
    pub dropped: u64,
    pub timer: Option<tauri::async_runtime::JoinHandle<()>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    id: String,
    reload: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Logs {
    id: String,
    cursor: u64,
    limit: u32,
    level: Option<String>,
}
/// 每页一个会话；接入全部事件并确认 domain 启用后，才允许显式刷新。
pub async fn start(app: &AppHandle, params: Value) -> Result<Value, Failure> {
    let input: Start = serde_json::from_value(params).map_err(invalid)?;
    super::pin(app, &input.id)?;
    let state = app.state::<AutomationState>();
    if let Some(session) = state
        .diagnostics
        .lock()
        .unwrap()
        .values()
        .find(|s| s.page == input.id && s.expires > pinpage_protocol::now())
    {
        return Ok(summary(session, "afterAttach", None));
    }
    close_page(app, &input.id);
    let window = app
        .get_webview_window(&input.id)
        .ok_or_else(|| Failure::new("pageClosed", "诊断需要已开启页面"))?;
    let id = uuid::Uuid::new_v4().to_string();
    let at = pinpage_protocol::now();
    let session = Session {
        id: id.clone(),
        page: input.id.clone(),
        at,
        expires: at + 15 * 60 * 1000,
        records: VecDeque::new(),
        bytes: 0,
        next: 1,
        dropped: 0,
        timer: None,
    };
    state
        .diagnostics
        .lock()
        .unwrap()
        .insert(id.clone(), session);
    #[cfg(windows)]
    if let Err(error) = native::attach(&window, &id).await {
        let _ = stop(app, &id);
        return Err(error);
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        let _ = stop(app, &id);
        Err(Failure::new("unsupported", "当前平台不支持页面诊断"))
    }
    #[cfg(windows)]
    {
        let expiry_app = app.clone();
        let expiry_id = id.clone();
        let timer = tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(15 * 60)).await;
            let _ = stop(&expiry_app, &expiry_id);
        });
        if let Some(session) = state.diagnostics.lock().unwrap().get_mut(&id) {
            session.timer = Some(timer);
        } else {
            timer.abort();
        }
        if input.reload {
            let reload_app = app.clone();
            let page = input.id.clone();
            tauri::async_runtime::spawn_blocking(move || {
                crate::windows::action(&reload_app, &page, "reload")
            })
            .await
            .map_err(invalid)?
            .map_err(crate::services::source::native)?;
        }
        let load = state
            .loads
            .lock()
            .unwrap()
            .get(&input.id)
            .map(|l| l.id.clone());
        let sessions = state.diagnostics.lock().unwrap();
        Ok(summary(
            sessions.get(&id).ok_or_else(|| invalid("会话已关闭"))?,
            if input.reload {
                "reload"
            } else {
                "afterAttach"
            },
            load,
        ))
    }
}
/// 返回接入时间和有效期，不承诺回溯首屏历史。
fn summary(s: &Session, coverage: &str, load: Option<String>) -> Value {
    json!({"sessionId":s.id,"pageId":s.page,"coverage":coverage,"attachedAt":s.at,"expiresAt":s.expires,"loadId":load})
}
/// 记录 UTF-8 安全截断，预算先达到 1000 条或 2 MiB 即淘汰。
pub fn push(app: &AppHandle, session: &str, mut record: Value) {
    let state = app.state::<AutomationState>();
    let mut sessions = state.diagnostics.lock().unwrap();
    let Some(s) = sessions.get_mut(session) else {
        return;
    };
    if pinpage_protocol::now() >= s.expires {
        return;
    }
    if let Some(message) = record["message"].as_str() {
        if message.len() > 12000 {
            let mut end = 12000;
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            let text = message[..end].to_owned();
            record["message"] = json!(text);
            record["truncated"] = json!(true);
        }
    }
    record["sequence"] = json!(s.next);
    record["timestamp"] = json!(pinpage_protocol::now());
    record["pageId"] = json!(s.page);
    record["loadId"] = json!(state
        .loads
        .lock()
        .unwrap()
        .get(&s.page)
        .map(|l| l.id.clone()));
    let mut size = serde_json::to_vec(&record).map_or(0, |b| b.len());
    while size > 16 * 1024 {
        let Some(message) = record["message"].as_str() else {
            return;
        };
        if message.is_empty() {
            return;
        }
        let mut end = message.len() / 2;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        let shortened = message[..end].to_owned();
        record["message"] = json!(shortened);
        record["truncated"] = json!(true);
        size = serde_json::to_vec(&record).map_or(0, |bytes| bytes.len());
    }
    s.next += 1;
    s.bytes += size;
    s.records.push_back(record);
    while s.records.len() > 1000 || s.bytes > 2 * 1024 * 1024 {
        if let Some(old) = s.records.pop_front() {
            s.bytes = s
                .bytes
                .saturating_sub(serde_json::to_vec(&old).unwrap().len());
            s.dropped += 1;
        }
    }
}
/// 宿主事件复用同一会话，不在无会话时分配日志缓存。
pub fn host(app: &AppHandle, page: &str, message: &str, level: &str) {
    let id = app
        .state::<AutomationState>()
        .diagnostics
        .lock()
        .unwrap()
        .values()
        .find(|s| s.page == page)
        .map(|s| s.id.clone());
    if let Some(id) = id {
        push(
            app,
            &id,
            json!({"source":"host","level":level,"message":message}),
        );
    }
}
/// 游标缺口与级别筛选分别处理，分页编码保持低于 512 KiB。
pub fn logs(app: &AppHandle, params: Value) -> Result<Value, Failure> {
    let input: Logs = serde_json::from_value(params).map_err(invalid)?;
    if !(1..=500).contains(&input.limit) {
        return Err(invalid("limit 必须为 1～500"));
    }
    if input
        .level
        .as_ref()
        .is_some_and(|s| !["debug", "info", "warning", "error"].contains(&s.as_str()))
    {
        return Err(invalid("日志级别不合法"));
    }
    let state = app.state::<AutomationState>();
    let sessions = state.diagnostics.lock().unwrap();
    let s = sessions
        .get(&input.id)
        .filter(|s| s.expires > pinpage_protocol::now())
        .ok_or_else(|| Failure::new("sessionExpired", "诊断会话已停止或过期"))?;
    let first = s
        .records
        .front()
        .and_then(|r| r["sequence"].as_u64())
        .unwrap_or(s.next);
    let gap = input.cursor.saturating_add(1) < first;
    if input.cursor >= s.next {
        return Err(invalid("游标超出会话范围"));
    }
    let mut result = vec![];
    let mut bytes = 0;
    let mut cursor = input.cursor;
    for record in &s.records {
        let sequence = record["sequence"].as_u64().unwrap();
        if sequence <= input.cursor {
            continue;
        }
        if input
            .level
            .as_ref()
            .is_some_and(|level| record["level"] != *level)
        {
            cursor = sequence;
            continue;
        }
        let size = serde_json::to_vec(record).unwrap().len();
        if result.len() >= input.limit as usize || bytes + size > 500 * 1024 {
            break;
        }
        cursor = sequence;
        bytes += size;
        result.push(record.clone());
    }
    Ok(
        json!({"records":result,"nextCursor":cursor,"dropped":s.dropped,"gap":gap,"expiresAt":s.expires}),
    )
}
/// 删除缓存立即阻止迟到事件，原生监听在所属 UI 线程注销。
pub fn stop(app: &AppHandle, id: &str) -> Result<Value, Failure> {
    let removed = app
        .state::<AutomationState>()
        .diagnostics
        .lock()
        .unwrap()
        .remove(id);
    if let Some(session) = &removed {
        if let Some(timer) = &session.timer {
            timer.abort();
        }
    }
    #[cfg(windows)]
    native::detach(app, id);
    Ok(json!({"sessionId":id,"stopped":removed.is_some()}))
}
/// 页面关闭、删除或进程异常都终止对应会话。
pub fn close_page(app: &AppHandle, page: &str) {
    let ids: Vec<_> = app
        .state::<AutomationState>()
        .diagnostics
        .lock()
        .unwrap()
        .values()
        .filter(|s| s.page == page)
        .map(|s| s.id.clone())
        .collect();
    for id in ids {
        let _ = stop(app, &id);
    }
}
/// 应用退出释放所有会话，日志从不写入磁盘。
pub fn stop_all(app: &AppHandle) {
    let ids: Vec<_> = app
        .state::<AutomationState>()
        .diagnostics
        .lock()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    for id in ids {
        let _ = stop(app, &id);
    }
}
/// 输入不合法与过期会话分开报告。
fn invalid(e: impl ToString) -> Failure {
    Failure::new("invalidArgument", e)
}
