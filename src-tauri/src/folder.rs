// 功能：管理逐页文件夹授权，并在授权根目录内提供只读列举与打开定位。
use crate::{
    errors::AppMessage,
    models::PinConfig,
    pathguard,
    state::{self, AppState},
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// 单次列举返回的条目上限，超出时截断并标记，避免大目录拖垮页面。
const MAX_ENTRIES: usize = 2000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntry {
    pub name: String,
    /// "file" 或 "dir"；符号链接不跟随，直接跳过。
    pub kind: &'static str,
    pub size_bytes: u64,
    pub modified_ms: Option<u64>,
    pub hidden: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderListing {
    /// 授权根目录的显示名，页面重启后无需重新选择即可展示。
    pub root_label: String,
    pub entries: Vec<FolderEntry>,
    pub truncated: bool,
}

/// 读取页面授权根目录；未授权或根失效返回对应消息码。
fn granted_root(pin: &PinConfig) -> Result<PathBuf, AppMessage> {
    let root = pin
        .folder_root
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppMessage::new("errors.folderNoGrant"))?;
    let path = Path::new(root)
        .canonicalize()
        .map_err(|_| AppMessage::new("errors.folderRootMissing"))?;
    if !path.is_dir() {
        return Err(AppMessage::new("errors.folderRootMissing"));
    }
    Ok(path)
}

/// 把共享边界校验的失败映射为文件夹能力的消息码。
fn guard_error(error: pathguard::GuardError) -> AppMessage {
    match error {
        pathguard::GuardError::Chars | pathguard::GuardError::Outside => {
            AppMessage::new("errors.folderOutside")
        }
        pathguard::GuardError::RootMissing => AppMessage::new("errors.folderRootMissing"),
        pathguard::GuardError::NotFound => AppMessage::new("errors.folderNotFound"),
    }
}

/// 目录显示名；盘符根目录没有文件名时退回完整盘符。
fn root_label(root: &Path) -> String {
    root.file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned())
}

/// Windows 隐藏属性判定；其他平台暂不识别隐藏文件。
#[cfg(windows)]
fn is_hidden(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes() & 0x2 != 0
}

#[cfg(not(windows))]
fn is_hidden(_meta: &std::fs::Metadata) -> bool {
    false
}

/// 列举授权根目录内指定相对路径的条目，按名称排序并限制数量。
/// 符号链接条目跳过；目录联接虽可列出，进入时仍会被边界校验拒绝。
pub fn list(pin: &PinConfig, relative: &str) -> Result<FolderListing, AppMessage> {
    let root = granted_root(pin)?;
    let target = pathguard::resolve(&root, relative).map_err(guard_error)?;
    if !target.is_dir() {
        return Err(AppMessage::new("errors.folderNotFound"));
    }
    let read =
        std::fs::read_dir(&target).map_err(|_| AppMessage::new("errors.folderReadFailed"))?;
    let mut entries = Vec::new();
    let mut truncated = false;
    for item in read {
        let Ok(item) = item else { continue };
        if entries.len() >= MAX_ENTRIES {
            truncated = true;
            break;
        }
        let Ok(file_type) = item.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let Ok(meta) = item.metadata() else { continue };
        let modified_ms = meta.modified().ok().and_then(|time| {
            time.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|value| value.as_millis() as u64)
        });
        entries.push(FolderEntry {
            name: item.file_name().to_string_lossy().into_owned(),
            kind: if meta.is_dir() { "dir" } else { "file" },
            size_bytes: if meta.is_dir() { 0 } else { meta.len() },
            modified_ms,
            hidden: is_hidden(&meta),
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(FolderListing {
        root_label: root_label(&root),
        entries,
        truncated,
    })
}

/// 校验并返回授权根目录内待打开文件的绝对路径，只允许文件。
pub fn open_target(pin: &PinConfig, relative: &str) -> Result<PathBuf, AppMessage> {
    let root = granted_root(pin)?;
    let target = pathguard::resolve(&root, relative).map_err(guard_error)?;
    if !target.is_file() {
        return Err(AppMessage::new("errors.folderNotFound"));
    }
    Ok(target)
}

/// 返回条目所在目录：文件取父目录，目录取自身，用于在资源管理器中打开。
pub fn reveal_target(pin: &PinConfig, relative: &str) -> Result<PathBuf, AppMessage> {
    let root = granted_root(pin)?;
    let target = pathguard::resolve(&root, relative).map_err(guard_error)?;
    if target.is_dir() {
        return Ok(target);
    }
    Ok(target.parent().map(Path::to_path_buf).unwrap_or(root))
}

/// 保存用户刚选择的文件夹授权；写入失败时回滚内存配置并报告。
/// 返回展示用目录名，完整路径只留在 Rust 配置中。
pub fn save_root(app: &AppHandle, id: &str, root: &Path) -> Result<String, AppMessage> {
    let canonical = root
        .canonicalize()
        .map_err(|_| AppMessage::new("errors.folderPathInvalid"))?;
    if !canonical.is_dir() {
        return Err(AppMessage::new("errors.folderPathInvalid"));
    }
    let label = root_label(&canonical);
    let state = app.state::<AppState>();
    let _operation = state.operation.lock().unwrap();
    state.ensure_mutable()?;
    let previous = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .map(|pin| pin.folder_root.clone())
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))?;
    let value = canonical.to_string_lossy().into_owned();
    if let Some(pin) = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter_mut()
        .find(|pin| pin.id == id)
    {
        pin.folder_root = Some(value);
    }
    if let Err(error) = state.save() {
        if let Some(pin) = state
            .settings
            .lock()
            .unwrap()
            .pins
            .iter_mut()
            .find(|pin| pin.id == id)
        {
            pin.folder_root = previous;
        }
        return Err(error);
    }
    state::notify(app);
    Ok(label)
}

#[cfg(test)]
#[path = "folder_tests.rs"]
mod tests;
