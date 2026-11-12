// 功能：对 skill 目录与 ZIP 执行路径、链接和大小预算校验。
use crate::{invalid, Bundle, ID};
use pinpage_protocol::Failure;
use std::{
    io::{Read, Write},
    path::{Component, Path},
};

/// 拒绝盘符、ADS、反斜线、越界、空组件以及安装记录注入。
pub fn safe_path(value: &str) -> Result<(), Failure> {
    if value.is_empty()
        || value.contains(['\\', ':', '\0', '*', '?', '"', '<', '>', '|'])
        || value.chars().any(char::is_control)
        || value.split('/').any(|part| {
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
                || (stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
        || value
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' ']))
        || Path::new(value)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
        || value == crate::RECORD
    {
        return Err(invalid("skill 包内路径不合法"));
    }
    Ok(())
}
/// Windows 联接也是重解析点，不能只检查 is_symlink。
pub fn regular(path: &Path) -> Result<std::fs::Metadata, Failure> {
    let meta = std::fs::symlink_metadata(path).map_err(invalid)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(invalid("拒绝链接或目录联接"));
        }
    }
    if meta.file_type().is_symlink() {
        return Err(invalid("拒绝符号链接"));
    }
    Ok(meta)
}
/// 安装目标的每级父目录也不能通过链接重定向。
pub fn boundary(path: &Path) -> Result<(), Failure> {
    for ancestor in path.ancestors() {
        if ancestor.exists() {
            regular(ancestor)?;
        }
    }
    Ok(())
}
/// 内核文件锁随句柄关闭或进程退出释放，避免两个 CLI 混合交换同一技能。
pub fn lock(root: &Path) -> Result<std::fs::File, Failure> {
    std::fs::create_dir_all(root).map_err(invalid)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(".pinpage-pages.lock"))
        .map_err(invalid)?;
    file.try_lock()
        .map_err(|_| Failure::new("skillTargetConflict", "此目录已有进行中的技能操作"))?;
    Ok(file)
}

/// 备份完整原目录，包括安装记录、空目录以及入口已删除的情况。
pub fn backup_tree(source: &Path, target: &Path) -> Result<(), Failure> {
    boundary(source)?;
    boundary(target)?;
    std::fs::create_dir_all(target).map_err(invalid)?;
    for entry in std::fs::read_dir(source).map_err(invalid)? {
        let path = entry.map_err(invalid)?.path();
        let meta = regular(&path)?;
        let output = target.join(path.file_name().unwrap());
        if meta.is_dir() {
            backup_tree(&path, &output)?;
        } else {
            std::fs::copy(&path, &output).map_err(invalid)?;
        }
    }
    Ok(())
}
/// 读取受预算限制的普通目录；受管理记录不属于分发文件。
pub fn read_tree(root: &Path) -> Result<Bundle, Failure> {
    boundary(root)?;
    let mut bundle = Bundle::new();
    visit(root, root, &mut bundle)?;
    Ok(bundle)
}
/// 每次进入子目录前检查重解析点，不跟随链接递归。
fn visit(root: &Path, path: &Path, bundle: &mut Bundle) -> Result<(), Failure> {
    regular(path)?;
    for entry in std::fs::read_dir(path).map_err(invalid)? {
        let path = entry.map_err(invalid)?.path();
        let meta = regular(&path)?;
        if meta.is_dir() {
            visit(root, &path, bundle)?;
        } else if meta.is_file() {
            let name = path
                .strip_prefix(root)
                .map_err(invalid)?
                .to_str()
                .ok_or_else(|| invalid("文件名不是 Unicode"))?
                .replace('\\', "/");
            if name == crate::RECORD {
                continue;
            }
            safe_path(&name)?;
            if meta.len() > 4 * 1024 * 1024 {
                return Err(invalid("skill 文件过大"));
            }
            bundle.insert(name, std::fs::read(path).map_err(invalid)?);
            budget(bundle)?;
        } else {
            return Err(invalid("skill 含非常规文件"));
        }
    }
    Ok(())
}
/// ZIP 严格要求单一顶层目录，解压前检查重复、大小和路径。
pub fn read_zip(path: &Path) -> Result<Bundle, Failure> {
    regular(path)?;
    let mut archive =
        zip::ZipArchive::new(std::fs::File::open(path).map_err(invalid)?).map_err(invalid)?;
    if archive.len() > 256 {
        return Err(invalid("skill 文件过多"));
    }
    let mut bundle = Bundle::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(invalid)?;
        let name = file
            .name()
            .strip_prefix(&format!("{ID}/"))
            .ok_or_else(|| invalid("ZIP 必须包含单一 pinpage-pages 顶层目录"))?
            .to_owned();
        if file.is_dir() {
            if !name.is_empty() {
                safe_path(name.trim_end_matches('/'))?;
            }
            continue;
        }
        safe_path(&name)?;
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(invalid("ZIP 不允许链接"));
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(invalid)?;
        if bytes.len() > 4 * 1024 * 1024 || bundle.insert(name, bytes).is_some() {
            return Err(invalid("ZIP 文件过大或重名"));
        }
        budget(&bundle)?;
    }
    Ok(bundle)
}
/// 总预算限制外部资源造成的内存与磁盘占用。
fn budget(bundle: &Bundle) -> Result<(), Failure> {
    if bundle.len() > 128 || bundle.values().map(Vec::len).sum::<usize>() > 16 * 1024 * 1024 {
        return Err(invalid("skill 超出 128 文件 / 16 MiB 预算"));
    }
    Ok(())
}
/// 入口最后写入，暂存期不出现有效 SKILL.md。
pub fn write_tree(root: &Path, bundle: &Bundle) -> Result<(), Failure> {
    std::fs::create_dir_all(root).map_err(invalid)?;
    for (name, bytes) in bundle.iter().filter(|(p, _)| p.as_str() != "SKILL.md") {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(invalid)?;
        std::fs::write(path, bytes).map_err(invalid)?;
    }
    std::fs::write(root.join(".skill-entry"), &bundle["SKILL.md"]).map_err(invalid)
}
/// 导出 ZIP 与目录采用同一资源集合。
pub fn write_zip(path: &Path, bundle: &Bundle) -> Result<(), Failure> {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).map_err(invalid)?);
    for (name, bytes) in bundle {
        zip.start_file(
            format!("{ID}/{name}"),
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .map_err(invalid)?;
        zip.write_all(bytes).map_err(invalid)?;
    }
    zip.finish().map_err(invalid)?;
    Ok(())
}
