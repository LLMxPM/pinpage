// 功能：嵌入 native 资源，为 Rust 原生界面提供与前端一致的翻译与回退。
use crate::{
    errors::AppMessage,
    models::{AppLocale, LocalePreference},
    platform,
};
use serde_json::Value;
use std::{collections::BTreeMap, sync::LazyLock};

const ZH_TEXTS: &str = include_str!("../../locales/zh-CN/native.json");
const EN_TEXTS: &str = include_str!("../../locales/en-US/native.json");

/// 编译期把两份 native 资源展开为点分键表，避免运行时再解析 JSON。
static CATALOGS: LazyLock<[BTreeMap<String, String>; 2]> = LazyLock::new(|| {
    [
        flatten(ZH_TEXTS).unwrap_or_default(),
        flatten(EN_TEXTS).unwrap_or_default(),
    ]
});

/// 把嵌套 JSON 展平为 `a.b.c -> 文案`，非字符串值一律忽略。
fn flatten(text: &str) -> Result<BTreeMap<String, String>, String> {
    let root: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let mut out = BTreeMap::new();
    walk(&root, "", &mut out);
    fn walk(value: &Value, prefix: &str, out: &mut BTreeMap<String, String>) {
        match value {
            Value::Object(fields) => {
                for (key, child) in fields {
                    walk(child, &format!("{prefix}{key}."), out);
                }
            }
            Value::String(text) => {
                out.insert(prefix.trim_end_matches('.').into(), text.clone());
            }
            _ => {}
        }
    }
    walk(&root, "", &mut out);
    Ok(out)
}

/// 按键取当前语言文案，缺键回退中文，仍找不到时原样返回键名。
pub fn text(locale: AppLocale, key: &str, args: &BTreeMap<String, String>) -> String {
    let catalog = &CATALOGS[if locale == AppLocale::EnUs { 1 } else { 0 }];
    let fallback = &CATALOGS[0];
    let raw = catalog
        .get(key)
        .or_else(|| fallback.get(key))
        .map(String::as_str)
        .unwrap_or(key);
    substitute(raw, args)
}

/// 渲染一条结构化消息，供托盘 tooltip 等 Rust 侧展示。
pub fn render(locale: AppLocale, message: &AppMessage) -> String {
    text(locale, &message.code, &message.args)
}

/// 把一条消息作为原因写进另一条消息的详情。
///
/// 外层消息码保持不变，因此切换语言后主体说明仍会重新翻译；
/// 只有原因部分停留在记录当时的语言，用于回滚和托盘刷新等嵌套场景。
pub fn caused_by(message: AppMessage, locale: AppLocale, cause: &AppMessage) -> AppMessage {
    message.detail(render(locale, cause))
}

/// 供日志使用：按中文渲染，保证诊断输出稳定可 grep。
pub fn log_text(message: &AppMessage) -> String {
    render(AppLocale::ZhCn, message)
}

/// 只做 `{name}` 文本替换；native 资源不含复数与其他 Vue 语法。
fn substitute(raw: &str, args: &BTreeMap<String, String>) -> String {
    if !raw.contains('{') {
        return raw.to_string();
    }
    let mut result = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        match rest.find('}') {
            Some(end) => {
                let name = &rest[..end];
                result.push_str(args.get(name).map(String::as_str).unwrap_or(""));
                rest = &rest[end + 1..];
            }
            None => {
                result.push('{');
                break;
            }
        }
    }
    result.push_str(rest);
    result
}

/// 解析用户偏好为有效语言；`system` 依赖平台读取，失败时回退中文。
pub fn resolve(preference: LocalePreference) -> AppLocale {
    match preference {
        LocalePreference::ZhCn => AppLocale::ZhCn,
        LocalePreference::EnUs => AppLocale::EnUs,
        LocalePreference::System => preferred().unwrap_or(AppLocale::ZhCn),
    }
}

/// 按 Windows 首选界面语言顺序挑选：先匹配到的语言生效，无匹配返回 None。
pub fn preferred() -> Option<AppLocale> {
    from_language_list(&platform::locale::user_preferred_languages())
}

/// 语言列表解析：`en*` 使用英文资源，`zh*` 使用中文资源，其余跳过。
pub fn from_language_list(languages: &[String]) -> Option<AppLocale> {
    languages.iter().find_map(|tag| {
        let primary = tag.split('-').next().unwrap_or("").to_lowercase();
        match primary.as_str() {
            "en" => Some(AppLocale::EnUs),
            "zh" => Some(AppLocale::ZhCn),
            _ => None,
        }
    })
}

/// 资源自检：两份目录键集合一致，且 native 资源不含被禁止的语法。
#[cfg(test)]
#[path = "i18n_tests.rs"]
mod tests;
