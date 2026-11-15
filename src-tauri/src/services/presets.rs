// 功能：为已鉴权 CLI 提供页面预设查询与带实例、修订校验的串行变更。
use super::{pages, source::native};
use crate::{automation, models::PresetInput, state::AppState, windows::presets};
use pinpage_protocol::Failure;
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Change {
    id: String,
    preset_id: String,
    expected_revision: u64,
    input: Option<Value>,
    name: Option<String>,
}

/// 返回所属页面的完整模板列表；匹配标记只是当前状态，不代表以后窗口仍会匹配。
pub fn list(app: &AppHandle, id: &str) -> Result<Value, Failure> {
    let state = app.state::<AppState>();
    let _lock = state.operation.lock().unwrap();
    let pin = automation::pin(app, id)?;
    Ok(
        json!({"pageId":id,"presets":pin.presets,"activePresetId":pin.preset,"revision":automation::revision(app, &pin)}),
    )
}

/// 拒绝模板文件中的未知字段，避免把来源、授权或拼错的窗口参数当成已保存。
fn input(value: Value) -> Result<PresetInput, Failure> {
    let allowed = [
        "name",
        "mode",
        "alwaysOnTop",
        "decorations",
        "rounded",
        "transparentBackground",
        "positionLocked",
        "sizeLocked",
        "width",
        "height",
        "fx",
        "fy",
        "fullscreen",
    ];
    if value
        .as_object()
        .is_none_or(|object| object.keys().any(|key| !allowed.contains(&key.as_str())))
    {
        return Err(Failure::new("invalidArgument", "预设文件包含未知字段"));
    }
    serde_json::from_value(value).map_err(|e| Failure::new("invalidArgument", e))
}

/// 复用工作台的保存、删除和应用事务；校验与提交共用操作锁，窗口编辑不会绕过修订检查。
pub fn mutate(
    app: &AppHandle,
    method: &str,
    params: Value,
    expected: Option<&str>,
    operation: &str,
) -> Result<Value, Failure> {
    let change: Change =
        serde_json::from_value(params).map_err(|e| Failure::new("invalidArgument", e))?;
    let state = app.state::<AppState>();
    let _lock = state.operation.lock().unwrap();
    state.ensure_mutable().map_err(native)?;
    let pin = automation::pin(app, &change.id)?;
    if expected
        != Some(
            app.state::<automation::AutomationState>()
                .instance_id
                .as_str(),
        )
        || change.expected_revision != automation::revision(app, &pin)
    {
        return Err(Failure::new("revisionConflict", "实例或页面修订已变化"));
    }
    presets::validation::validate_id(&change.preset_id).map_err(native)?;
    let mut warnings: Vec<&str> = vec![];
    match method {
        "window.presets.put" if change.name.is_none() => {
            let value = change
                .input
                .ok_or_else(|| Failure::new("invalidArgument", "缺少预设文件内容"))?;
            presets::put(app, &change.id, &change.preset_id, input(value)?).map_err(native)?;
        }
        "window.presets.save" if change.input.is_none() => {
            let name = change
                .name
                .ok_or_else(|| Failure::new("invalidArgument", "缺少预设名称"))?;
            presets::capture(app, &pin, &name, &change.preset_id).map_err(native)?;
        }
        "window.presets.delete" if change.name.is_none() && change.input.is_none() => {
            presets::delete(app, &change.id, &change.preset_id).map_err(native)?;
        }
        "window.presets.apply" if change.name.is_none() && change.input.is_none() => {
            let preset = presets::find(&pin, &change.preset_id)
                .ok_or_else(|| Failure::new("presetNotFound", "预设不存在"))?;
            if crate::platform::validate_preset(crate::platform::capabilities(), &preset.fields)
                .is_err()
            {
                warnings.push("platformFallback");
            }
            presets::apply(app, &pin, &change.preset_id).map_err(native)?;
        }
        _ => return Err(Failure::new("invalidArgument", "预设操作参数不匹配")),
    }
    let mut result = pages::outcome(app, &change.id)?;
    result["presetId"] = json!(change.preset_id);
    result["warnings"] = json!(warnings);
    // 预设变更不导航，不等待原页面恰好正在进行的加载。
    result["loadId"] = Value::Null;
    result["loadState"] = json!("notRequested");
    automation::record_commit(app, operation, result.clone());
    Ok(result)
}
