// 功能：读取 Windows 用户首选界面语言，为“跟随系统”提供有序语言列表。

/// Windows 使用 `GetUserPreferredUILanguages` 读取用户偏好顺序，
/// 避免由 WebView 的浏览器语言决定托盘等原生界面的语言。
#[cfg(windows)]
pub fn user_preferred_languages() -> Vec<String> {
    use std::mem::zeroed;
    use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    unsafe {
        let mut count = 0u32;
        let mut size = 0u32;
        // 第一次调用只取数量与缓冲区长度，缓冲区传空指针。
        if GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            std::ptr::null_mut(),
            &mut size,
        ) == 0
            || size == 0
        {
            return Vec::new();
        }
        let mut buffer: Vec<u16> = vec![zeroed(); size as usize];
        if GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            buffer.as_mut_ptr(),
            &mut size,
        ) == 0
        {
            return Vec::new();
        }
        // 返回值是多段字符串：以 \0 分隔、以空段结束。
        split_multizip(&buffer)
    }
}

/// 非 Windows 平台不提供系统语言读取，由调用方回退中文。
#[cfg(not(windows))]
pub fn user_preferred_languages() -> Vec<String> {
    Vec::new()
}

/// 把 UTF-16 多段缓冲区拆成语言标签列表。
fn split_multizip(buffer: &[u16]) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = Vec::new();
    for unit in buffer {
        if *unit == 0 {
            if current.is_empty() {
                break;
            }
            items.push(String::from_utf16_lossy(&current));
            current.clear();
        } else {
            current.push(*unit);
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 多段缓冲区按空字符切分，遇到结束空段即停止。
    fn splits_multizip_buffer() {
        let buffer: Vec<u16> = "zh-CN\0en-US\0\0extra".encode_utf16().collect();
        assert_eq!(split_multizip(&buffer), vec!["zh-CN", "en-US"]);
    }

    #[test]
    /// 读取系统语言不应 panic，且返回小写可解析的标签。
    fn reads_without_panicking() {
        let _ = user_preferred_languages();
    }
}
