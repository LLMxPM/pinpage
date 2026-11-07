// 功能：验证网址协议与本地资源目录边界，覆盖真实目录和 Windows 目录联接。
use super::*;

struct Fixture(PathBuf);

impl Fixture {
    /// 创建独立临时目录，避免测试读取用户文件或相互污染。
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pinpage-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(path.join("root/子目录")).unwrap();
        std::fs::write(path.join("root/index.html"), "<html></html>").unwrap();
        std::fs::write(path.join("root/子目录/测试 文件.json"), "{}").unwrap();
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
            .is_some_and(|name| name.starts_with("pinpage-test-"));
        if target.parent() == Some(temporary_root.as_path()) && safe_name {
            let _ = std::fs::remove_dir_all(target);
        }
    }
}

#[test]
/// 网络地址补全、回环服务和不安全协议具有明确的处理结果。
fn validates_network_urls() {
    assert_eq!(
        normalize_url("example.com").unwrap(),
        "https://example.com/"
    );
    assert_eq!(
        normalize_url("localhost:3000/tool").unwrap(),
        "http://localhost:3000/tool"
    );
    assert_eq!(
        normalize_url("127.0.0.1:8080").unwrap(),
        "http://127.0.0.1:8080/"
    );
    for value in [
        "",
        "not a url",
        "file:///C:/secret.txt",
        "ftp://example.com",
        "javascript:alert(1)",
        "https://user:pass@example.com",
    ] {
        assert!(normalize_url(value).is_err(), "应拒绝 {value}");
    }
}

#[test]
/// 正常资源与中文编码路径可读取，目录穿越及 Windows ADS 被拒绝。
fn bounds_local_resources() {
    let fixture = Fixture::new();
    let root = fixture.0.join("root");
    assert!(resolve_path(&root, "/index.html")
        .unwrap()
        .ends_with("index.html"));
    assert!(resolve_path(
        &root,
        "/%E5%AD%90%E7%9B%AE%E5%BD%95/%E6%B5%8B%E8%AF%95%20%E6%96%87%E4%BB%B6.json"
    )
    .is_ok());
    for path in [
        "/../outside.txt",
        "/%2e%2e/outside.txt",
        "/%2e%2e%5coutside.txt",
        "/C:/outside.txt",
        "/index.html:secret",
        "/%00",
        "/missing.html",
    ] {
        assert!(resolve_path(&root, path).is_err(), "应拒绝 {path}");
    }
}

#[test]
/// 导入入口生成以页面 ID 为主机的地址，兄弟窗口不能互相访问。
fn validates_bundled_origins() {
    let source = PageSource::Bundled {
        entry: "index.html".into(),
    };
    let url = source_url("p-one", &source).unwrap();
    assert_eq!(url.as_str(), "pinpage://p-one/index.html");
    assert!(navigation_allowed("p-one", &source, &url));
    assert!(navigation_allowed(
        "p-one",
        &source,
        &Url::parse("http://pinpage.p-one/index.html").unwrap()
    ));
    assert!(!navigation_allowed("p-two", &source, &url));
    assert!(!navigation_allowed(
        "p-one",
        &source,
        &Url::parse("https://example.com").unwrap()
    ));
    // 入口必须是纯相对路径，越界路径在生成地址前就被拒绝。
    // `C:/x.html` 只在 Windows 上是绝对路径，Unix 上属于合法相对路径。
    let mut invalid_entries = vec!["", "../outside.html", "/abs.html"];
    if cfg!(windows) {
        invalid_entries.push("C:/x.html");
    }
    for entry in invalid_entries {
        let invalid = PageSource::Bundled {
            entry: entry.into(),
        };
        assert!(source_url("p-one", &invalid).is_err(), "应拒绝入口 {entry}");
    }
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
    let result = resolve_path(&root, "/escape/outside.txt");
    // 删除联接本身，再由 Fixture 清理目录，绝不沿联接递归删除。
    std::fs::remove_dir(&link).unwrap();
    assert!(result.is_err());
}
