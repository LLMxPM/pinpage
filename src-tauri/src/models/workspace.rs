// 功能：定义全局设置、窗口手势修饰键及严格输入与容错配置的转换契约。
use super::{LocalePreference, PinConfig};
use crate::errors::AppMessage;
use serde::{Deserialize, Serialize};

/// 左右鼠标手势共用的修饰键；Alt 仅接受左 Alt，Ctrl 与 Shift 接受左右两侧键。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GestureModifier {
    #[default]
    Alt,
    Ctrl,
    Shift,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    /// 语言偏好未知或类型不符时只回退该字段，不影响页面与快捷键。
    #[serde(deserialize_with = "lenient_locale")]
    pub locale: LocalePreference,
    /// 磁盘上的坏字段只回退 Alt，不能让已有页面丢失。
    #[serde(deserialize_with = "lenient_gesture_modifier")]
    pub gesture_modifier: GestureModifier,
    pub restore_session: bool,
    pub manager_shortcut: String,
    pub recover_shortcut: String,
    /// 单条无法识别的记录只被跳过，不会让整份配置回退为默认值。
    #[serde(deserialize_with = "lenient_pins")]
    pub pins: Vec<PinConfig>,
}

/// 逐条解析页面记录，跳过来源类型已不再支持的旧条目。
fn lenient_pins<'de, D>(deserializer: D) -> Result<Vec<PinConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect())
}

/// 语言字段容错：任何无法识别的取值都只回退中文，不让整份配置解析失败。
fn lenient_locale<'de, D>(deserializer: D) -> Result<LocalePreference, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

/// 仅配置读取允许容错；未知字符串及错误类型均回退默认修饰键。
fn lenient_gesture_modifier<'de, D>(deserializer: D) -> Result<GestureModifier, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

impl Default for Settings {
    /// 提供首版配置、默认快捷键与手势，允许旧配置缺失新增字段。
    fn default() -> Self {
        Self {
            schema_version: 1,
            locale: LocalePreference::default(),
            gesture_modifier: GestureModifier::default(),
            restore_session: true,
            manager_shortcut: "Ctrl+Alt+P".into(),
            recover_shortcut: "Ctrl+Alt+B".into(),
            pins: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    /// 严格校验：只接受三个约定值字符串，非法输入由命令层返回明确错误。
    pub locale: String,
    /// 不使用磁盘配置的容错规则，避免错误输入被静默保存为 Alt。
    pub gesture_modifier: String,
    pub restore_session: bool,
    pub manager_shortcut: String,
    pub recover_shortcut: String,
}

/// 全局设置的可编辑部分，不包含页面配置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSettings {
    pub locale: LocalePreference,
    pub gesture_modifier: GestureModifier,
    pub restore_session: bool,
    pub manager_shortcut: String,
    pub recover_shortcut: String,
}

impl WorkspaceSettings {
    /// 从 IPC 输入构造候选设置，语言和手势取值必须落在约定集合内。
    pub fn from_input(input: &SettingsInput) -> Result<Self, AppMessage> {
        let locale = match input.locale.as_str() {
            "zh-CN" => LocalePreference::ZhCn,
            "en-US" => LocalePreference::EnUs,
            "system" => LocalePreference::System,
            _ => return Err(AppMessage::new("errors.localeInvalid")),
        };
        let gesture_modifier = match input.gesture_modifier.as_str() {
            "alt" => GestureModifier::Alt,
            "ctrl" => GestureModifier::Ctrl,
            "shift" => GestureModifier::Shift,
            "disabled" => GestureModifier::Disabled,
            _ => return Err(AppMessage::new("errors.gestureModifierInvalid")),
        };
        Ok(Self {
            locale,
            gesture_modifier,
            restore_session: input.restore_session,
            manager_shortcut: input.manager_shortcut.clone(),
            recover_shortcut: input.recover_shortcut.clone(),
        })
    }

    /// 从当前配置读出全部可编辑字段，用于失败回滚前的快照。
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            locale: settings.locale,
            gesture_modifier: settings.gesture_modifier,
            restore_session: settings.restore_session,
            manager_shortcut: settings.manager_shortcut.clone(),
            recover_shortcut: settings.recover_shortcut.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 构造合法 IPC 设置，并单独替换修饰键以检查严格校验边界。
    fn input_value(modifier: serde_json::Value) -> serde_json::Value {
        json!({
            "locale": "en-US",
            "gestureModifier": modifier,
            "restoreSession": false,
            "managerShortcut": "Ctrl+Alt+J",
            "recoverShortcut": "Ctrl+Alt+K"
        })
    }

    #[test]
    /// IPC 四个合法值必须准确映射，且保留其他可编辑字段。
    fn input_accepts_all_gesture_modifiers() {
        for (text, expected) in [
            ("alt", GestureModifier::Alt),
            ("ctrl", GestureModifier::Ctrl),
            ("shift", GestureModifier::Shift),
            ("disabled", GestureModifier::Disabled),
        ] {
            let input = serde_json::from_value(input_value(json!(text))).unwrap();
            let settings = WorkspaceSettings::from_input(&input).unwrap();
            assert_eq!(settings.gesture_modifier, expected);
            assert_eq!(settings.locale, LocalePreference::EnUs);
            assert!(!settings.restore_session);
            assert_eq!(settings.manager_shortcut, "Ctrl+Alt+J");
            assert_eq!(settings.recover_shortcut, "Ctrl+Alt+K");
        }
    }

    #[test]
    /// 未知值、大小写和额外空白均明确报错，不按磁盘规则回退。
    fn input_rejects_invalid_gesture_strings() {
        for text in [
            "", "Alt", "CTRL", "control", "meta", "none", " alt", "shift ",
        ] {
            let input = serde_json::from_value(input_value(json!(text))).unwrap();
            let error = WorkspaceSettings::from_input(&input).unwrap_err();
            assert_eq!(error.code, "errors.gestureModifierInvalid");
        }
    }

    #[test]
    /// IPC 缺失修饰键或传入非字符串时在反序列化阶段拒绝。
    fn input_rejects_missing_or_non_string_gesture() {
        for value in [json!(null), json!(123), json!(true), json!([]), json!({})] {
            assert!(serde_json::from_value::<SettingsInput>(input_value(value)).is_err());
        }
        let mut missing = input_value(json!("alt"));
        missing.as_object_mut().unwrap().remove("gestureModifier");
        assert!(serde_json::from_value::<SettingsInput>(missing).is_err());
    }

    #[test]
    /// 新增修饰键不改变既有语言字段的严格校验规则。
    fn input_still_rejects_invalid_locale() {
        let mut value = input_value(json!("ctrl"));
        value["locale"] = json!("fr-FR");
        let input = serde_json::from_value(value).unwrap();
        assert_eq!(
            WorkspaceSettings::from_input(&input).unwrap_err().code,
            "errors.localeInvalid"
        );
    }
}
