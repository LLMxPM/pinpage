// 功能：验证文件夹能力的授权门槛、列举排序、边界拒绝与打开定位。
use super::*;
use crate::models::{PageSource, WindowMode};

struct Fixture(PathBuf);

impl Fixture {
    /// 创建独立临时目录，避免测试读取用户文件或相互污染。
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pinpage-folder-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(path.join("root/子目录")).unwrap();
        std::fs::write(path.join("root/a.txt"), "hello").unwrap();
        std::fs::write(path.join("root/子目录/b.json"), "{}").unwrap();
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
            .is_some_and(|name| name.starts_with("pinpage-folder-"));
        if target.parent() == Some(temporary_root.as_path()) && safe_name {
            let _ = std::fs::remove_dir_all(target);
        }
    }
}

/// 构造仅关心授权根目录的页面配置。
fn pin(root: Option<String>) -> PinConfig {
    PinConfig {
        id: "p-test".into(),
        name: "test".into(),
        source: PageSource::Bundled {
            entry: "index.html".into(),
        },
        width: 400.,
        height: 300.,
        x: None,
        y: None,
        scale: 1.,
        always_on_top: false,
        decorations: false,
        enabled: true,
        mode: WindowMode::Floating,
        rounded: false,
        transparent_background: false,
        position_locked: false,
        size_locked: false,
        page_context_menu: false,
        preset: None,
        folder_root: root,
    }
}

#[test]
/// 未授权页面调用列举直接拒绝，不触碰文件系统。
fn list_needs_grant() {
    let error = list(&pin(None), "").unwrap_err();
    assert_eq!(error.code, "errors.folderNoGrant");
}

#[test]
/// 根目录与子目录均可列举，条目按名称排序并区分类型。
fn lists_and_navigates() {
    let fixture = Fixture::new();
    let root = fixture.0.join("root");
    let config = pin(Some(root.to_string_lossy().into_owned()));
    let listing = list(&config, "").unwrap();
    assert_eq!(listing.root_label, "root");
    let names: Vec<&str> = listing.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["a.txt", "子目录"]);
    assert_eq!(listing.entries[0].kind, "file");
    assert_eq!(listing.entries[0].size_bytes, 5);
    assert_eq!(listing.entries[1].kind, "dir");
    assert!(!listing.truncated);
    let sub = list(&config, "子目录").unwrap();
    assert_eq!(sub.entries.len(), 1);
    assert_eq!(sub.entries[0].name, "b.json");
}

#[test]
/// 越界路径与缺失目标分别报告对应消息码。
fn rejects_escape_and_missing() {
    let fixture = Fixture::new();
    let root = fixture.0.join("root");
    let config = pin(Some(root.to_string_lossy().into_owned()));
    assert_eq!(
        list(&config, "../outside.txt").unwrap_err().code,
        "errors.folderOutside"
    );
    assert_eq!(
        list(&config, "C:/outside.txt").unwrap_err().code,
        "errors.folderOutside"
    );
    assert_eq!(
        list(&config, "missing").unwrap_err().code,
        "errors.folderNotFound"
    );
}

#[test]
/// 授权根目录失效时引导重选，而不是当作未授权。
fn root_missing_reported() {
    let fixture = Fixture::new();
    let config = pin(Some(fixture.0.join("gone").to_string_lossy().into_owned()));
    assert_eq!(
        list(&config, "").unwrap_err().code,
        "errors.folderRootMissing"
    );
}

#[test]
/// 打开只接受文件，定位返回所在目录；越界同样拒绝。
fn open_and_reveal_targets() {
    let fixture = Fixture::new();
    let root = fixture.0.join("root");
    let config = pin(Some(root.to_string_lossy().into_owned()));
    assert!(open_target(&config, "a.txt").unwrap().is_file());
    assert_eq!(
        open_target(&config, "子目录").unwrap_err().code,
        "errors.folderNotFound"
    );
    assert_eq!(
        open_target(&config, "../outside.txt").unwrap_err().code,
        "errors.folderOutside"
    );
    assert_eq!(
        reveal_target(&config, "a.txt").unwrap(),
        root.canonicalize().unwrap()
    );
    assert!(reveal_target(&config, "子目录").unwrap().is_dir());
}
