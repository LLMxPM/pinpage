// 功能：定义跨 Rust 与前端传递的结构化消息，替代已经翻译好的字符串。
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display};

/// 稳定消息契约：`code` 指向 native 资源键，`args` 是受控文本参数，
/// `detail` 只保留系统或第三方诊断原文，不参与翻译。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMessage {
    pub code: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppMessage {
    /// 以 native 资源键构造消息，例如 `errors.invalidUrl`。
    pub fn new(code: &str) -> Self {
        Self {
            code: code.into(),
            ..Default::default()
        }
    }

    /// 追加一个具名参数；参数值始终以文本插入，不会被再次解析为模板。
    pub fn arg(mut self, name: &str, value: impl Display) -> Self {
        self.args.insert(name.into(), value.to_string());
        self
    }

    /// 追加诊断详情，用于系统和第三方错误，允许保留原始语言。
    pub fn detail(mut self, value: impl Display) -> Self {
        let text = value.to_string();
        if !text.trim().is_empty() {
            self.detail = Some(text);
        }
        self
    }
}

/// 把任意错误值收敛为可读消息；非本应用产生的错误统一回退到未知消息。
pub fn unknown(detail: impl Display) -> AppMessage {
    AppMessage::new("errors.unknown").detail(detail)
}
