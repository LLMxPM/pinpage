// 功能：维护独立于内容页权限的本机控制、实例修订、加载代次和短期操作状态。
pub mod diagnostics;
mod dispatch;
mod operations;
mod pipe;
#[cfg(windows)]
mod pipe_io;
pub use operations::committed as record_commit;
mod geometry;
#[cfg(windows)]
pub mod lifecycle;
use crate::{models::PinConfig, state::AppState};
use pinpage_protocol::Failure;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Mutex};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct Load {
    pub id: String,
    pub state: String,
    pub pending: bool,
    pub error: Option<Failure>,
}
pub struct AutomationState {
    pub instance_id: String,
    revisions: Mutex<HashMap<String, (u64, Value)>>,
    pub loads: Mutex<HashMap<String, Load>>,
    pub changed: tokio::sync::Notify,
    pub operations: Mutex<Vec<operations::Operation>>,
    pub diagnostics: Mutex<HashMap<String, diagnostics::Session>>,
    #[cfg(windows)]
    pub control: Mutex<Option<pipe::Service>>,
}
impl Default for AutomationState {
    /// 每次进程启动轮换实例，修订不能跨进程复用。
    fn default() -> Self {
        Self {
            instance_id: uuid::Uuid::new_v4().to_string(),
            revisions: Mutex::new(HashMap::new()),
            loads: Mutex::new(HashMap::new()),
            changed: tokio::sync::Notify::new(),
            operations: Mutex::new(vec![]),
            diagnostics: Mutex::new(HashMap::new()),
            #[cfg(windows)]
            control: Mutex::new(None),
        }
    }
}
/// 启动失败只报告管理通知，不放宽原有 WebView 权限。
pub fn start(app: &AppHandle) {
    #[cfg(windows)]
    if let Err(error) = pipe::start(app) {
        crate::state::report(
            app,
            crate::errors::AppMessage::new("errors.automationFailed").detail(error),
        );
    }
}
/// 应用退出唤醒事件驱动管道并注销所有诊断监听。
pub fn stop(app: &AppHandle) {
    #[cfg(windows)]
    pipe::stop(app, false);
    diagnostics::stop_all(app);
}
/// 工作台关闭入口时释放管道句柄，重开时重新签发凭据。
pub fn configure(app: &AppHandle, enabled: bool) -> Result<(), Failure> {
    #[cfg(windows)]
    {
        if enabled {
            pipe::start(app)?;
        } else {
            pipe::stop(app, true);
            pipe::publish_disabled(app)?;
        }
    }
    #[cfg(not(windows))]
    let _ = (app, enabled);
    Ok(())
}
/// 排除常规位置采样，明确窗口编辑通过 touch 单独递增修订。
pub fn revision(app: &AppHandle, pin: &PinConfig) -> u64 {
    let mut value = serde_json::to_value(pin).unwrap();
    for key in ["x", "y", "width", "height", "scale", "preset"] {
        value.as_object_mut().unwrap().remove(key);
    }
    let state = app.state::<AutomationState>();
    let mut items = state.revisions.lock().unwrap();
    let entry = items.entry(pin.id.clone()).or_insert((1, value.clone()));
    if entry.1 != value {
        entry.0 += 1;
        entry.1 = value;
    }
    entry.0
}
/// 工作台与 CLI 的明确设置提交均递增，拖动和加载事件不递增。
pub fn touch(app: &AppHandle, id: &str) {
    if let Some(state) = app.try_state::<AutomationState>() {
        if let Some(item) = state.revisions.lock().unwrap().get_mut(id) {
            item.0 += 1;
        }
    }
}
/// 发起受控导航时预留代次，随后 Started 事件沿用该 ID。
pub fn begin_load(app: &AppHandle, id: &str) -> String {
    let state = app.state::<AutomationState>();
    let load = Load {
        id: uuid::Uuid::new_v4().to_string(),
        state: "loading".into(),
        pending: true,
        error: None,
    };
    let result = load.id.clone();
    state.loads.lock().unwrap().insert(id.into(), load);
    state.changed.notify_waiters();
    result
}
/// 普通页面导航生成新代次；仅宿主刚发起的导航可消费预留代次。
pub fn navigation(app: &AppHandle, id: &str, started: bool) {
    let state = app.state::<AutomationState>();
    let mut loads = state.loads.lock().unwrap();
    if started {
        match loads.get_mut(id) {
            Some(load) if load.pending => load.pending = false,
            _ => {
                loads.insert(
                    id.into(),
                    Load {
                        id: uuid::Uuid::new_v4().to_string(),
                        state: "loading".into(),
                        pending: false,
                        error: None,
                    },
                );
            }
        }
    } else if let Some(load) = loads.get_mut(id) {
        load.state = "loaded".into();
        load.pending = false;
    }
    drop(loads);
    state.changed.notify_waiters();
    diagnostics::host(
        app,
        id,
        if started {
            "navigationStarted"
        } else {
            "navigationCompleted"
        },
        "info",
    );
}
/// 导航失败或关闭不会被当作 loaded；等待者收到同一代次的终态。
pub fn load_error(app: &AppHandle, id: &str, error: Failure) {
    let state = app.state::<AutomationState>();
    if let Some(load) = state.loads.lock().unwrap().get_mut(id) {
        load.state = if error.code == "pageClosed" {
            "closed"
        } else {
            "failed"
        }
        .into();
        load.error = Some(error.clone());
    }
    if error.code == "loadFailed" {
        if let Some(runtime) = app.state::<AppState>().runtime.lock().unwrap().get_mut(id) {
            runtime.loading = false;
            runtime.error = Some(
                crate::errors::AppMessage::new("errors.navigationFailed")
                    .arg("reason", &error.message),
            );
        }
        crate::state::notify(app);
    }
    state.changed.notify_waiters();
}
/// 获取稳定页面配置，未发现时返回独立机器错误。
pub fn pin(app: &AppHandle, id: &str) -> Result<PinConfig, Failure> {
    app.state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| Failure::new("pageNotFound", "页面不存在"))
}
/// 检查结果不返回文件夹根授权或实际导航 URL，防止日志带出敏感查询参数。
pub fn inspect(app: &AppHandle, id: &str) -> Result<Value, Failure> {
    let mut pin = pin(app, id)?;
    let revision = revision(app, &pin);
    pin.folder_root = None;
    let state = app.state::<AutomationState>();
    let load = state.loads.lock().unwrap().get(id).cloned();
    let runtime = app
        .state::<AppState>()
        .runtime
        .lock()
        .unwrap()
        .get(id)
        .cloned();
    Ok(
        json!({"pageId":id,"config":pin,"revision":revision,"loadId":load.as_ref().map(|l|&l.id),"loadState":load.as_ref().map_or("notRequested",|l|l.state.as_str()),"open":runtime.as_ref().is_some_and(|r|r.open),"visible":runtime.as_ref().is_some_and(|r|r.visible),"recentError":load.and_then(|l|l.error)}),
    )
}
/// 等待不持有全局操作锁；新导航、关闭和失败均明确终止旧等待。
pub async fn wait(app: &AppHandle, id: &str, load_id: &str, settle: u64) -> Result<Value, Failure> {
    let state = app.state::<AutomationState>();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let notified = state.changed.notified();
        let load = state
            .loads
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| Failure::new("loadNotFound", "页面尚未加载"))?;
        if load.id != load_id {
            return Err(Failure::new("loadReplaced", "导航已被新代次替代"));
        }
        if let Some(error) = load.error {
            return Err(error);
        }
        if load.state == "loaded" {
            if settle > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(settle)).await;
                if state
                    .loads
                    .lock()
                    .unwrap()
                    .get(id)
                    .is_none_or(|l| l.id != load_id)
                {
                    return Err(Failure::new("loadReplaced", "稳定等待期间导航变化"));
                }
            }
            return inspect(app, id);
        }
        if load.state != "loading" {
            return Err(Failure::new("pageClosed", "页面已关闭"));
        }
        tokio::time::timeout_at(deadline, notified)
            .await
            .map_err(|_| Failure::new("timeout", "导航等待超时"))?;
    }
}
