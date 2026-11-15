// 功能：验证预设命令的必需身份约束、布局导入开关和更新同步策略。
use super::*;
use clap::Parser;

/// 从真实参数解析入口构造 CLI，避免测试绕过 Clap 的约束和默认值。
fn cli(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("pinpage-cli").chain(args.iter().copied())).unwrap()
}

#[test]
/// 列表允许只读查询，写入缺失实例或修订时不能发送请求。
fn preset_mutations_require_identity_and_revision() {
    let (method, _, _) = remote(&cli(&["window", "presets", "list", "p-one"])).unwrap();
    assert_eq!(method, "window.presets.list");
    assert!(Cli::try_parse_from([
        "pinpage-cli",
        "window",
        "presets",
        "apply",
        "p-one",
        "compact"
    ])
    .is_err());
    let missing = cli(&[
        "window",
        "presets",
        "apply",
        "p-one",
        "compact",
        "--expected-revision",
        "3",
    ]);
    assert_eq!(remote(&missing).unwrap_err().code, "invalidArgument");
    let valid = cli(&[
        "--expected-instance",
        "instance",
        "window",
        "presets",
        "apply",
        "p-one",
        "compact",
        "--expected-revision",
        "3",
    ]);
    let (method, params, _) = remote(&valid).unwrap();
    assert_eq!(method, "window.presets.apply");
    assert_eq!(params["expectedRevision"], 3);
    assert_eq!(params["presetId"], "compact");
}

#[test]
/// 取消应用布局不取消导入命名列表，更新默认保留列表且可显式替换。
fn install_and_update_have_independent_preset_policies() {
    let (_, params, _) = remote(&cli(&[
        "pages",
        "install",
        "--source",
        "tool.pinpage",
        "--apply-package-preset",
        "false",
    ]))
    .unwrap();
    assert_eq!(params["applyPackagePreset"], false);
    assert_eq!(params["importPackagePresets"], true);
    for (extra, expected) in [(vec![], "keep"), (vec!["--presets", "replace"], "replace")] {
        let mut args = vec![
            "--expected-instance",
            "instance",
            "pages",
            "update",
            "p-one",
            "--source",
            "tool.pinpage",
            "--expected-revision",
            "1",
        ];
        args.extend(extra);
        let (_, params, _) = remote(&cli(&args)).unwrap();
        assert_eq!(params["presets"], expected);
    }
}

#[test]
/// 页面包导出默认排除个人数据；显式开启数据与覆盖才能发送对应请求。
fn export_defaults_to_no_personal_storage_or_overwrite() {
    for (extra, expected) in [
        (vec![], false),
        (vec!["--with-data", "true", "--overwrite"], true),
    ] {
        let mut args = vec!["pages", "export", "p-one", "--out", "tool.pinpage"];
        args.extend(extra);
        let (method, params, _) = remote(&cli(&args)).unwrap();
        assert_eq!(method, "pages.export");
        assert_eq!(params["withData"], expected);
        assert_eq!(params["overwrite"], expected);
    }
}
