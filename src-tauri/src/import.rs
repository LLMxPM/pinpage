// 功能：把用户选择的网页复制到应用数据目录，形成不依赖原始路径的导入页。
use crate::{errors::AppMessage, models::ImportMode};
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager};

/// 单次导入的文件数量上限，避免把整个资料目录复制进应用数据目录。
const MAX_FILES: u64 = 4000;
/// 单次导入的总体积上限，页面属于轻量工具，超出时应先精简目录。
const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// 随包页面数据的暂存文件名，窗口首次创建时读取并立即删除。
pub const PENDING_DATA: &str = "pending-data.json";
/// 新来源的暂存目录名，完整落盘后才与站点副本交换。
const INCOMING_DIR: &str = "incoming";
/// 被让位的旧站点副本目录名，交换成功后立即删除，中断时下次重新导入前清理。
const STALE_DIR: &str = "stale";

/// 原生选择器签发的导入暂存，校验通过后才会真正复制文件。
#[derive(Clone, Debug)]
pub struct ImportGrant {
    /// 目录导入时为所选目录，单文件导入时为所选文件。
    pub path: PathBuf,
    pub mode: ImportMode,
    /// 目录导入时记录根级 HTML 候选，创建时只接受其中之一。
    pub candidates: Vec<String>,
    /// 内置来源的类型随票据保存，创建时不能用前端输入替换来源。
    pub example: crate::examples::ExampleId,
}

/// 应用数据目录下的导入页根目录，每个页面一个以 ID 命名的子目录。
pub fn pages_root(app: &AppHandle) -> Result<PathBuf, AppMessage> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("pages"))
        .map_err(|error| AppMessage::new("errors.appDataDir").detail(error))
}

/// 页面 ID 必须是单级普通目录名，防止配置文件被改写后越出 pages 根目录。
fn validate_id(id: &str) -> Result<(), AppMessage> {
    let mut parts = Path::new(id).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(AppMessage::new("errors.pinIdInvalid")),
    }
}

/// 页面在导入根目录下的私有目录，站点副本与暂存目录都在其中。
fn page_root(app: &AppHandle, id: &str) -> Result<PathBuf, AppMessage> {
    validate_id(id)?;
    Ok(pages_root(app)?.join(id))
}

/// 页面的站点副本根目录；资源协议与删除操作都以它为唯一边界。
pub fn site_root(app: &AppHandle, id: &str) -> Result<PathBuf, AppMessage> {
    Ok(page_root(app, id)?.join("site"))
}

/// 新来源的暂存目录：先完整落盘，再通过 `swap_site` 转正，避免替换中途失败留下空站点。
pub fn incoming_root(app: &AppHandle, id: &str) -> Result<PathBuf, AppMessage> {
    Ok(page_root(app, id)?.join(INCOMING_DIR))
}

/// 用暂存目录替换站点副本。
///
/// 旧副本先改名让位再被新副本顶替：改名失败说明文件仍被占用，此时原副本完好无损；
/// 新副本改名失败则把旧副本还原，任何一条路径都不会让页面指向空目录。
pub fn swap_site(app: &AppHandle, id: &str) -> Result<(), AppMessage> {
    let root = page_root(app, id)?;
    let (site, incoming, stale) = (
        root.join("site"),
        root.join(INCOMING_DIR),
        root.join(STALE_DIR),
    );
    if !incoming.is_dir() {
        return Err(AppMessage::new("errors.swapSourceMissing"));
    }
    if stale.exists() {
        // 上次交换中断留下的旧副本，先清掉才腾得出目录名。
        std::fs::remove_dir_all(&stale)
            .map_err(|error| AppMessage::new("errors.cleanupFailed").detail(error))?;
    }
    if site.exists() {
        std::fs::rename(&site, &stale).map_err(|error| {
            AppMessage::new("errors.sourceReplaceFailed")
                .detail(format!("{}：{error}", site.display()))
        })?;
    }
    if let Err(error) = std::fs::rename(&incoming, &site) {
        let _ = std::fs::rename(&stale, &site);
        return Err(AppMessage::new("errors.sourceReplaceFailed")
            .detail(format!("{}：{error}", incoming.display())));
    }
    // 旧副本只影响磁盘占用，删除失败不改写本次替换结果，下次重新导入前会再清理。
    if let Err(error) = std::fs::remove_dir_all(&stale) {
        eprintln!("清理旧站点副本失败：{}：{error}", stale.display());
    }
    Ok(())
}

/// 列出根目录下的 HTML 入口候选，不递归子目录，避免误选深层页面。
pub fn entry_candidates(root: &Path) -> Result<Vec<String>, AppMessage> {
    let mut found = Vec::new();
    for item in std::fs::read_dir(root)
        .map_err(|error| AppMessage::new("errors.readDirFailed").detail(error))?
    {
        let item = item.map_err(|error| AppMessage::new("errors.readDirFailed").detail(error))?;
        // 链接可能指向受保护目录或形成环，入口扫描一律跳过。
        let file_type = item
            .file_type()
            .map_err(|error| AppMessage::new("errors.readDirFailed").detail(error))?;
        if !file_type.is_file() {
            continue;
        }
        let path = item.path();
        if !is_html(&path) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
            found.push(name.to_string());
        }
    }
    found.sort();
    Ok(found)
}

/// 预览阶段统计文件数量与体积，超限时提前拒绝，不产生任何复制。
pub fn measure(root: &Path) -> Result<(u64, u64), AppMessage> {
    let mut files = 0;
    let mut bytes = 0;
    walk(root, &mut |_, _, size| {
        files += 1;
        bytes += size;
        check_budget(files, bytes)
    })?;
    Ok((files, bytes))
}

/// 把整个目录复制为站点副本，保留相对结构以便样式、脚本和 JSON 请求继续解析。
pub fn copy_site(source: &Path, target: &Path) -> Result<(), AppMessage> {
    std::fs::create_dir_all(target)
        .map_err(|error| AppMessage::new("errors.createDirFailed").detail(error))?;
    let mut files = 0;
    let mut bytes = 0;
    walk(source, &mut |path, relative, size| {
        // 统计与复制分开进行，复制阶段再次校验，防止统计后文件继续变大。
        files += 1;
        bytes += size;
        check_budget(files, bytes)?;
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| AppMessage::new("errors.createDirFailed").detail(error))?;
        }
        std::fs::copy(path, &destination).map_err(|error| {
            AppMessage::new("errors.copyFileFailed").detail(format!("{}：{error}", path.display()))
        })?;
        Ok(())
    })
}

/// 单文件导入只复制入口本身，返回站点副本内的相对入口。
pub fn copy_entry(source: &Path, target: &Path) -> Result<String, AppMessage> {
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppMessage::new("errors.fileNameInvalid"))?;
    let size = std::fs::metadata(source)
        .map_err(|_| AppMessage::new("errors.readFileFailed"))?
        .len();
    check_budget(1, size)?;
    std::fs::create_dir_all(target)
        .map_err(|error| AppMessage::new("errors.createDirFailed").detail(error))?;
    std::fs::copy(source, target.join(name))
        .map_err(|error| AppMessage::new("errors.copyEntryFailed").detail(error))?;
    Ok(name.to_string())
}

/// 创建时确认入口确实是候选之一且位于所选目录内，防止前端伪造路径。
pub fn verify_entry(root: &Path, entry: &str, candidates: &[String]) -> Result<(), AppMessage> {
    if !candidates.iter().any(|candidate| candidate == entry) {
        return Err(AppMessage::new("errors.entryNotListed"));
    }
    let relative = Path::new(entry);
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(AppMessage::new("errors.entryInvalid"));
    }
    let root = root
        .canonicalize()
        .map_err(|_| AppMessage::new("errors.rootMissing"))?;
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|_| AppMessage::new("errors.entryMissing"))?;
    if !path.starts_with(&root) || !path.is_file() || !is_html(&path) {
        return Err(AppMessage::new("errors.entryNotHtml"));
    }
    Ok(())
}

/// 删除页面时清理站点副本，只处理 pages 根目录下与页面 ID 同名的一级目录。
pub fn remove_page(app: &AppHandle, id: &str) -> Result<(), AppMessage> {
    validate_id(id)?;
    let pages = pages_root(app)?;
    let target = pages.join(id);
    if target.parent() != Some(pages.as_path()) || !target.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(&target)
        .map_err(|error| AppMessage::new("errors.cleanupFailed").detail(error))
}

/// 只接受 html 与 htm 扩展名，大小写不敏感。
fn is_html(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| matches!(extension.to_lowercase().as_str(), "html" | "htm"))
}

/// 超限时给出可直接执行的提示，不返回难以理解的内部错误。
fn check_budget(files: u64, bytes: u64) -> Result<(), AppMessage> {
    if files > MAX_FILES {
        return Err(AppMessage::new("errors.tooManyFiles").arg("max", MAX_FILES));
    }
    if bytes > MAX_BYTES {
        return Err(AppMessage::new("errors.tooLarge").arg("max", MAX_BYTES / 1024 / 1024));
    }
    Ok(())
}

/// 深度优先遍历真实目录和普通文件，符号链接与目录联接一律跳过。
pub(crate) fn walk(
    root: &Path,
    visit: &mut impl FnMut(&Path, &Path, u64) -> Result<(), AppMessage>,
) -> Result<(), AppMessage> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let items = std::fs::read_dir(&directory).map_err(|error| {
            AppMessage::new("errors.readDirFailed")
                .detail(format!("{}：{error}", directory.display()))
        })?;
        for item in items {
            let item =
                item.map_err(|error| AppMessage::new("errors.readDirFailed").detail(error))?;
            // file_type 不跟随链接，链接因此既不会被复制也不会被递归。
            let file_type = item
                .file_type()
                .map_err(|error| AppMessage::new("errors.readDirFailed").detail(error))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = item.path();
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| AppMessage::new("errors.pathEscaped"))?;
                let size = item.metadata().map(|meta| meta.len()).unwrap_or(0);
                visit(&path, relative, size)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
