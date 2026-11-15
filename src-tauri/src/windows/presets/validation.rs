// 功能：统一校验可迁移预设，并按明确策略合并页面自己的预设列表。
use super::{normalize, MAX_CUSTOM, MAX_NAME};
use crate::{
    errors::AppMessage,
    models::{PinConfig, PresetFields, PresetInput, WindowPreset},
};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncPolicy {
    #[default]
    Keep,
    Merge,
    Replace,
}

/// ID 用于命令和原生菜单，只允许有界 ASCII 标识，且唯一性限定在所属页面内。
pub fn validate_id(id: &str) -> Result<(), AppMessage> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(AppMessage::new("errors.presetIdInvalid"));
    }
    Ok(())
}

/// 数值先严格校验再归一化；包可跨平台传输，平台能力只在显式编辑或应用时处理。
pub fn validate_fields(fields: &mut PresetFields) -> Result<(), AppMessage> {
    if !crate::windows::limits::contains(fields.width, fields.height)
        || !fields.fx.is_finite()
        || !fields.fy.is_finite()
        || !(0. ..=1.).contains(&fields.fx)
        || !(0. ..=1.).contains(&fields.fy)
    {
        return Err(AppMessage::new("errors.presetFieldsInvalid"));
    }
    normalize(fields);
    Ok(())
}

/// 名称与布局采用与工作台相同的限制，不把未知平台能力从原始模板中删掉。
pub fn validate_input(input: &mut PresetInput) -> Result<(), AppMessage> {
    input.name = input.name.trim().into();
    if input.name.is_empty() || input.name.chars().count() > MAX_NAME {
        return Err(AppMessage::new("errors.presetNameRange").arg("max", MAX_NAME));
    }
    validate_fields(&mut input.fields)
}

/// 外部包必须整组合法；不复用配置恢复时逐条容错的反序列化规则。
pub fn validate_list(presets: &mut [WindowPreset]) -> Result<(), AppMessage> {
    if presets.len() > MAX_CUSTOM {
        return Err(AppMessage::new("errors.presetLimit").arg("max", MAX_CUSTOM));
    }
    let mut ids = HashSet::new();
    for preset in presets {
        validate_id(&preset.id)?;
        if !ids.insert(preset.id.clone()) {
            return Err(AppMessage::new("errors.presetIdInvalid"));
        }
        let mut input = PresetInput {
            name: preset.name.clone(),
            fields: preset.fields.clone(),
        };
        validate_input(&mut input)?;
        preset.name = input.name;
        preset.fields = input.fields;
    }
    Ok(())
}

/// 合并只追加新 ID，同 ID 的不同内容明确冲突；失败时不改变原列表或当前窗口。
pub fn sync(
    pin: &mut PinConfig,
    incoming: &[WindowPreset],
    policy: SyncPolicy,
) -> Result<(), AppMessage> {
    if matches!(policy, SyncPolicy::Keep) {
        return Ok(());
    }
    let mut next = match policy {
        SyncPolicy::Replace => incoming.to_vec(),
        _ => pin.presets.clone(),
    };
    if matches!(policy, SyncPolicy::Merge) {
        for preset in incoming {
            if let Some(existing) = next.iter().find(|item| item.id == preset.id) {
                if existing.name != preset.name || existing.fields != preset.fields {
                    return Err(AppMessage::new("errors.presetConflict").arg("id", &preset.id));
                }
            } else {
                next.push(preset.clone());
            }
        }
    }
    validate_list(&mut next)?;
    if let Some(id) = &pin.preset {
        let before = pin.presets.iter().find(|p| &p.id == id);
        let after = next.iter().find(|p| &p.id == id);
        if before.map(|p| &p.fields) != after.map(|p| &p.fields) {
            pin.preset = None;
        }
    }
    pin.presets = next;
    Ok(())
}
