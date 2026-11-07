// 功能：验证采集脚本与引导页的生成结果，避免模板占位符或关键逻辑丢失。
use super::*;

#[test]
/// 采集脚本注入页面 ID 与记录上限，不再残留占位符。
fn injects_identity_and_limit() {
    let script = collect_script("p-test");
    assert!(script.contains(r#"const id = "p-test";"#));
    assert!(script.contains("const MAX_RECORDS = 3000;"));
    assert!(!script.contains("__ID__"));
    assert!(!script.contains("__MAX_RECORDS__"));
}

#[test]
/// 采集覆盖三种存储，并通过一次性的回传命令上报。
fn collects_browser_storages() {
    let script = collect_script("p-test");
    for keyword in [
        "localStorage",
        "sessionStorage",
        "indexedDB.databases",
        "openCursor",
        "submit_page_data",
    ] {
        assert!(script.contains(keyword), "脚本应包含 {keyword}");
    }
}

#[test]
/// 引导页先取随包数据再跳转，即使取数失败也会落到入口。
fn restores_then_redirects() {
    let page = restore_page(crate::models::AppLocale::ZhCn);
    assert!(page.contains("/__pinpage__/data.json"));
    assert!(page.contains("location.replace(target)"));
    assert!(page.contains("indexedDB.open"));
    assert!(page.contains("onupgradeneeded"));
}

#[test]
/// 引导页的语言属性和提示文字随传入语言变化。
fn renders_requested_locale() {
    let english = restore_page(crate::models::AppLocale::EnUs);
    assert!(english.contains(r#"<html lang="en-US">"#));
    assert!(english.contains("Restoring page data"));
    let chinese = restore_page(crate::models::AppLocale::ZhCn);
    assert!(chinese.contains(r#"<html lang="zh-CN">"#));
    assert!(chinese.contains("正在恢复页面数据"));
}
