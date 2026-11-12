// 功能：验证本机导入来源的候选、大小预算、单文件范围和内容摘要。
use super::*;
struct Fixture(PathBuf);
impl Fixture {
    /// 每个用例只创建独立临时目录，不接触应用或用户配置。
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("pinpage-local-source-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("source")).unwrap();
        std::fs::write(root.join("source/index.html"), "<html>中文</html>").unwrap();
        std::fs::write(
            root.join("source/app.js"),
            "// 功能：测试资源\nconsole.log('v1');",
        )
        .unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn directory_is_complete_and_content_digest_changes() {
    let f = Fixture::new();
    let a = prepare(&f.0.join("source"), None, &f.0.join("a")).unwrap();
    assert!(f.0.join("a/app.js").is_file());
    std::fs::write(
        f.0.join("source/app.js"),
        "// 功能：第二版\nconsole.log('v2');",
    )
    .unwrap();
    let b = prepare(&f.0.join("source"), Some("index.html"), &f.0.join("b")).unwrap();
    assert_ne!(a.digest, b.digest);
}
#[test]
fn candidates_and_single_file_scope_are_strict() {
    let f = Fixture::new();
    std::fs::write(f.0.join("source/other.htm"), "other").unwrap();
    let e = prepare(&f.0.join("source"), None, &f.0.join("a"))
        .err()
        .unwrap();
    assert_eq!(e.code, "invalidSource");
    assert_eq!(
        e.details.unwrap()["candidates"].as_array().unwrap().len(),
        2
    );
    assert!(prepare(&f.0.join("source"), Some("../index.html"), &f.0.join("b")).is_err());
    prepare(&f.0.join("source/index.html"), None, &f.0.join("single")).unwrap();
    assert!(!f.0.join("single/app.js").exists());
}
#[test]
fn rejects_relative_paths_and_oversized_single_file() {
    let f = Fixture::new();
    assert!(prepare(Path::new("relative"), None, &f.0.join("a")).is_err());
    let file = std::fs::File::create(f.0.join("large.html")).unwrap();
    file.set_len(256 * 1024 * 1024 + 1).unwrap();
    drop(file);
    assert!(prepare(&f.0.join("large.html"), None, &f.0.join("b")).is_err());
}
