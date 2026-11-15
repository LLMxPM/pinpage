// 功能：将已鉴权的本机路径复制到暂存站点，锁定源文件并计算稳定内容摘要。
use crate::{
    errors::AppMessage,
    models::{PageSource, PresetFields, WindowPreset},
};
use pinpage_protocol::Failure;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

pub struct Prepared {
    pub source: PageSource,
    pub preset: Option<PresetFields>,
    pub presets: Vec<WindowPreset>,
    pub storage: Option<String>,
    pub digest: String,
}
/// 保留原生 code/args/detail，调用者无需解析本地化文案。
pub fn native(error: AppMessage) -> Failure {
    Failure::new(
        match error.code.as_str() {
            "errors.pinNotFound" => "pageNotFound",
            "errors.presetNotFound" => "presetNotFound",
            "errors.presetConflict" => "presetConflict",
            "errors.presetFieldsInvalid"
            | "errors.presetIdInvalid"
            | "errors.presetNameRange"
            | "errors.presetLimit"
            | "errors.savePathInvalid" => "invalidArgument",
            "errors.platformUnsupported" => "unsupported",
            "errors.exportExists" => "outputExists",
            _ => "executionFailed",
        },
        crate::i18n::render(crate::models::AppLocale::ZhCn, &error),
    )
    .detail(error)
}
/// 拒绝链接、目录联接及指向链接的祖先，复制过程不会把源码目录授权给页面。
fn regular(path: &Path) -> Result<std::fs::Metadata, Failure> {
    let meta = std::fs::symlink_metadata(path).map_err(invalid)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(invalid("来源含链接或目录联接"));
        }
    }
    if meta.file_type().is_symlink() {
        return Err(invalid("来源含符号链接"));
    }
    Ok(meta)
}
/// 路径必须已经由 CLI 解析为绝对路径；服务端不依赖自己的工作目录。
fn root(path: &Path) -> Result<PathBuf, Failure> {
    if !path.is_absolute() {
        return Err(invalid("来源必须是绝对路径"));
    }
    for ancestor in path.ancestors() {
        regular(ancestor)?;
    }
    path.canonicalize().map_err(invalid)
}
/// Windows 共享标志禁止复制期间写入和删除；目录句柄阻止祖先重命名。
fn locked(path: &Path, directory: bool) -> Result<std::fs::File, Failure> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options
            .share_mode(if directory { 3 } else { 1 })
            .custom_flags(if directory { 0x02200000 } else { 0x00200000 });
    }
    let file = options.open(path).map_err(invalid)?;
    let meta = file.metadata().map_err(invalid)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(invalid("复制时来源变为链接"));
        }
    }
    if meta.is_dir() != directory {
        return Err(invalid("来源类型变化"));
    }
    Ok(file)
}
/// 先持有所有目录与文件的句柄，再复制；总量沿用 4000 文件 / 256 MiB。
type LockedFiles = (Vec<std::fs::File>, Vec<(String, std::fs::File)>);
fn scan(root: &Path) -> Result<LockedFiles, Failure> {
    let mut stack = vec![root.to_owned()];
    let mut dirs = vec![];
    let mut files = vec![];
    let mut bytes = 0u64;
    while let Some(dir) = stack.pop() {
        regular(&dir)?;
        dirs.push(locked(&dir, true)?);
        for entry in std::fs::read_dir(&dir).map_err(invalid)? {
            let path = entry.map_err(invalid)?.path();
            let meta = regular(&path)?;
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(invalid)?
                    .to_str()
                    .ok_or_else(|| invalid("文件名不是 Unicode"))?
                    .replace('\\', "/");
                if relative.contains([':', '\0']) {
                    return Err(invalid("来源路径含非法字符"));
                }
                let file = locked(&path, false)?;
                bytes = bytes.saturating_add(file.metadata().map_err(invalid)?.len());
                files.push((relative, file));
                if files.len() > 4000 || bytes > 256 * 1024 * 1024 {
                    return Err(invalid("来源超过 4000 文件或 256 MiB"));
                }
            } else {
                return Err(invalid("来源含非常规文件"));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((dirs, files))
}
/// 目录、HTML 和页面包共用安全复制；更新调用方主动丢弃包快照与布局。
pub fn prepare(path: &Path, entry: Option<&str>, target: &Path) -> Result<Prepared, Failure> {
    let path = root(path)?;
    let meta = regular(&path)?;
    std::fs::create_dir_all(target).map_err(invalid)?;
    if meta.is_dir() {
        let (_dirs, mut files) = scan(&path)?;
        let candidates: Vec<_> = files
            .iter()
            .filter(|(p, _)| !p.contains('/') && html(Path::new(p)))
            .map(|(p, _)| p.clone())
            .collect();
        let entry = match entry {
            Some(p) if candidates.iter().any(|c| c == p) => p.to_owned(),
            None if candidates.len() == 1 => candidates[0].clone(),
            _ => {
                return Err(invalid("必须指定根目录 HTML 入口")
                    .detail(serde_json::json!({"candidates":candidates})))
            }
        };
        let digest = copy(&mut files, target)?;
        // 拒绝复制期间新出现的文件，不把部分来源当成一致快照。
        let (_, after) = scan(&path)?;
        if after
            .iter()
            .map(|(p, _)| p)
            .ne(files.iter().map(|(p, _)| p))
        {
            return Err(Failure::new("sourceChanged", "复制期间来源发生变化"));
        }
        Ok(Prepared {
            source: PageSource::Bundled { entry },
            preset: None,
            presets: vec![],
            storage: None,
            digest,
        })
    } else if html(&path) {
        if entry.is_some() {
            return Err(invalid("--entry 只用于目录"));
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| invalid("文件名不合法"))?
            .to_owned();
        let mut files = vec![(name.clone(), locked(&path, false)?)];
        let digest = copy(&mut files, target)?;
        Ok(Prepared {
            source: PageSource::Bundled { entry: name },
            preset: None,
            presets: vec![],
            storage: None,
            digest,
        })
    } else if path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("pinpage"))
    {
        if entry.is_some() {
            return Err(invalid("页面包入口由清单定义"));
        }
        let mut file = locked(&path, false)?;
        if file.metadata().map_err(invalid)?.len() > 300 * 1024 * 1024 {
            return Err(invalid("页面包过大"));
        }
        let mut hash = Sha256::new();
        std::io::copy(&mut file, &mut HashWriter(&mut hash)).map_err(invalid)?;
        let extracted = crate::package::extract(&path, target).map_err(native)?;
        Ok(Prepared {
            source: PageSource::Bundled {
                entry: extracted.entry,
            },
            preset: extracted.window_preset,
            presets: extracted.window_presets,
            storage: extracted.storage,
            digest: format!("{:x}", hash.finalize()),
        })
    } else {
        Err(invalid("仅支持目录、HTML 或 .pinpage"))
    }
}
/// 数据流按块复制并累计摘要，避免将整个站点载入内存。
fn copy(files: &mut [(String, std::fs::File)], target: &Path) -> Result<String, Failure> {
    let mut hash = Sha256::new();
    let mut total = 0u64;
    for (name, file) in files {
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        let dest = target.join(&name);
        std::fs::create_dir_all(dest.parent().unwrap()).map_err(invalid)?;
        let mut output = std::fs::File::create(dest).map_err(invalid)?;
        let before = file.metadata().map_err(invalid)?;
        if before.len() > 256 * 1024 * 1024 {
            return Err(invalid("文件过大"));
        }
        hash.update(before.len().to_le_bytes());
        let mut buffer = [0u8; 65536];
        loop {
            let len = file.read(&mut buffer).map_err(invalid)?;
            if len == 0 {
                break;
            }
            total += len as u64;
            if total > 256 * 1024 * 1024 {
                return Err(invalid("来源过大"));
            }
            output.write_all(&buffer[..len]).map_err(invalid)?;
            hash.update(&buffer[..len]);
        }
        output.sync_all().map_err(invalid)?;
        let after = file.metadata().map_err(invalid)?;
        file.rewind().map_err(invalid)?;
        if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
            return Err(Failure::new("sourceChanged", "来源在复制期间变化"));
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}
struct HashWriter<'a>(&'a mut Sha256);
impl Write for HashWriter<'_> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.update(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
/// 入口扩展名大小写不敏感。
fn html(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
}
/// 来源错误不混入配置写入错误域。
fn invalid(error: impl ToString) -> Failure {
    Failure::new("invalidSource", error)
}
#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
