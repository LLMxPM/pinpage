// 功能：管理 GitHub Release 升级单任务、状态快照与后台每日检查。
mod download;
mod install;
mod model;
mod storage;
#[cfg(test)]
mod tests;

use crate::{errors::AppMessage, state::AppState};
pub use model::Snapshot;
use model::{Preferences, Status};
use std::{path::PathBuf, sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

pub struct UpdateState {
    task: Mutex<Task>,
    path: PathBuf,
}

struct Task {
    snapshot: Snapshot,
    candidate: Option<Update>,
    bytes: Option<Vec<u8>>,
}

impl UpdateState {
    /// 从独立偏好文件恢复检查节流；开发和验证实例不能安装正式应用。
    pub fn load(app: &AppHandle, path: PathBuf) -> Self {
        let loaded = storage::load(&path);
        let error = loaded.as_ref().err().cloned();
        Self {
            path,
            task: Mutex::new(Task {
                snapshot: Snapshot {
                    current_version: app.package_info().version.to_string(),
                    supported: cfg!(all(windows, target_arch = "x86_64"))
                        && !cfg!(debug_assertions)
                        && app.config().identifier == "com.pinpage.desktop",
                    status: Status::Idle,
                    latest_version: None,
                    notes: None,
                    downloaded: 0,
                    total: None,
                    error,
                    preferences: loaded.unwrap_or(Preferences {
                        auto_check: false,
                        ..Preferences::default()
                    }),
                },
                candidate: None,
                bytes: None,
            }),
        }
    }
}

/// 将升级任务快照只发往受信任管理窗口，内容页不接收状态或错误详情。
fn publish(app: &AppHandle) {
    let snapshot = snapshot(app);
    let _ = app.emit_to("main", "updates-changed", snapshot);
}

/// 管理窗口重建时读取最新状态，不重新开始正在运行的下载。
pub fn snapshot(app: &AppHandle) -> Snapshot {
    app.state::<UpdateState>()
        .task
        .lock()
        .unwrap()
        .snapshot
        .clone()
}

/// 统一记录失败并释放单任务状态，保留候选供下载重试或 Release 手动安装。
fn failed(app: &AppHandle, error: AppMessage) {
    let state = app.state::<UpdateState>();
    let mut task = state.task.lock().unwrap();
    task.snapshot.status = Status::Error;
    task.snapshot.error = Some(error);
    drop(task);
    publish(app);
}

/// 只接受正式版递增，并固定 GitHub 静态入口；下载阶段使用单独的较长超时。
pub fn check(app: &AppHandle, automatic: bool) -> Result<(), AppMessage> {
    let state = app.state::<UpdateState>();
    let mut task = state.task.lock().unwrap();
    if !task.snapshot.supported {
        return Err(AppMessage::new("errors.updateUnsupported"));
    }
    if automatic
        && (!model::due(&task.snapshot.preferences, model::now())
            || task.snapshot.status.occupied())
    {
        return Ok(());
    }
    if task.snapshot.status.occupied() {
        return Err(AppMessage::new("errors.updateBusy"));
    }
    let mut preferences = task.snapshot.preferences.clone();
    preferences.last_check = Some(model::now());
    storage::save(&state.path, &preferences)?;
    task.snapshot.preferences = preferences;
    task.snapshot.status = Status::Checking;
    task.snapshot.error = None;
    task.snapshot.latest_version = None;
    task.snapshot.notes = None;
    task.snapshot.downloaded = 0;
    task.snapshot.total = None;
    task.candidate = None;
    task.bytes = None;
    drop(task);
    publish(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = async {
            let mut update = app
                .updater_builder()
                .endpoints(vec![model::ENDPOINT.parse().unwrap()])
                .map_err(check_error)?
                .timeout(Duration::from_secs(20))
                .version_comparator(|current, remote| {
                    remote.version.pre.is_empty()
                        && remote.version.build.is_empty()
                        && remote.version > current
                })
                .on_before_exit({
                    let app = app.clone();
                    move || {
                        app.state::<AppState>()
                            .exiting
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                })
                .build()
                .map_err(check_error)?
                .check()
                .await
                .map_err(check_error)?;
            if let Some(value) = &mut update {
                model::validate_release(&value.version, &value.download_url)?;
                value.timeout = Some(Duration::from_secs(300));
            }
            Ok::<_, AppMessage>(update)
        }
        .await;
        match result {
            Ok(update) => {
                let state = app.state::<UpdateState>();
                let mut task = state.task.lock().unwrap();
                task.snapshot.status = match &update {
                    Some(value)
                        if automatic
                            && task.snapshot.preferences.ignored_version.as_ref()
                                == Some(&value.version) =>
                    {
                        Status::Ignored
                    }
                    Some(_) => Status::Available,
                    None => Status::UpToDate,
                };
                task.snapshot.latest_version = update.as_ref().map(|value| value.version.clone());
                task.snapshot.notes = update.as_ref().and_then(|value| {
                    value
                        .body
                        .as_ref()
                        .map(|text| text.chars().take(16_000).collect())
                });
                task.candidate = update;
                drop(task);
                publish(&app);
            }
            Err(error) => failed(&app, error),
        }
    });
    Ok(())
}

/// 检查请求错误转为稳定消息，网络和 JSON 诊断保留在详情中。
fn check_error(error: tauri_plugin_updater::Error) -> AppMessage {
    AppMessage::new("errors.updateCheckFailed").detail(error)
}

/// 分发固定升级动作，不接收前端提供的地址、公钥或安装文件路径。
pub fn action(app: &AppHandle, action: &str, enabled: Option<bool>) -> Result<(), AppMessage> {
    match action {
        "check" => check(app, false),
        "download" => download::start(app),
        "install" => install::start(app),
        "autoCheck" | "ignore" => {
            let state = app.state::<UpdateState>();
            let mut task = state.task.lock().unwrap();
            if task.snapshot.status == Status::Installing {
                return Err(AppMessage::new("errors.updateBusy"));
            }
            let mut preferences = task.snapshot.preferences.clone();
            if action == "autoCheck" {
                preferences.auto_check =
                    enabled.ok_or_else(|| AppMessage::new("errors.updateActionInvalid"))?;
            } else {
                if task.snapshot.status.occupied() && task.snapshot.status != Status::Ready {
                    return Err(AppMessage::new("errors.updateBusy"));
                }
                preferences.ignored_version = Some(
                    task.candidate
                        .as_ref()
                        .ok_or_else(|| AppMessage::new("errors.updateNotReady"))?
                        .version
                        .clone(),
                );
            }
            storage::save(&state.path, &preferences)?;
            task.snapshot.preferences = preferences;
            if action == "ignore" {
                task.snapshot.status = Status::Ignored;
                task.snapshot.error = None;
                task.bytes = None;
                task.snapshot.downloaded = 0;
                task.snapshot.total = None;
            }
            drop(task);
            publish(app);
            Ok(())
        }
        "openRelease" => {
            use tauri_plugin_opener::OpenerExt;
            let version = snapshot(app).latest_version;
            let url = version.map_or(model::RELEASES.into(), |version| {
                format!("{}/tag/v{version}", model::RELEASES)
            });
            app.opener()
                .open_url(url, None::<&str>)
                .map_err(|error| AppMessage::new("errors.updateOpenFailed").detail(error))
        }
        _ => Err(AppMessage::new("errors.updateActionInvalid")),
    }
}

/// 原生后台线程按小时唤醒并检查每日间隔，不创建管理 WebView 或强制下载。
pub fn start_background(app: &AppHandle) {
    if !snapshot(app).supported {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(15));
        loop {
            if app
                .state::<AppState>()
                .exiting
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                break;
            }
            if let Err(error) = check(&app, true) {
                failed(&app, error);
            }
            std::thread::sleep(Duration::from_secs(3600));
        }
    });
}
