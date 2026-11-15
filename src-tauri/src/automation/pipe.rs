// 功能：发布受登录 SID 保护的实例描述，并在事件驱动管道上鉴权和分发请求。
#[cfg(windows)]
use {
    super::AutomationState,
    crate::state::AppState,
    pinpage_protocol::{security, transport, Failure, Instance, Request, Response},
    tauri::{AppHandle, Manager},
};

#[cfg(windows)]
pub struct Service {
    stop: std::sync::Arc<std::os::windows::io::OwnedHandle>,
    done: std::sync::mpsc::Receiver<()>,
}

/// 原生重叠 I/O 线程只在连接或退出事件时唤醒，配置和站点仍仅由应用进程写入。
#[cfg(windows)]
pub fn start(app: &AppHandle) -> Result<(), Failure> {
    let state = app.state::<AutomationState>();
    if !app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .local_control
    {
        return publish_disabled(app);
    }
    let mut service = state.control.lock().unwrap();
    if service.is_some() {
        return Ok(());
    }
    let id = app.config().identifier.clone();
    let instance = Instance {
        app_id: id.clone(),
        pid: std::process::id(),
        session_id: security::session(std::process::id())?,
        instance_id: state.instance_id.clone(),
        pipe: format!(r"\\.\pipe\PinPage-{}", state.instance_id),
        protocol_version: pinpage_protocol::VERSION,
        enabled: true,
        credential: format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        ),
    };
    let path = transport::descriptor_path(&id)?;
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent).map_err(failed)?;
    security::Acl::current(true)?.protect(parent)?;
    let pipe = super::pipe_io::create(&instance.pipe)?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, serde_json::to_vec(&instance).map_err(failed)?).map_err(failed)?;
    crate::state::replace_file(&temporary, &path).map_err(failed)?;
    let stop = pipe.stop.clone();
    let (done_sender, done) = std::sync::mpsc::channel();
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        while let Ok(true) = pipe.connect() {
            let result = (|| -> Result<(), Failure> {
                let session = pipe.session()?;
                let request: Request = pipe.read()?;
                let enabled = app
                    .state::<AppState>()
                    .settings
                    .lock()
                    .unwrap()
                    .local_control;
                let result = match request.authorize(&instance, session, enabled) {
                    Ok(()) => tauri::async_runtime::block_on(super::dispatch::request(
                        &app,
                        request.clone(),
                    )),
                    Err(error) => Err(error),
                };
                let response = Response::result(
                    request.request_id,
                    Some(instance.instance_id.clone()),
                    result,
                );
                pipe.write(&response)?;
                let ack: serde_json::Value = pipe.read()?;
                if ack["requestId"] != response.request_id || ack["received"] != true {
                    return Err(Failure::new("protocolMismatch", "响应确认不匹配"));
                }
                Ok(())
            })();
            if let Err(error) = result {
                eprintln!("本机控制请求失败：{}", error.code);
            }
            pipe.disconnect();
        }
        let _ = std::fs::remove_file(path);
        drop(pipe);
        let _ = done_sender.send(());
    });
    *service = Some(Service { stop, done });
    Ok(())
}
/// 服务端启动失败不包含描述文件正文。
#[cfg(windows)]
fn failed(error: impl ToString) -> Failure {
    Failure::new("connectionFailed", error)
}

/// 关闭后不保留管道、连接或秘密；工作台等待释放完成后才允许重新启用。
#[cfg(windows)]
pub fn stop(app: &AppHandle, wait: bool) {
    use std::os::windows::io::AsRawHandle;
    if let Some(service) = app
        .state::<AutomationState>()
        .control
        .lock()
        .unwrap()
        .take()
    {
        unsafe {
            windows_sys::Win32::System::Threading::SetEvent(service.stop.as_raw_handle());
        }
        if wait {
            let _ = service
                .done
                .recv_timeout(std::time::Duration::from_secs(15));
        }
    }
    if let Ok(path) = transport::descriptor_path(&app.config().identifier) {
        let _ = std::fs::remove_file(path);
    }
}
/// 禁用描述保留进程和会话验证所需字段，不包含有效凭据。
#[cfg(windows)]
pub fn publish_disabled(app: &AppHandle) -> Result<(), Failure> {
    let state = app.state::<AutomationState>();
    let instance = Instance {
        app_id: app.config().identifier.clone(),
        pid: std::process::id(),
        session_id: security::session(std::process::id())?,
        instance_id: state.instance_id.clone(),
        pipe: format!(r"\\.\pipe\PinPage-{}", state.instance_id),
        protocol_version: pinpage_protocol::VERSION,
        credential: String::new(),
        enabled: false,
    };
    let path = transport::descriptor_path(&instance.app_id)?;
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent).map_err(failed)?;
    security::Acl::current(true)?.protect(parent)?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, serde_json::to_vec(&instance).map_err(failed)?).map_err(failed)?;
    crate::state::replace_file(&temp, &path).map_err(failed)
}
