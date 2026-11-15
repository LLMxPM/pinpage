// 功能：验证共享路径边界模块，覆盖目录、字面百分号与 Windows 目录联接。
use super::*;

struct Fixture(PathBuf);

impl Fixture {
    /// 创建独立临时目录，避免测试读取用户文件或相互污染。
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pinpage-guard-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(path.join("root/子目录")).unwrap();
        std::fs::write(path.join("root/index.html"), "<html></html>").unwrap();
        std::fs::write(path.join("root/100%.txt"), "literal percent").unwrap();
        std::fs::write(path.join("outside.txt"), "not authorized").unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    /// 清理本测试创建的随机目录，失败不掩盖断言结果。
    fn drop(&mut self) {
        let (Ok(temporary_root), Ok(target)) =
            (std::env::temp_dir().canonicalize(), self.0.canonicalize())
        else {
            return;
        };
        let safe_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.starts_with("pinpage-guard-"));
        if target.parent() == Some(temporary_root.as_path()) && safe_name {
            let _ = std::fs::remove_dir_all(target);
        }
    }
}

#[test]
/// 文件、目录与空相对路径均可解析，字面百分号不被解码。
fn resolves_files_and_dirs() {
    let fixture = Fixture::new();
    let root = fixture.0.join("root");
    assert!(resolve(&root, "/index.html").unwrap().is_file());
    assert!(resolve(&root, "子目录").unwrap().is_dir());
    assert!(resolve(&root, "").unwrap().is_dir());
    assert!(resolve(&root, "/100%.txt").unwrap().is_file());
}

#[test]
/// 目录穿越、非法字符与缺失目标被拒绝，根目录失效单独报告。
fn rejects_escape_and_missing() {
    let fixture = Fixture::new();
    let root = fixture.0.join("root");
    for path in [
        "../outside.txt",
        "/../outside.txt",
        "子目录/../../outside.txt",
        "C:/outside.txt",
        "index.html:secret",
        "子目录\\..\\outside.txt",
        "\0",
        "missing.html",
    ] {
        assert!(resolve(&root, path).is_err(), "应拒绝 {path}");
    }
    assert!(matches!(
        resolve(&fixture.0.join("missing-root"), "x"),
        Err(GuardError::RootMissing)
    ));
    assert!(matches!(
        resolve(&root, "missing"),
        Err(GuardError::NotFound)
    ));
}

#[cfg(windows)]
#[test]
/// 目录联接指向根目录外时，即使文本路径位于根目录内也必须拒绝。
fn rejects_windows_junction_escape() {
    let fixture = Fixture::new();
    let root = fixture.0.join("root");
    let link = root.join("escape");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&fixture.0)
        .output()
        .unwrap();
    assert!(status.status.success(), "创建测试目录联接失败");
    let result = resolve(&root, "escape/outside.txt");
    // 删除联接本身，再由 Fixture 清理目录，绝不沿联接递归删除。
    std::fs::remove_dir(&link).unwrap();
    assert!(matches!(result, Err(GuardError::Outside)));
}
