// 功能：验证来源兑换的入口白名单边界，覆盖不接触应用数据目录的纯校验分支。
use super::*;
use crate::import::ImportGrant;
use crate::models::ImportMode;
use std::path::PathBuf;

/// 临时来源目录，内含一个真实 HTML 文件，用于区分“入口名合法”与“入口确实存在”。
struct Fixture {
    root: PathBuf,
    file: PathBuf,
}

impl Fixture {
    /// 在系统临时目录创建随机前缀的来源目录，避免与其他测试相互干扰。
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("pinpage-source-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("index.html");
        std::fs::write(&file, "<html></html>").unwrap();
        Self { root, file }
    }
}

impl Drop for Fixture {
    /// 只删除本测试创建的随机目录，不沿目录联接递归删除用户内容。
    fn drop(&mut self) {
        let (Ok(temporary_root), Ok(target)) = (
            std::env::temp_dir().canonicalize(),
            self.root.canonicalize(),
        ) else {
            return;
        };
        let safe_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.starts_with("pinpage-source-"));
        if target.parent() == Some(temporary_root.as_path()) && safe_name {
            let _ = std::fs::remove_dir_all(target);
        }
    }
}

/// 构造票据内容；示例标识固定取默认值，测试只关心入口白名单。
fn grant(mode: ImportMode, path: PathBuf, candidates: &[&str]) -> ImportGrant {
    ImportGrant {
        path,
        mode,
        candidates: candidates
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        example: Default::default(),
    }
}

#[test]
/// 单文件导入的入口必须就是所选文件本身，且文件仍存在。
fn file_mode_only_accepts_the_picked_file() {
    let fixture = Fixture::new();
    let picked = grant(ImportMode::File, fixture.file.clone(), &[]);
    assert!(verify_entry(&picked, "index.html").is_ok());
    assert_eq!(
        verify_entry(&picked, "other.html").unwrap_err().code,
        "errors.entryMismatch"
    );
    let missing = grant(ImportMode::File, fixture.root.join("missing.html"), &[]);
    assert_eq!(
        verify_entry(&missing, "missing.html").unwrap_err().code,
        "errors.entryMismatch"
    );
}

#[test]
/// 页面包与内置示例只能使用签发时记录的候选，前端不能替换来源内容。
fn issued_sources_are_limited_to_recorded_candidates() {
    let fixture = Fixture::new();
    let package = grant(
        ImportMode::Package,
        fixture.root.join("demo.pinpage.zip"),
        &["index.html"],
    );
    assert!(verify_entry(&package, "index.html").is_ok());
    assert_eq!(
        verify_entry(&package, "../index.html").unwrap_err().code,
        "errors.entryMismatchSource"
    );
    let example = grant(ImportMode::Example, fixture.root.clone(), &["index.html"]);
    assert_eq!(
        verify_entry(&example, "evil.html").unwrap_err().code,
        "errors.entryMismatchSource"
    );
}

#[test]
/// 目录导入委托给导入器的入口校验，越出所选目录或未列入候选都会被拒绝。
fn directory_mode_reuses_import_validation() {
    let fixture = Fixture::new();
    let scanned = grant(ImportMode::Directory, fixture.root.clone(), &["index.html"]);
    assert!(verify_entry(&scanned, "index.html").is_ok());
    assert_eq!(
        verify_entry(&scanned, "notes.html").unwrap_err().code,
        "errors.entryNotListed"
    );
    let escaped = grant(
        ImportMode::Directory,
        fixture.root.clone(),
        &["../index.html"],
    );
    assert!(verify_entry(&escaped, "../index.html").is_err());
}
