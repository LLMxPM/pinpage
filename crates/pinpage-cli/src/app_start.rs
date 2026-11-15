// 功能：显式启动托盘应用，禁止 GUI 继承 CLI 输出管道而阻塞 PowerShell JSON 读取。
use pinpage_protocol::Failure;
use std::path::Path;

/// 应用路径独立传给 CreateProcess，命令行仅包含受引号保护的 argv0 与固定 --tray。
#[cfg(windows)]
pub fn spawn(path: &Path) -> Result<(), Failure> {
    use windows_sys::Win32::{Foundation::*, System::Threading::*};
    let executable = pinpage_protocol::security::wide(path);
    if executable[..executable.len() - 1]
        .iter()
        .any(|c| *c == 0 || *c == b'"' as u16)
    {
        return Err(Failure::new("invalidArgument", "应用路径含非法字符"));
    }
    let mut command = vec![b'"' as u16];
    command.extend_from_slice(&executable[..executable.len() - 1]);
    command.extend("\" --tray".encode_utf16());
    command.push(0);
    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    let create = |flags, command: &mut Vec<u16>, process: &mut PROCESS_INFORMATION| unsafe {
        CreateProcessW(
            executable.as_ptr(),
            command.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags,
            std::ptr::null(),
            std::ptr::null(),
            &startup,
            process,
        )
    };
    // 允许脱离终端作业时一起脱离；受限制的作业可退回不继承任何输出句柄的创建方式。
    let created = create(
        DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB,
        &mut command,
        &mut process,
    ) != 0;
    let created = created
        || (unsafe { GetLastError() } == ERROR_ACCESS_DENIED
            && create(DETACHED_PROCESS, &mut command, &mut process) != 0);
    if !created {
        return Err(Failure::new(
            "appNotRunning",
            std::io::Error::last_os_error(),
        ));
    }
    unsafe {
        CloseHandle(process.hThread);
        CloseHandle(process.hProcess);
    }
    Ok(())
}
/// 其他平台仍保持无控制台输出继承；当前控制连接会在调用前报告不支持。
#[cfg(not(windows))]
pub fn spawn(path: &Path) -> Result<(), Failure> {
    std::process::Command::new(path)
        .arg("--tray")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| Failure::new("appNotRunning", e))
}
