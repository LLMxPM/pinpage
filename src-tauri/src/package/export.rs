// 功能：将页面站点、存储快照和当前窗口布局导出为标准 .pinpage。
use super::{validate_manifest, Manifest, MANIFEST, PACKAGE_VERSION, STORAGE};
use crate::{
    errors::AppMessage,
    import,
    models::{PageSource, PinConfig, PresetFields},
};
use std::{
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::AppHandle;

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
    let fields = crate::windows::presets::snapshot_fields(app, pin)?;
    write_dir(&site, &pin.name, entry, storage, Some(&fields), target)
}

/// 不依赖应用句柄的打包实现，便于直接对目录做往返测试。
pub fn write_dir(
    site: &Path,
    name: &str,
    entry: &str,
    storage: Option<&str>,
    window_preset: Option<&PresetFields>,
    target: &Path,
) -> Result<u64, AppMessage> {
    let (files, _) = import::measure(site)?;
    let mut manifest = Manifest {
        schema_version: PACKAGE_VERSION,
        name: name.to_string(),
        entry: entry.to_string(),
        exported_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or_default(),
        files,
        window_preset: window_preset.cloned(),
    };
    validate_manifest(&mut manifest)?;
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
