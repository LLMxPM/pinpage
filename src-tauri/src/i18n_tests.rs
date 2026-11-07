// 功能：验证 native 资源的加载、语言回退、参数替换与系统语言解析。
use super::*;
use crate::errors::AppMessage;
use crate::models::LocalePreference;
use std::collections::BTreeMap;

#[test]
/// 两份资源都能按点分键取到文案。
fn resolves_keys_in_both_locales() {
    assert_eq!(
        text(AppLocale::ZhCn, "tray.openManager", &Default::default()),
        "打开管理界面"
    );
    assert_eq!(
        text(AppLocale::EnUs, "tray.openManager", &Default::default()),
        "Open the manager"
    );
}

#[test]
/// 未知键回退中文；两份都没有时原样返回键名，便于定位缺键。
fn falls_back_to_chinese_then_key() {
    let texts = BTreeMap::from([("path".to_string(), "C:\\tmp".to_string())]);
    assert_eq!(
        text(AppLocale::EnUs, "errors.configBackedUp", &texts),
        "The configuration could not be read and was backed up to C:\\tmp"
    );
    assert_eq!(
        text(AppLocale::EnUs, "errors.doesNotExist", &Default::default()),
        "errors.doesNotExist"
    );
}

#[test]
/// 参数只按名字替换，缺失参数留空，用户数据不会被再次解析为模板。
fn substitutes_named_arguments_only() {
    let message = AppMessage::new("errors.restoreFailed")
        .arg("name", "订单 {id} 看板")
        .arg("reason", "已阻止不支持的导航协议或本地来源");
    let rendered = render(AppLocale::ZhCn, &message);
    assert!(rendered.contains("订单 {id} 看板"), "实际结果：{rendered}");
    assert!(rendered.contains("已阻止不支持的导航协议或本地来源"));
    let missing = text(AppLocale::ZhCn, "errors.tooManyFiles", &Default::default());
    assert!(missing.contains("导入文件超过"), "实际结果：{missing}");
}

#[test]
/// 显式偏好直接决定语言，system 在无匹配语言时回退中文。
fn preference_resolution_follows_rules() {
    assert_eq!(
        resolve(LocalePreference::EnUs),
        AppLocale::EnUs,
        "显式英文不受系统语言影响"
    );
    assert_eq!(from_language_list(&[]), None);
    assert_eq!(
        from_language_list(&["de-DE".into(), "en-GB".into()]),
        Some(AppLocale::EnUs)
    );
    assert_eq!(
        from_language_list(&["zh-TW".into()]),
        Some(AppLocale::ZhCn),
        "繁体标签使用简体资源"
    );
    assert_eq!(
        from_language_list(&["de-DE".into()]),
        None,
        "无匹配语言时由调用方回退中文"
    );
    assert_eq!(
        from_language_list(&["en-US".into(), "zh-CN".into()]),
        Some(AppLocale::EnUs),
        "按列表顺序取第一个可匹配语言"
    );
}
