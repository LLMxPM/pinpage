// 功能：把导入页的站点副本与页面数据打包成 .pinpage，并支持解包回站点副本。
use crate::{
    errors::AppMessage,
    import,
    models::{PageSource, PinConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Component, Path},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::AppHandle;

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
}

/// 解包结果：相对入口与随包页面数据。
pub struct Extracted {
    pub entry: String,
    pub storage: Option<String>,
}

/// 把站点副本、页面数据与清单写成 ZIP 包，返回写入的文件数。
pub fn write(
    app: &AppHandle,
    pin: &PinConfig,
    storage: Option<&str>,
    target: &Path,
) -> Result<u64, AppMessage> {
    let PageSource::Bundled { entry } = &pin.source else {
        return Err(AppMessage::new("errors.exportNeedsImport"));
    };
    let site = import::site_root(app, &pin.id)?;
    if !site.is_dir() {
        return Err(AppMessage::new("errors.importContentLost"));
    }
    write_dir(&site, &pin.name, entry, storage, target)
}

/// 不依赖应用句柄的打包实现，便于直接对目录做往返测试。
pub fn write_dir(
    site: &Path,
    name: &str,
    entry: &str,
    storage: Option<&str>,
    target: &Path,
) -> Result<u64, AppMessage> {
    let (files, _) = import::measure(site)?;
    let manifest = Manifest {
        schema_version: PACKAGE_VERSION,
        name: name.to_string(),
        entry: entry.to_string(),
        exported_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or_default(),
        files,
    };
    let file = std::fs::File::create(target)
        .map_err(|error| AppMessage::new("errors.createExportFailed").detail(error))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(MANIFEST, options)
        .map_err(|error| AppMessage::new("errors.writePackageFailed").detail(error))?;
    let manifest = serde_json::to_string_pretty(&manifest)
        .map_err(|error| AppMessage::new("errors.writePackageFailed").detail(error))?;
    zip.write_all(manifest.as_bytes())
        .map_err(|error| AppMessage::new("errors.writePackageFailed").detail(error))?;
    import::walk(site, &mut |path, relative, _| {
        // 包内统一使用正斜杠，跨平台解包时不必再处理分隔符差异。
        let name = format!("site/{}", relative.to_string_lossy().replace('\\', "/"));
        zip.start_file(name, options)
            .map_err(|error| AppMessage::new("errors.writePackageFailed").detail(error))?;
        let mut source = std::fs::File::open(path)
            .map_err(|error| AppMessage::new("errors.copyFileFailed").detail(error))?;
        std::io::copy(&mut source, &mut zip)
            .map_err(|error| AppMessage::new("errors.writePackageFailed").detail(error))?;
        Ok(())
    })?;
    if let Some(payload) = storage {
        zip.start_file(STORAGE, options)
            .map_err(|error| AppMessage::new("errors.writePackageFailed").detail(error))?;
        zip.write_all(payload.as_bytes())
            .map_err(|error| AppMessage::new("errors.writePackageFailed").detail(error))?;
    }
    zip.finish()
        .map_err(|error| AppMessage::new("errors.writePackageFailed").detail(error))?;
    Ok(files)
}

/// 只读取清单，用于导入前的预览与校验。
pub fn read_manifest(path: &Path) -> Result<Manifest, AppMessage> {
    let file = std::fs::File::open(path)
        .map_err(|error| AppMessage::new("errors.readPackageFailed").detail(error))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| AppMessage::new("errors.packageInvalid").detail(error))?;
    let mut entry = archive
        .by_name(MANIFEST)
        .map_err(|_| AppMessage::new("errors.packageNoManifest"))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|error| AppMessage::new("errors.packageManifestInvalid").detail(error))?;
    let manifest: Manifest = serde_json::from_str(&text)
        .map_err(|error| AppMessage::new("errors.packageManifestInvalid").detail(error))?;
    if manifest.schema_version != PACKAGE_VERSION {
        return Err(AppMessage::new("errors.packageVersion"));
    }
    if !valid_entry(&manifest.entry) {
        return Err(AppMessage::new("errors.packageEntryInvalid"));
    }
    Ok(manifest)
}

/// 解包到站点副本，拒绝越界路径、超量文件和非 HTML 入口。
pub fn extract(source: &Path, target: &Path) -> Result<Extracted, AppMessage> {
    let manifest = read_manifest(source)?;
    let file = std::fs::File::open(source)
        .map_err(|error| AppMessage::new("errors.readPackageFailed").detail(error))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| AppMessage::new("errors.packageInvalid").detail(error))?;
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
    })
}

/// 入口必须是纯相对路径的 HTML，避免清单被构造成越界路径。
fn valid_entry(entry: &str) -> bool {
    let path = Path::new(entry);
    !path.as_os_str().is_empty()
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
