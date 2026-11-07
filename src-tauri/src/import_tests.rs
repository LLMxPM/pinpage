// 功能：验证导入目录的入口扫描、结构复制与链接跳过边界。
use super::*;

struct Fixture(PathBuf);

impl Fixture {
    /// 创建包含根级入口、子目录页面和普通资源的独立临时目录。
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pinpage-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(path.join("site/子目录")).unwrap();
        std::fs::write(path.join("site/index.html"), "<html></html>").unwrap();
        std::fs::write(path.join("site/app.htm"), "<html></html>").unwrap();
        std::fs::write(path.join("site/子目录/deep.html"), "<html></html>").unwrap();
        std::fs::write(path.join("site/notes.txt"), "x").unwrap();
        std::fs::create_dir_all(path.join("out")).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    /// 只清理本测试创建的随机目录，绝不沿目录联接递归删除目标内容。
    fn drop(&mut self) {
        let (Ok(temporary_root), Ok(target)) =
            (std::env::temp_dir().canonicalize(), self.0.canonicalize())
        else {
            return;
        };
        let safe_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.starts_with("pinpage-import-"));
        if target.parent() == Some(temporary_root.as_path()) && safe_name {
            let _ = std::fs::remove_dir_all(target);
        }
    }
}

#[test]
/// 入口候选只包含根目录下的 HTML，不递归子目录也不接受其他类型。
fn lists_root_html_candidates() {
    let fixture = Fixture::new();
    let candidates = entry_candidates(&fixture.0.join("site")).unwrap();
    assert_eq!(candidates, vec!["app.htm", "index.html"]);
}

#[test]
/// 目录复制保留相对结构，样式、脚本与子目录页面仍能按原路径解析。
fn copies_site_structure() {
    let fixture = Fixture::new();
    let source = fixture.0.join("site");
    let target = fixture.0.join("out/site");
    let (files, bytes) = measure(&source).unwrap();
    assert_eq!(files, 4);
    assert!(bytes > 0);
    copy_site(&source, &target).unwrap();
    assert!(target.join("index.html").is_file());
    assert!(target.join("子目录/deep.html").is_file());
    assert_eq!(
        std::fs::read_to_string(target.join("notes.txt")).unwrap(),
        "x"
    );
}

#[test]
/// 入口必须是扫描候选之一，候选之外的路径、非 HTML 与 ADS 一律拒绝。
fn rejects_entry_outside_candidates() {
    let fixture = Fixture::new();
    let site = fixture.0.join("site");
    let candidates = entry_candidates(&site).unwrap();
    assert!(verify_entry(&site, "index.html", &candidates).is_ok());
    for value in [
        "",
        "../outside.html",
        "子目录/deep.html",
        "notes.txt",
        "index.html:secret",
    ] {
        assert!(
            verify_entry(&site, value, &candidates).is_err(),
            "应拒绝 {value}"
        );
    }
}

#[test]
/// 单文件导入只复制入口本身，不复制同目录下的其他资源。
fn copies_single_entry_only() {
    let fixture = Fixture::new();
    let source = fixture.0.join("site/index.html");
    let target = fixture.0.join("out/single");
    let entry = copy_entry(&source, &target).unwrap();
    assert_eq!(entry, "index.html");
    assert!(target.join("index.html").is_file());
    assert!(!target.join("app.htm").exists());
}

#[cfg(windows)]
#[test]
/// 目录联接不会被复制，避免把链接目标内容拖进应用数据目录。
fn skips_windows_junction() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.0.join("outside")).unwrap();
    std::fs::write(fixture.0.join("outside/secret.txt"), "secret").unwrap();
    let source = fixture.0.join("site");
    let link = source.join("escape");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(fixture.0.join("outside"))
        .output()
        .unwrap();
    assert!(status.status.success(), "创建测试目录联接失败");
    let target = fixture.0.join("out/site");
    let result = copy_site(&source, &target);
    // 先删除联接本身，再由 Fixture 清理目录，绝不沿联接递归删除。
    std::fs::remove_dir(&link).unwrap();
    result.unwrap();
    assert!(!target.join("escape").exists());
}
