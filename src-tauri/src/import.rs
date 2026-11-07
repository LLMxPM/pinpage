// 功能：把用户选择的网页复制到应用数据目录，形成不依赖原始路径的导入页。
use crate::models::ImportMode;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Manager};

/// 单次导入的文件数量上限，避免把整个资料目录复制进应用数据目录。
const MAX_FILES: u64 = 4000;
/// 单次导入的总体积上限，页面属于轻量工具，超出时应先精简目录。
const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// 随包页面数据的暂存文件名，窗口首次创建时读取并立即删除。
pub const PENDING_DATA: &str = "pending-data.json";

/// 原生选择器签发的导入暂存，校验通过后才会真正复制文件。
#[derive(Clone, Debug)]
pub struct ImportGrant {
    /// 目录导入时为所选目录，单文件导入时为所选文件。
    pub path: PathBuf,
    pub mode: ImportMode,
    /// 目录导入时记录根级 HTML 候选，创建时只接受其中之一。
    pub candidates: Vec<String>,
}

/// 应用数据目录下的导入页根目录，每个页面一个以 ID 命名的子目录。
pub fn pages_root(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join("pages"))
        .map_err(|error| format!("无法定位应用数据目录：{error}"))
}

/// 页面 ID 必须是单级普通目录名，防止配置文件被改写后越出 pages 根目录。
fn validate_id(id: &str) -> Result<(), String> {
    let mut parts = Path::new(id).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err("页面 ID 不合法".into()),
    }
}

/// 页面的站点副本根目录；资源协议与删除操作都以它为唯一边界。
pub fn site_root(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    validate_id(id)?;
    Ok(pages_root(app)?.join(id).join("site"))
}

/// 列出根目录下的 HTML 入口候选，不递归子目录，避免误选深层页面。
pub fn entry_candidates(root: &Path) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for item in std::fs::read_dir(root).map_err(|error| format!("无法读取所选目录：{error}"))?
    {
        let item = item.map_err(|error| error.to_string())?;
        // 链接可能指向受保护目录或形成环，入口扫描一律跳过。
        let file_type = item.file_type().map_err(|error| error.to_string())?;
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
pub fn measure(root: &Path) -> Result<(u64, u64), String> {
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
pub fn copy_site(source: &Path, target: &Path) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|error| format!("无法创建导入目录：{error}"))?;
    let mut files = 0;
    let mut bytes = 0;
    walk(source, &mut |path, relative, size| {
        // 统计与复制分开进行，复制阶段再次校验，防止统计后文件继续变大。
        files += 1;
        bytes += size;
        check_budget(files, bytes)?;
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::copy(path, &destination)
            .map(|_| ())
            .map_err(|error| format!("复制 {} 失败：{error}", path.display()))
    })
}

/// 单文件导入只复制入口本身，返回站点副本内的相对入口。
pub fn copy_entry(source: &Path, target: &Path) -> Result<String, String> {
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("所选文件名无效")?;
    let size = std::fs::metadata(source)
        .map_err(|_| "无法读取所选文件")?
        .len();
    check_budget(1, size)?;
    std::fs::create_dir_all(target).map_err(|error| format!("无法创建导入目录：{error}"))?;
    std::fs::copy(source, target.join(name))
        .map_err(|error| format!("复制入口文件失败：{error}"))?;
    Ok(name.to_string())
}

/// 创建时确认入口确实是候选之一且位于所选目录内，防止前端伪造路径。
pub fn verify_entry(root: &Path, entry: &str, candidates: &[String]) -> Result<(), String> {
    if !candidates.iter().any(|candidate| candidate == entry) {
        return Err("入口不在导入清单中，请重新选择".into());
    }
    let relative = Path::new(entry);
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err("入口路径不合法".into());
    }
    let root = root.canonicalize().map_err(|_| "所选目录已失效")?;
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|_| "入口文件不存在")?;
    if !path.starts_with(&root) || !path.is_file() || !is_html(&path) {
        return Err("入口必须是所选目录内的 HTML 文件".into());
    }
    Ok(())
}

/// 删除页面时清理站点副本，只处理 pages 根目录下与页面 ID 同名的一级目录。
pub fn remove_page(app: &AppHandle, id: &str) -> Result<(), String> {
    validate_id(id)?;
    let pages = pages_root(app)?;
    let target = pages.join(id);
    if target.parent() != Some(pages.as_path()) || !target.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(&target).map_err(|error| format!("清理导入内容失败：{error}"))
}

/// 只接受 html 与 htm 扩展名，大小写不敏感。
fn is_html(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| matches!(extension.to_lowercase().as_str(), "html" | "htm"))
}

/// 超限时给出可直接执行的提示，不返回难以理解的内部错误。
fn check_budget(files: u64, bytes: u64) -> Result<(), String> {
    if files > MAX_FILES {
        return Err(format!("导入文件超过 {MAX_FILES} 个，请先精简目录再导入"));
    }
    if bytes > MAX_BYTES {
        return Err(format!(
            "导入体积超过 {} MiB，请先精简目录再导入",
            MAX_BYTES / 1024 / 1024
        ));
    }
    Ok(())
}

/// 深度优先遍历真实目录和普通文件，符号链接与目录联接一律跳过。
pub(crate) fn walk(
    root: &Path,
    visit: &mut impl FnMut(&Path, &Path, u64) -> Result<(), String>,
) -> Result<(), String> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let items = std::fs::read_dir(&directory)
            .map_err(|error| format!("无法读取 {}：{error}", directory.display()))?;
        for item in items {
            let item = item.map_err(|error| error.to_string())?;
            // file_type 不跟随链接，链接因此既不会被复制也不会被递归。
            let file_type = item.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                continue;
            }
            let path = item.path();
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let relative = path.strip_prefix(root).map_err(|_| "导入路径越界")?;
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
