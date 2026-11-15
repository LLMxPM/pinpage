// 功能：验证导出发布的覆盖约束、失败保留原文件与临时文件清理边界。
use super::*;

/// 在操作系统临时目录创建独立文件夹，清理时核对随机目录仍直属临时根。
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pinpage-export-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let root = std::env::temp_dir().canonicalize().unwrap();
        let path = self.0.canonicalize().unwrap();
        if path.parent() == Some(root.as_path())
            && path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("pinpage-export-")
        {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

#[test]
/// 默认发布不覆盖已经存在的目标，明确覆盖时才替换成完整的新文件。
fn publishing_preserves_existing_output_until_explicit_overwrite() {
    let fixture = Fixture::new();
    let stage = fixture.0.join("stage.tmp");
    let target = fixture.0.join("工具.pinpage");
    std::fs::write(&stage, "新包").unwrap();
    publish(&stage, &target, false).unwrap();
    assert!(!stage.exists());
    std::fs::write(&stage, "下一版").unwrap();
    assert_eq!(
        publish(&stage, &target, false).unwrap_err().code,
        "errors.exportExists"
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "新包");
    std::fs::remove_file(&stage).unwrap();
    std::fs::write(&stage, "替换包").unwrap();
    publish(&stage, &target, true).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "替换包");
    assert!(!stage.exists());
}

#[test]
/// 输出被目录占用或暂存包缺失时失败，既有内容保持原样。
fn failed_publish_does_not_destroy_original() {
    let fixture = Fixture::new();
    let target = fixture.0.join("工具.pinpage");
    std::fs::write(&target, "原包").unwrap();
    assert!(publish(&fixture.0.join("missing"), &target, true).is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "原包");
    let directory = fixture.0.join("目录.pinpage");
    std::fs::create_dir(&directory).unwrap();
    assert!(check_target(&directory, true).is_err());
}
