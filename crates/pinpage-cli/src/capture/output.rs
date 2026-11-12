// 功能：在输出同目录完成写入与原子发布，兼容不支持硬链接的 Windows 文件系统。
use pinpage_protocol::Failure;
use std::{io::Write, path::Path};
/// 默认发布时禁止替换；写入失败或目标冲突都保留原文件并清理暂存文件。
pub fn atomic_write(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), Failure> {
    let path = std::path::absolute(path).map_err(failed)?;
    let parent = path.parent().ok_or_else(|| failed("输出路径无父目录"))?;
    std::fs::create_dir_all(parent).map_err(failed)?;
    let temp = parent.join(format!(".pinpage-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(failed)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(failed)?;
        drop(file);
        publish(&temp, &path, overwrite)
    })();
    let _ = std::fs::remove_file(temp);
    result
}
/// Windows 原生移动不带 REPLACE_EXISTING 时，竞争创建的目标也不会被覆盖。
#[cfg(windows)]
fn publish(temp: &Path, target: &Path, overwrite: bool) -> Result<(), Failure> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{core::PCWSTR, Win32::Storage::FileSystem::*};
    let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let flags = MOVEFILE_WRITE_THROUGH
        | if overwrite {
            MOVEFILE_REPLACE_EXISTING
        } else {
            MOVE_FILE_FLAGS(0)
        };
    unsafe { MoveFileExW(PCWSTR(from.as_ptr()), PCWSTR(to.as_ptr()), flags) }.map_err(|e| {
        if !overwrite && target.exists() {
            Failure::new("outputExists", "输出已存在")
        } else {
            failed(e)
        }
    })
}
/// 其他平台保持同目录发布；桌面采集能力本身仍明确返回不支持。
#[cfg(not(windows))]
fn publish(temp: &Path, target: &Path, overwrite: bool) -> Result<(), Failure> {
    if overwrite {
        std::fs::rename(temp, target).map_err(failed)
    } else {
        std::fs::hard_link(temp, target).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Failure::new("outputExists", "输出已存在")
            } else {
                failed(e)
            }
        })
    }
}
/// 保存失败与采集环境失败分开报告。
fn failed(error: impl ToString) -> Failure {
    Failure::new("captureSaveFailed", error)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protects_existing_output_and_replaces_explicitly() {
        let root = std::env::temp_dir().join(format!("pinpage-capture-{}", uuid::Uuid::new_v4()));
        let path = root.join("中文 图.png");
        atomic_write(&path, b"old", false).unwrap();
        assert_eq!(
            atomic_write(&path, b"new", false).unwrap_err().code,
            "outputExists"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"old");
        atomic_write(&path, b"new", true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(root);
    }
}
