// 功能：验证内置包可复现、标准导入往返、完整资源与白名单标识边界。
use super::*;
use std::path::PathBuf;

#[path = "../build/examples.rs"]
mod builder;

struct Fixture(PathBuf);

impl Fixture {
    /// 测试只在随机临时目录中生成包和站点，不触及正在运行的用户实例。
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pinpage-examples-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    /// 只移除本测试创建的直属临时目录，避免越界递归清理。
    fn drop(&mut self) {
        let (Ok(root), Ok(target)) = (std::env::temp_dir().canonicalize(), self.0.canonicalize())
        else {
            return;
        };
        if target.parent() == Some(root.as_path())
            && target
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("pinpage-examples-")
        {
            let _ = std::fs::remove_dir_all(target);
        }
    }
}

#[test]
/// 未登记的标识不进入文件系统或包索引查询。
fn rejects_unknown_examples() {
    for value in ["../notes", "unknown", "C:\\temp\\clock"] {
        assert!(serde_json::from_value::<ExampleId>(serde_json::json!(value)).is_err());
    }
}

#[test]
/// 构建与分发打包器生成相同字节，内置包和磁盘包共享解析结果，子目录资源不会遗漏。
fn examples_are_reproducible_standard_packages() {
    let fixture = Fixture::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let generated = builder::pack_all(&source, &fixture.0.join("packages")).unwrap();
    assert_eq!(generated.len(), PACKAGES.len());
    for example in [
        ExampleId::Timer,
        ExampleId::Notes,
        ExampleId::Folder,
        ExampleId::SystemMonitor,
        ExampleId::Stocks,
        ExampleId::Calendar,
        ExampleId::Kanban,
    ] {
        let bytes = package_bytes(example);
        let (manifest, size) = preview(example).unwrap();
        assert_eq!(size, bytes.len() as u64);
        assert!(manifest.window_preset.is_some());
        assert!(!manifest.window_presets.is_empty());
        let (id, _) = PACKAGES
            .iter()
            .find(|(_, candidate)| *candidate == bytes)
            .unwrap();
        let path = &generated.iter().find(|(name, _)| name == id).unwrap().1;
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        let disk_manifest = package::read_manifest(path).unwrap();
        assert_eq!(disk_manifest.window_preset, manifest.window_preset);
        let target = fixture.0.join(id);
        let extracted = package::extract(path, &target).unwrap();
        assert_eq!(extracted.entry, manifest.entry);
        assert!(extracted.storage.is_none());
        assert_eq!(crate::import::measure(&target).unwrap().0, manifest.files);
        let html = std::fs::read_to_string(target.join(&manifest.entry)).unwrap();
        for attribute in ["href=\"./", "src=\"./"] {
            for suffix in html.split(attribute).skip(1) {
                let resource = suffix.split('"').next().unwrap();
                assert!(target.join(resource).is_file(), "{id} 缺少资源 {resource}");
            }
        }
        let second = fixture.0.join(format!("{id}-copy"));
        let memory = extract(&second, example).unwrap();
        assert_eq!(memory.window_preset, extracted.window_preset);
        assert_eq!(
            std::fs::read(second.join(&manifest.entry)).unwrap(),
            html.as_bytes()
        );
    }
    assert!(fixture.0.join("timer/presets.json").is_file());
}

#[test]
/// 合并后旧白名单标识仍能创建新工具，不需要重复维护旧页面包。
fn legacy_ids_resolve_to_merged_tools() {
    assert_eq!(
        package_bytes(ExampleId::Clock),
        package_bytes(ExampleId::Timer)
    );
    assert_eq!(
        package_bytes(ExampleId::Checklist),
        package_bytes(ExampleId::Notes)
    );
    assert_eq!(label_key(ExampleId::Clock), label_key(ExampleId::Timer));
    assert_eq!(label_key(ExampleId::Checklist), label_key(ExampleId::Notes));
}
