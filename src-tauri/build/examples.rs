// 功能：将标准 manifest.json + site/ 源码目录打包为可复现的 .pinpage，供构建和手动分发共用。
use std::{
    error::Error,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// 打包集合中的全部页面；返回稳定排序的标识和包路径，不把生成文件写回源码目录。
pub fn pack_all(source: &Path, output: &Path) -> Result<Vec<(String, PathBuf)>> {
    fs::create_dir_all(output)?;
    let mut directories = fs::read_dir(source)?.collect::<std::io::Result<Vec<_>>>()?;
    directories.sort_by_key(|entry| entry.file_name());
    let mut packages = Vec::new();
    for directory in directories {
        if !directory.file_type()?.is_dir() || !directory.path().join("manifest.json").is_file() {
            continue;
        }
        let id = directory.file_name().to_string_lossy().into_owned();
        if !id
            .bytes()
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-')
        {
            return Err(format!("示例目录名必须使用小写字母、数字或短横线：{id}").into());
        }
        let target = output.join(format!("{id}.pinpage"));
        pack(&directory.path(), &target)?;
        packages.push((id, target));
    }
    if packages.is_empty() {
        return Err("未找到包含 manifest.json 的示例源码目录".into());
    }
    Ok(packages)
}

/// 只收集站点内的真实文件，拒绝链接；固定顺序与时间戳使同一份源码生成相同字节。
fn collect(root: &Path, relative: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(root.join(relative))?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let metadata = fs::symlink_metadata(entry.path())?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(format!(
                    "示例资源不允许目录联接或重解析点：{}",
                    entry.path().display()
                )
                .into());
            }
        }
        let path = relative.join(entry.file_name());
        if metadata.file_type().is_symlink() {
            return Err(format!("示例资源不允许符号链接：{}", entry.path().display()).into());
        } else if metadata.is_dir() {
            collect(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// 从标准包源码打包；补充统计字段，保留默认窗口预设等清单扩展，不携带个人存储数据。
fn pack(source: &Path, target: &Path) -> Result<()> {
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(source.join("manifest.json"))?)?;
    let entry = manifest["entry"].as_str().ok_or("示例清单缺少入口")?;
    let entry_path = Path::new(entry);
    if manifest["schemaVersion"] != 1
        || entry_path.as_os_str().is_empty()
        || entry.contains(['\\', ':'])
        || !entry_path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        || !entry_path
            .extension()
            .and_then(|part| part.to_str())
            .is_some_and(|extension| matches!(extension.to_lowercase().as_str(), "html" | "htm"))
    {
        return Err("示例清单版本或 HTML 入口无效".into());
    }
    let site = source.join("site");
    let mut files = Vec::new();
    collect(&site, Path::new(""), &mut files)?;
    if !files.iter().any(|path| path == entry_path) {
        return Err(format!("示例入口不存在：{entry}").into());
    }
    let bytes = files.iter().try_fold(0_u64, |total, path| {
        fs::metadata(site.join(path)).map(|metadata| total + metadata.len())
    })?;
    if files.len() > 4000 || bytes > 256 * 1024 * 1024 {
        return Err("示例资源超过页面包数量或体积上限".into());
    }
    manifest["files"] = files.len().into();
    manifest["exportedAt"] = 0.into();
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    archive.start_file("manifest.json", options)?;
    archive.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    for relative in files {
        let name = format!("site/{}", relative.to_string_lossy().replace('\\', "/"));
        archive.start_file(name, options)?;
        archive.write_all(&fs::read(site.join(relative))?)?;
    }
    let bytes = archive.finish()?.into_inner();
    // 构建期间避免无内容变化的写入引发额外重编译。
    if fs::read(target).ok().as_deref() != Some(bytes.as_slice()) {
        fs::write(target, bytes)?;
    }
    Ok(())
}
