// 功能：构造当前登录 SID 的显式 ACL，并验证管道两端的进程和会话。
use crate::Failure;
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::*,
    Security::{Authorization::*, *},
    System::Threading::*,
};

/// Win32 字符串只在调用期间使用，拒绝输入中的 NUL。
pub fn wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}
/// 核对当前进程或管道对端所属的 Windows 登录会话。
pub fn session(pid: u32) -> Result<u32, Failure> {
    let mut id = 0;
    if unsafe { windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId(pid, &mut id) } == 0
    {
        return Err(failed());
    }
    Ok(id)
}
/// 将最近的系统错误归入鉴权错误域。
fn failed() -> Failure {
    Failure::new("authenticationFailed", std::io::Error::last_os_error())
}

pub struct Acl {
    descriptor: PSECURITY_DESCRIPTOR,
}
impl Acl {
    /// 管道仅授予连接所需的单项权利，排除 FILE_CREATE_PIPE_INSTANCE；目录仅登录 SID 可访问。
    pub fn current(directory: bool) -> Result<Self, Failure> {
        unsafe {
            let mut token = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return Err(failed());
            }
            let mut length = 0;
            GetTokenInformation(token, TokenGroups, std::ptr::null_mut(), 0, &mut length);
            let mut buffer = vec![0u64; (length as usize).div_ceil(8)];
            let ok = GetTokenInformation(
                token,
                TokenGroups,
                buffer.as_mut_ptr().cast(),
                length,
                &mut length,
            );
            CloseHandle(token);
            if ok == 0 {
                return Err(failed());
            }
            let groups = &*(buffer.as_ptr() as *const TOKEN_GROUPS);
            let items =
                std::slice::from_raw_parts(groups.Groups.as_ptr(), groups.GroupCount as usize);
            let sid = items
                .iter()
                .find(|item| item.Attributes & 0xc0000000 == 0xc0000000)
                .ok_or_else(|| Failure::new("authenticationFailed", "缺少交互登录 SID"))?;
            let mut sid_text = std::ptr::null_mut();
            if ConvertSidToStringSidW(sid.Sid, &mut sid_text) == 0 {
                return Err(failed());
            }
            let mut len = 0;
            while *sid_text.add(len) != 0 {
                len += 1;
            }
            let sid_string = String::from_utf16_lossy(std::slice::from_raw_parts(sid_text, len));
            LocalFree(sid_text.cast());
            let rights = if directory { "FA" } else { "0x0012019b" };
            let inherit = if directory { "OICI" } else { "" };
            let sddl = wide(format!("D:P(A;{inherit};{rights};;;{sid_string})"));
            let mut descriptor = std::ptr::null_mut();
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(failed());
            }
            Ok(Self { descriptor })
        }
    }
    /// 属性的有效期必须覆盖 CreateNamedPipeW 调用。
    pub fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.descriptor,
            bInheritHandle: 0,
        }
    }
    /// 显式保护实例描述所在的会话目录，子文件继承同一登录 SID。
    pub fn protect(&self, path: &Path) -> Result<(), Failure> {
        let path = wide(path);
        if unsafe {
            SetFileSecurityW(
                path.as_ptr(),
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                self.descriptor,
            )
        } == 0
        {
            return Err(failed());
        }
        Ok(())
    }
}
impl Drop for Acl {
    /// 安全描述符由 LocalAlloc 分配，不能用 Rust 分配器释放。
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.descriptor);
        }
    }
}

/// 发送凭据前验证服务端 PID，避免把秘密交给抢占管道的错误进程。
/// # Safety
/// handle 必须为调用期间有效的管道句柄。
pub unsafe fn server_pid(handle: HANDLE) -> Result<u32, Failure> {
    let mut pid = 0;
    if unsafe { windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId(handle, &mut pid) }
        == 0
    {
        return Err(failed());
    }
    Ok(pid)
}
/// 服务端使用内核提供的客户端会话，拒绝请求正文伪造会话号。
/// # Safety
/// handle 必须为调用期间有效的服务端管道句柄。
pub unsafe fn client_session(handle: HANDLE) -> Result<u32, Failure> {
    let mut pid = 0;
    if unsafe { windows_sys::Win32::System::Pipes::GetNamedPipeClientProcessId(handle, &mut pid) }
        == 0
    {
        return Err(failed());
    }
    session(pid)
}

/// 客户端显式排除创建实例权限，并使用 overlapped 句柄接入 Tokio。
pub fn connect(pipe: &str) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    use windows_sys::Win32::Storage::FileSystem::*;
    let pipe = wide(pipe);
    let handle = unsafe {
        CreateFileW(
            pipe.as_ptr(),
            0x0012019b,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    unsafe { tokio::net::windows::named_pipe::NamedPipeClient::from_raw_handle(handle) }
}
