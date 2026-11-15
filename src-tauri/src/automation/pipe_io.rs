// 功能：用原生重叠 I/O 完成单实例管道的事件等待和有界读写，避免连接复用的预读残留。
use pinpage_protocol::{security, Failure, MAX_MESSAGE};
use std::{
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::Arc,
};
use windows_sys::Win32::{
    Foundation::*,
    Storage::FileSystem::*,
    System::{Pipes::*, Threading::*, IO::*},
};

pub struct Pipe {
    handle: OwnedHandle,
    pub stop: Arc<OwnedHandle>,
}
/// OwnedHandle 管理 Win32 句柄，保证连接、事件和退出路径均释放。
fn owned(handle: HANDLE) -> Result<OwnedHandle, Failure> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        Err(failed())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }
}
/// 通过登录 SID ACL 创建唯一管道，客户端权利不包含创建第二个实例。
pub fn create(name: &str) -> Result<Pipe, Failure> {
    let acl = security::Acl::current(false)?;
    let attrs = acl.attributes();
    let name = security::wide(name);
    let handle = owned(unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            65536,
            65536,
            0,
            &attrs,
        )
    })?;
    let stop = Arc::new(owned(unsafe {
        CreateEventW(std::ptr::null(), 1, 0, std::ptr::null())
    })?);
    Ok(Pipe { handle, stop })
}
impl Pipe {
    /// 接入与退出均由内核事件唤醒，空闲不轮询桌面或进程。
    pub fn connect(&self) -> Result<bool, Failure> {
        if unsafe { WaitForSingleObject(self.stop.as_raw_handle(), 0) } == WAIT_OBJECT_0 {
            return Ok(false);
        }
        let event = owned(unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) })?;
        let mut overlap = OVERLAPPED {
            hEvent: event.as_raw_handle(),
            ..Default::default()
        };
        if unsafe { ConnectNamedPipe(self.handle.as_raw_handle(), &mut overlap) } != 0 {
            return Ok(true);
        }
        match unsafe { GetLastError() } {
            ERROR_PIPE_CONNECTED => Ok(true),
            ERROR_IO_PENDING => self.await_io(&mut overlap, INFINITE).map(|v| v.is_some()),
            _ => Err(failed()),
        }
    }
    /// 调用方已完成响应确认，原生句柄不保留用户态的上一次预读缓存。
    pub fn disconnect(&self) {
        unsafe {
            DisconnectNamedPipe(self.handle.as_raw_handle());
        }
    }
    /// 登录会话由内核对端进程提供，而非请求正文。
    pub fn session(&self) -> Result<u32, Failure> {
        unsafe { security::client_session(self.handle.as_raw_handle()) }
    }
    /// 有长度前缀的消息先检查预算再分配，支持多个底层 ReadFile 分片。
    pub fn read<T: serde::de::DeserializeOwned>(&self) -> Result<T, Failure> {
        let mut prefix = [0u8; 4];
        self.read_exact(&mut prefix)?;
        let length = u32::from_le_bytes(prefix) as usize;
        if length == 0 || length > MAX_MESSAGE {
            return Err(Failure::new("invalidArgument", "消息超出 1 MiB 限制"));
        }
        let mut bytes = vec![0; length];
        self.read_exact(&mut bytes)?;
        serde_json::from_slice(&bytes).map_err(|e| Failure::new("invalidArgument", e))
    }
    /// 不把截图放入管道；控制响应编码后仍限制在 1 MiB。
    pub fn write(&self, value: &impl serde::Serialize) -> Result<(), Failure> {
        let bytes = serde_json::to_vec(value).map_err(|e| Failure::new("executionFailed", e))?;
        if bytes.len() > MAX_MESSAGE {
            return Err(Failure::new("responseTooLarge", "响应超出限制"));
        }
        self.write_all(&(bytes.len() as u32).to_le_bytes())?;
        self.write_all(&bytes)
    }
    /// 精确读取帧内字节，断开不能被当作空 JSON。
    fn read_exact(&self, mut bytes: &mut [u8]) -> Result<(), Failure> {
        while !bytes.is_empty() {
            let count = self.transfer(bytes.as_mut_ptr(), bytes.len(), false)?;
            if count == 0 {
                return Err(Failure::new("connectionFailed", "客户端断开"));
            }
            bytes = &mut bytes[count..];
        }
        Ok(())
    }
    /// 精确写入全部字节，后续响应确认保证断开时数据已经被客户端取走。
    fn write_all(&self, mut bytes: &[u8]) -> Result<(), Failure> {
        while !bytes.is_empty() {
            let count = self.transfer(bytes.as_ptr().cast_mut(), bytes.len(), true)?;
            if count == 0 {
                return Err(Failure::new("connectionFailed", "写入中断"));
            }
            bytes = &bytes[count..];
        }
        Ok(())
    }
    /// 每次 I/O 的 OVERLAPPED 与缓冲区均保留到内核完成或取消完成。
    fn transfer(&self, buffer: *mut u8, length: usize, write: bool) -> Result<usize, Failure> {
        if unsafe { WaitForSingleObject(self.stop.as_raw_handle(), 0) } == WAIT_OBJECT_0 {
            return Err(Failure::new("cancelled", "控制入口已关闭"));
        }
        let event = owned(unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) })?;
        let mut overlap = OVERLAPPED {
            hEvent: event.as_raw_handle(),
            ..Default::default()
        };
        let mut count = 0;
        let ok = unsafe {
            if write {
                WriteFile(
                    self.handle.as_raw_handle(),
                    buffer,
                    length.min(u32::MAX as usize) as u32,
                    &mut count,
                    &mut overlap,
                )
            } else {
                ReadFile(
                    self.handle.as_raw_handle(),
                    buffer,
                    length.min(u32::MAX as usize) as u32,
                    &mut count,
                    &mut overlap,
                )
            }
        };
        if ok != 0 {
            return Ok(count as usize);
        }
        if unsafe { GetLastError() } != ERROR_IO_PENDING {
            return Err(failed());
        }
        self.await_io(&mut overlap, 10000)?
            .map(|v| v as usize)
            .ok_or_else(|| Failure::new("cancelled", "应用退出"))
    }
    /// 超时或退出先取消并等待完成，禁止让内核继续访问已释放的 Rust 缓冲区。
    fn await_io(&self, overlap: &mut OVERLAPPED, timeout: u32) -> Result<Option<u32>, Failure> {
        let handles = [overlap.hEvent, self.stop.as_raw_handle()];
        let wait = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, timeout) };
        if wait == WAIT_OBJECT_0 {
            let mut count = 0;
            if unsafe { GetOverlappedResult(self.handle.as_raw_handle(), overlap, &mut count, 0) }
                == 0
            {
                return Err(failed());
            }
            return Ok(Some(count));
        }
        unsafe {
            CancelIoEx(self.handle.as_raw_handle(), overlap);
            WaitForSingleObject(overlap.hEvent, INFINITE);
        }
        if wait == WAIT_OBJECT_0 + 1 {
            return Ok(None);
        }
        Err(Failure::new("timeout", "管道 I/O 超时"))
    }
}
/// 原始系统错误保留诊断，但绝不记录请求凭据。
fn failed() -> Failure {
    Failure::new("connectionFailed", std::io::Error::last_os_error())
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 连续连接真实管道，确认上一连接的 EOF 或预读不会污染下一条请求。
    #[tokio::test]
    async fn repeated_connections_and_first_instance_protection() {
        let name = format!(r"\\.\pipe\PinPage-test-{}", uuid::Uuid::new_v4());
        let pipe = create(&name).unwrap();
        assert!(create(&name).is_err());
        let server = std::thread::spawn(move || {
            for i in 0..20 {
                assert!(pipe.connect().unwrap());
                let value: serde_json::Value = pipe.read().unwrap();
                assert_eq!(value["sequence"], i);
                pipe.write(&value).unwrap();
                let _: serde_json::Value = pipe.read().unwrap();
                pipe.disconnect();
            }
        });
        for i in 0..20 {
            let mut client = loop {
                match security::connect(&name) {
                    Ok(v) => break v,
                    Err(e) if e.raw_os_error() == Some(231) => {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            pinpage_protocol::transport::write(&mut client, &serde_json::json!({"sequence":i}))
                .await
                .unwrap();
            let response: serde_json::Value = pinpage_protocol::transport::read(&mut client)
                .await
                .unwrap();
            assert_eq!(response["sequence"], i);
            pinpage_protocol::transport::write(&mut client, &serde_json::json!({"received":true}))
                .await
                .unwrap();
            use tokio::io::AsyncReadExt;
            let _ = client.read_u8().await;
        }
        server.join().unwrap();
    }
    /// 禁用时唤醒空闲接入等待，退出后可重新创建同名管道，没有残留服务端句柄。
    #[test]
    fn idle_shutdown_releases_pipe() {
        let name = format!(r"\\.\pipe\PinPage-stop-test-{}", uuid::Uuid::new_v4());
        let pipe = create(&name).unwrap();
        let stop = pipe.stop.clone();
        let task = std::thread::spawn(move || assert!(!pipe.connect().unwrap()));
        unsafe {
            SetEvent(stop.as_raw_handle());
        }
        task.join().unwrap();
        assert!(create(&name).is_ok());
    }
}
