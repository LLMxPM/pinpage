// 功能：验证独立技能包往返、修改冲突和固定子目录边界。
use super::*;
/// 临时根目录只用于本测试，析构时删除自身内容。
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("pinpage-skills-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn export_install_roundtrip_and_conflicts() {
    let temp = Temp::new();
    let zip = temp.0.join("技能包.zip");
    export(&zip, false).unwrap();
    assert_eq!(load(Some(&zip)).unwrap(), builtin());
    let root = temp.0.join("中文 空格 skills");
    std::fs::create_dir_all(root.join("another")).unwrap();
    std::fs::write(root.join("another/keep"), "保留").unwrap();
    assert_eq!(
        install(Some(&root), Some(&zip), false, false).unwrap()["changed"],
        true
    );
    assert_eq!(
        install(Some(&root), None, false, false).unwrap()["changed"],
        false
    );
    std::fs::write(root.join(ID).join("SKILL.md"), "修改").unwrap();
    assert_eq!(
        install(Some(&root), None, true, false).unwrap_err().code,
        "skillModified"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("another/keep")).unwrap(),
        "保留"
    );
    assert_eq!(export(&zip, false).unwrap_err().code, "outputExists");
}
#[test]
fn rejects_unknown_target_and_unsafe_paths() {
    let temp = Temp::new();
    std::fs::create_dir_all(temp.0.join(ID)).unwrap();
    assert_eq!(
        install(Some(&temp.0), None, false, false).unwrap_err().code,
        "skillTargetConflict"
    );
    for p in [
        "../bad", "C:/bad", "x:y", "/abs", "a\\b", "a//b", "NUL.txt", "dir/COM1", "bad?",
    ] {
        assert!(files::safe_path(p).is_err(), "{p}");
    }
    let mut bundle = builtin();
    bundle.insert("other".into(), vec![]);
    assert!(validate(&bundle).is_err());
}

/// 导出目录与 ZIP 包含完全相同的文件；伪造最低能力和摘要都不能安装。
#[test]
fn directory_export_and_manifest_compatibility() {
    let temp = Temp::new();
    let exported = export(&temp.0, false).unwrap();
    let loaded = load(Some(&temp.0.join(ID))).unwrap();
    assert_eq!(loaded, builtin());
    let manifest = validate(&loaded).unwrap();
    assert_eq!(exported["skillVersion"], manifest.skill_version);
    let installed = Temp::new();
    install(Some(&installed.0), Some(&temp.0.join(ID)), false, false).unwrap();
    let status = status(Some(&installed.0)).unwrap();
    assert_eq!(status["builtinVersion"], exported["skillVersion"]);
    assert_eq!(status["installedVersion"], exported["skillVersion"]);
    let mut bundle = builtin();
    let mut manifest: Manifest = serde_json::from_slice(&bundle[MANIFEST]).unwrap();
    manifest.minimum_protocol = 99;
    bundle.insert(
        MANIFEST.into(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    assert_eq!(validate(&bundle).err().unwrap().code, "skillIncompatible");
}

/// 旧版技能升级后替换整套参考，清除废弃文件并保持新清单与实际文件一致。
#[test]
fn reorganized_bundle_replaces_previous_reference_layout() {
    let source = Temp::new();
    let root = Temp::new();
    let mut previous: Bundle = [
        ("SKILL.md", "---\nname: pinpage-pages\ndescription: 测试旧版技能\n---\n\n[页面](references/pages.md)\n[结构](references/structure.md)\n[命令](references/commands.md)\n[能力](references/host-api.md)\n"),
        ("references/pages.md", "# 旧版页面约束\n"),
        ("references/structure.md", "# 旧版结构\n"),
        ("references/commands.md", "# 旧版命令\n"),
        ("references/host-api.md", "# 旧版能力\n"),
    ]
    .into_iter()
    .map(|(name, text)| (name.into(), text.as_bytes().to_vec()))
    .collect();
    let mut manifest = validate(&builtin()).unwrap();
    manifest.skill_version = "1.0.3".into();
    manifest.files = previous
        .iter()
        .map(|(p, b)| (p.clone(), digest(b)))
        .collect();
    previous.insert(
        MANIFEST.into(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    for (name, bytes) in &previous {
        let path = source.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    install(Some(&root.0), Some(&source.0), false, false).unwrap();
    let result = install(Some(&root.0), None, true, false).unwrap();
    assert_eq!(result["previousVersion"], "1.0.3");
    assert_eq!(result["sourceVersion"], VERSION);
    assert_eq!(result["changed"], true);
    assert_eq!(files::read_tree(&root.0.join(ID)).unwrap(), builtin());
    for obsolete in ["pages.md", "structure.md", "commands.md"] {
        assert!(!root.0.join(ID).join("references").join(obsolete).exists());
    }
}

/// ZIP 越界在解包前被拒绝，不在用户目标写入任何内容。
#[test]
fn zip_traversal_is_rejected() {
    use std::io::Write;
    let temp = Temp::new();
    let path = temp.0.join("bad.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    zip.start_file(
        "pinpage-pages/../outside",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"bad").unwrap();
    zip.finish().unwrap();
    assert!(load(Some(&path)).is_err());
    assert!(!temp.0.join("outside").exists());
}
