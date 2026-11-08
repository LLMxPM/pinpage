// 功能：在 Rust 后台下载并验签升级包，节流进度通知且不依赖管理窗口存活。
use super::{failed, model::Status, publish, UpdateState};
use crate::errors::AppMessage;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// 领取唯一候选并异步下载；只有官方插件验签成功后才进入可安装状态。
pub(super) fn start(app: &AppHandle) -> Result<(), AppMessage> {
    let state = app.state::<UpdateState>();
    let mut task = state.task.lock().unwrap();
    if task.snapshot.status.occupied() {
        return Err(AppMessage::new("errors.updateBusy"));
    }
    let update = task
        .candidate
        .clone()
        .ok_or_else(|| AppMessage::new("errors.updateNotReady"))?;
    task.snapshot.status = Status::Downloading;
    task.snapshot.downloaded = 0;
    task.snapshot.total = None;
    task.snapshot.error = None;
    task.bytes = None;
    drop(task);
    publish(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let progress_app = app.clone();
        let finish_app = app.clone();
        let mut last = Instant::now();
        let result = update
            .download(
                move |chunk, total| {
                    let state = progress_app.state::<UpdateState>();
                    let mut task = state.task.lock().unwrap();
                    task.snapshot.downloaded += chunk as u64;
                    task.snapshot.total = total;
                    drop(task);
                    if last.elapsed() >= Duration::from_millis(150) {
                        publish(&progress_app);
                        last = Instant::now();
                    }
                },
                move || {
                    finish_app
                        .state::<UpdateState>()
                        .task
                        .lock()
                        .unwrap()
                        .snapshot
                        .status = Status::Verifying;
                    publish(&finish_app);
                },
            )
            .await;
        match result {
            Ok(bytes) => {
                let state = app.state::<UpdateState>();
                let mut task = state.task.lock().unwrap();
                task.snapshot.status = Status::Ready;
                task.bytes = Some(bytes);
                drop(task);
                publish(&app);
            }
            Err(error) => failed(
                &app,
                AppMessage::new("errors.updateDownloadFailed").detail(error),
            ),
        }
    });
    Ok(())
}
