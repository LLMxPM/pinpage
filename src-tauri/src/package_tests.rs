// 功能：验证页面包的往返打包、数据快照与解包路径边界。
use super::*;
use std::path::PathBuf;

struct Fixture(PathBuf);

impl Fixture {
    /// 创建含入口与子目录资源的独立临时站点。
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pinpage-package-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(path.join("site/子目录")).unwrap();
        std::fs::write(path.join("site/index.html"), "<html></html>").unwrap();
        std::fs::write(path.join("site/子目录/app.js"), "console.log(1)").unwrap();
        std::fs::create_dir_all(path.join("out")).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    /// 只清理本测试创建的随机目录，避免误删用户文件。
    fn drop(&mut self) {
        let (Ok(temporary_root), Ok(target)) =
            (std::env::temp_dir().canonicalize(), self.0.canonicalize())
        else {
            return;
        };
        let safe_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.starts_with("pinpage-package-"));
        if target.parent() == Some(temporary_root.as_path()) && safe_name {
            let _ = std::fs::remove_dir_all(target);
        }
    }
}

/// 直接写一个 ZIP，用于构造正常导入流程不会产生的异常包。
fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    for (name, data) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
/// 站点副本与数据快照可以完整往返，清单记录的名称与入口保持一致。
fn round_trips_site_and_storage() {
    let fixture = Fixture::new();
    let site = fixture.0.join("site");
    let package = fixture.0.join("out/示例.pinpage");
    let storage = r#"{"localStorage":{"timer":"300"},"sessionStorage":{}}"#;
    let files = write_dir(&site, "示例", "index.html", Some(storage), &package).unwrap();
    assert_eq!(files, 2);

    let manifest = read_manifest(&package).unwrap();
    assert_eq!(manifest.name, "示例");
    assert_eq!(manifest.entry, "index.html");
    assert_eq!(manifest.schema_version, PACKAGE_VERSION);

    let target = fixture.0.join("out/restored");
    let extracted = extract(&package, &target).unwrap();
    assert_eq!(extracted.entry, "index.html");
    assert_eq!(extracted.storage.as_deref(), Some(storage));
    assert!(target.join("index.html").is_file());
    assert!(target.join("子目录/app.js").is_file());
}

#[test]
/// 包内出现上级目录条目时解包必须失败，且不会在目标目录之外落盘。
fn rejects_parent_path_entries() {
    let fixture = Fixture::new();
    let package = fixture.0.join("out/escape.pinpage");
    write_zip(
        &package,
        &[
            (
                "manifest.json",
                br#"{"schemaVersion":1,"name":"escape","entry":"index.html","exportedAt":0,"files":1}"#
                    .as_slice(),
            ),
            ("site/index.html", b"<html></html>".as_slice()),
            ("../escaped.html", b"<html></html>".as_slice()),
        ],
    );
    let target = fixture.0.join("out/escape");
    assert!(extract(&package, &target).is_err());
    assert!(!fixture.0.join("out/escaped.html").exists());
}

#[test]
/// 清单入口不能是越界路径或非 HTML，避免被构造成任意文件。
fn rejects_invalid_manifest_entry() {
    let fixture = Fixture::new();
    for (index, entry) in ["../evil.html", "/abs.html", "index.txt", ""]
        .iter()
        .enumerate()
    {
        let package = fixture.0.join(format!("out/bad-{index}.pinpage"));
        let manifest = format!(
            r#"{{"schemaVersion":1,"name":"bad","entry":"{entry}","exportedAt":0,"files":1}}"#
        );
        write_zip(
            &package,
            &[
                ("manifest.json", manifest.as_bytes()),
                ("site/index.html", b"<html></html>"),
            ],
        );
        assert!(read_manifest(&package).is_err(), "应拒绝入口 {entry}");
    }
}

#[test]
/// 包内缺少清单声明的入口文件时不得创建站点副本。
fn rejects_missing_entry_file() {
    let fixture = Fixture::new();
    let package = fixture.0.join("out/missing.pinpage");
    write_zip(
        &package,
        &[
            (
                "manifest.json",
                br#"{"schemaVersion":1,"name":"missing","entry":"index.html","exportedAt":0,"files":1}"#
                    .as_slice(),
            ),
            ("site/other.html", b"<html></html>".as_slice()),
        ],
    );
    assert!(extract(&package, &fixture.0.join("out/missing")).is_err());
}

#[test]
/// 数据快照必须是合法 JSON，否则整包被拒绝而不是带病导入。
fn rejects_invalid_storage_json() {
    let fixture = Fixture::new();
    let package = fixture.0.join("out/bad-data.pinpage");
    write_dir(
        &fixture.0.join("site"),
        "bad",
        "index.html",
        Some("not json"),
        &package,
    )
    .unwrap();
    assert!(extract(&package, &fixture.0.join("out/bad")).is_err());
}
