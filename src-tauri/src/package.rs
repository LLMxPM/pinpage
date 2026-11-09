// 功能：把导入页的站点副本与页面数据打包成 .pinpage，并支持解包回站点副本。
use crate::{errors::AppMessage, models::PresetFields};
use serde::{Deserialize, Serialize};
use std::{
    io::{Cursor, Read, Seek},
    path::{Component, Path},
};
mod export;
pub use export::write;
#[cfg(test)]
use export::write_dir;

/// 包格式版本，与配置版本独立演进。
const PACKAGE_VERSION: u32 = 1;
/// 包内清单与数据快照的固定位置。
const MANIFEST: &str = "manifest.json";
const STORAGE: &str = "data/storage.json";
/// 解包时的文件数量与体积上限，与目录导入保持一致。
const MAX_FILES: u64 = 4000;
const MAX_BYTES: u64 = 256 * 1024 * 1024;
/// 随包页面数据的体积上限；IndexedDB 与内嵌二进制会让数据明显变大。
const MAX_STORAGE: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub name: String,
    pub entry: String,
    /// Unix 秒，避免为导出时间引入日期库。
    pub exported_at: u64,
    pub files: u64,
    /// 可选的默认窗口布局；旧版包缺失时沿用创建表单的设置，不导入全局预设记录。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_preset: Option<PresetFields>,
}

/// 解包结果：相对入口与随包页面数据。
pub struct Extracted {
    pub entry: String,
    pub storage: Option<String>,
    pub window_preset: Option<PresetFields>,
}

/// 只读取清单，用于导入前的预览与校验。
pub fn read_manifest(path: &Path) -> Result<Manifest, AppMessage> {
    let file = std::fs::File::open(path)
        .map_err(|error| AppMessage::new("errors.readPackageFailed").detail(error))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| AppMessage::new("errors.packageInvalid").detail(error))?;
    archive_manifest(&mut archive)
}

/// 内置示例与外部页面包使用同一清单解析器，内存来源不需要创建临时包文件。
pub fn read_manifest_bytes(bytes: &[u8]) -> Result<Manifest, AppMessage> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| AppMessage::new("errors.packageInvalid").detail(error))?;
    archive_manifest(&mut archive)
}

/// 在有界清单中读取入口和窗口布局；解析后才允许创建站点文件。
fn archive_manifest<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<Manifest, AppMessage> {
    let mut entry = archive
        .by_name(MANIFEST)
        .map_err(|_| AppMessage::new("errors.packageNoManifest"))?;
    if entry.size() > 256 * 1024 {
        return Err(AppMessage::new("errors.packageManifestInvalid"));
    }
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|error| AppMessage::new("errors.packageManifestInvalid").detail(error))?;
    let mut manifest: Manifest = serde_json::from_str(&text)
        .map_err(|error| AppMessage::new("errors.packageManifestInvalid").detail(error))?;
    validate_manifest(&mut manifest)?;
    Ok(manifest)
}

/// 版本与入口严格校验；随包预设检查数值边界但允许跨平台传输，应用时再做能力回退。
fn validate_manifest(manifest: &mut Manifest) -> Result<(), AppMessage> {
    if manifest.schema_version != PACKAGE_VERSION {
        return Err(AppMessage::new("errors.packageVersion"));
    }
    if !valid_entry(&manifest.entry) {
        return Err(AppMessage::new("errors.packageEntryInvalid"));
    }
    if let Some(fields) = &mut manifest.window_preset {
        if !crate::windows::limits::contains(fields.width, fields.height)
            || !fields.fx.is_finite()
            || !fields.fy.is_finite()
            || !(0. ..=1.).contains(&fields.fx)
            || !(0. ..=1.).contains(&fields.fy)
        {
            return Err(AppMessage::new("errors.presetFieldsInvalid"));
        }
        crate::windows::presets::normalize(fields);
    }
    Ok(())
}

/// 解包到站点副本，拒绝越界路径、超量文件和非 HTML 入口。
pub fn extract(source: &Path, target: &Path) -> Result<Extracted, AppMessage> {
    let file = std::fs::File::open(source)
        .map_err(|error| AppMessage::new("errors.readPackageFailed").detail(error))?;
    let archive = zip::ZipArchive::new(file)
        .map_err(|error| AppMessage::new("errors.packageInvalid").detail(error))?;
    extract_archive(archive, target)
}

/// 从内嵌标准页面包解出站点，沿用外部包的路径、体积和数据校验。
pub fn extract_bytes(bytes: &[u8], target: &Path) -> Result<Extracted, AppMessage> {
    let archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| AppMessage::new("errors.packageInvalid").detail(error))?;
    extract_archive(archive, target)
}

/// 统一解包磁盘和内存中的 ZIP，站点资源允许安全的子目录结构。
fn extract_archive<R: Read + Seek>(
    mut archive: zip::ZipArchive<R>,
    target: &Path,
) -> Result<Extracted, AppMessage> {
    let manifest = archive_manifest(&mut archive)?;
    std::fs::create_dir_all(target)
        .map_err(|error| AppMessage::new("errors.createDirFailed").detail(error))?;
    let mut files = 0;
    let mut bytes = 0;
    let mut storage = None;
    for index in 0..archive.len() {
        let mut item = archive
            .by_index(index)
            .map_err(|error| AppMessage::new("errors.packageInvalid").detail(error))?;
        if !item.is_file() {
            continue;
        }
        // enclosed_name 拒绝绝对路径、盘符与 .. 段，是解包的主要防线。
        let Some(name) = item.enclosed_name() else {
            return Err(AppMessage::new("errors.packageUnsafePath"));
        };
        if name == Path::new(STORAGE) {
            if item.size() > MAX_STORAGE {
                return Err(AppMessage::new("errors.packageDataTooLarge"));
            }
            let mut text = String::new();
            item.read_to_string(&mut text)
                .map_err(|error| AppMessage::new("errors.packageDataInvalid").detail(error))?;
            // 提前校验为合法 JSON，恢复脚本会再次解析后再嵌入页面。
            serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|error| AppMessage::new("errors.packageDataInvalid").detail(error))?;
            storage = Some(text);
            continue;
        }
        let Some(relative) = name.strip_prefix("site").ok() else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        files += 1;
        bytes += item.size();
        if files > MAX_FILES || bytes > MAX_BYTES {
            return Err(AppMessage::new("errors.packageTooLarge"));
        }
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| AppMessage::new("errors.createDirFailed").detail(error))?;
        }
        let mut buffer = Vec::new();
        item.read_to_end(&mut buffer)
            .map_err(|error| AppMessage::new("errors.packageInvalid").detail(error))?;
        std::fs::write(&destination, &buffer).map_err(|error| {
            AppMessage::new("errors.writeFileFailed")
                .detail(format!("{}：{error}", destination.display()))
        })?;
    }
    if !target.join(&manifest.entry).is_file() {
        return Err(AppMessage::new("errors.packageNoEntry"));
    }
    Ok(Extracted {
        entry: manifest.entry,
        storage,
        window_preset: manifest.window_preset,
    })
}

/// 入口必须是纯相对路径的 HTML，避免清单被构造成越界路径。
fn valid_entry(entry: &str) -> bool {
    let path = Path::new(entry);
    !path.as_os_str().is_empty()
        && !entry.contains(['\\', ':', '\0'])
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| matches!(extension.to_lowercase().as_str(), "html" | "htm"))
}

#[cfg(test)]
#[path = "package_tests.rs"]
mod tests;
