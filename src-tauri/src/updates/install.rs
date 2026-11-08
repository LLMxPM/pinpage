// 功能：安装前冻结配置写入、保存并备份会话，失败恢复普通运行状态。
use super::{model::Status, publish, UpdateState};
use crate::{errors::AppMessage, state::AppState};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

/// 只消费本进程已验签的数据；先保存备份，成功后才启动 Windows 安装程序。
pub(super) fn start(app: &AppHandle) -> Result<(), AppMessage> {
    let state = app.state::<UpdateState>();
    let mut task = state.task.lock().unwrap();
    if task.snapshot.status != Status::Ready {
        return Err(AppMessage::new("errors.updateNotReady"));
    }
    let update = task
        .candidate
        .clone()
        .ok_or_else(|| AppMessage::new("errors.updateNotReady"))?;
    let bytes = task
        .bytes
        .take()
        .ok_or_else(|| AppMessage::new("errors.updateNotReady"))?;
    task.snapshot.status = Status::Installing;
    task.snapshot.error = None;
    drop(task);
    publish(app);
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let result = {
            let _operation = state.operation.lock().unwrap();
            state.prepare_update()
        };
        let result = result.and_then(|()| {
            update
                .install(&bytes)
                .map_err(|error| AppMessage::new("errors.updateInstallFailed").detail(error))
        });
        if let Err(error) = result {
            state.installing.store(false, Ordering::SeqCst);
            state.exiting.store(false, Ordering::SeqCst);
            let updates = app.state::<UpdateState>();
            let mut task = updates.task.lock().unwrap();
            task.bytes = Some(bytes);
            task.snapshot.status = Status::Ready;
            task.snapshot.error = Some(error);
            drop(task);
            publish(&app);
        }
    });
    Ok(())
}
