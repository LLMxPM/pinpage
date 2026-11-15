// 功能：为本地资源协议与文件夹能力提供共享的根目录边界校验。
use std::path::{Component, Path, PathBuf};

/// 路径校验失败的原因；调用方按各自的消息码呈现，不暴露本地路径。
#[derive(Debug)]
pub enum GuardError {
    /// 相对路径包含非法字符（反斜杠、冒号或空字节）。
    Chars,
    /// 路径分量或规范化结果越出根目录。
    Outside,
    /// 根目录不存在或无法规范化。
    RootMissing,
    /// 目标不存在或无法规范化。
    NotFound,
}

/// 校验字面相对路径并返回根目录内的规范化绝对路径，允许文件或目录。
///
/// 输入不做百分号解码：资源协议先解码再调用，文件夹能力直接传 JSON 字符串，
/// 文件名中的 `%` 按字面处理。拒绝反斜杠、冒号与空字节，分量只允许普通名
/// 与当前目录；根与目标都规范化后确认边界，符号链接与目录联接指向根外时
/// 同样拒绝。空相对路径返回根目录本身。
pub fn resolve(root: &Path, relative: &str) -> Result<PathBuf, GuardError> {
    if relative.contains(['\\', ':', '\0']) {
        return Err(GuardError::Chars);
    }
    let path = Path::new(relative.trim_start_matches('/'));
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(GuardError::Outside);
    }
    let root = root.canonicalize().map_err(|_| GuardError::RootMissing)?;
    let target = root
        .join(path)
        .canonicalize()
        .map_err(|_| GuardError::NotFound)?;
    if !target.starts_with(&root) {
        return Err(GuardError::Outside);
    }
    Ok(target)
}

#[cfg(test)]
#[path = "pathguard_tests.rs"]
mod tests;
