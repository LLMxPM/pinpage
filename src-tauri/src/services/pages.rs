// 功能：实现本机页面安装和同 ID 更新，协调修订、去重与可恢复提交。
use super::source::{self, Prepared};
use crate::{
    automation,
    models::PageSource,
    state::{self, AppState},
};
use pinpage_protocol::Failure;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
mod staging;
pub use staging::new_pin;
use staging::{begin_stage, prepare};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Install {
    source: Option<PathBuf>,
    url: Option<String>,
    entry: Option<String>,
    name: Option<String>,
    client_key: Option<String>,
    closed: bool,
    apply_package_preset: bool,
    #[serde(default = "import_presets_default")]
    import_package_presets: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Update {
    id: String,
    source: PathBuf,
    entry: Option<String>,
    expected_revision: u64,
    #[serde(default)]
    presets: crate::windows::presets::validation::SyncPolicy,
}
/// 安装默认携带命名预设，不与初始窗口布局的选择绑定。
fn import_presets_default() -> bool {
    true
}
/// 本机安装校验来源互斥与幂等键；首次包数据仅在新记录提交时写入。
pub fn install(app: &AppHandle, input: Install, operation: &str) -> Result<Value, Failure> {
    if input.source.is_some() == input.url.is_some() || input.url.is_some() && input.entry.is_some()
    {
        return Err(Failure::new(
            "invalidSource",
            "必须选择唯一的本地来源或 URL",
        ));
    }
    if input
        .client_key
        .as_ref()
        .is_some_and(|k| k.is_empty() || k.len() > 200)
    {
        return Err(Failure::new(
            "invalidArgument",
            "client key 长度必须为 1～200 字节",
        ));
    }
    if input.name.as_ref().is_some_and(|n| n.chars().count() > 80) {
        return Err(Failure::new("invalidArgument", "名称最多 80 字符"));
    }
    let id = format!("p-{}", uuid::Uuid::new_v4().simple());
    let (stage, prepared) = if let Some(path) = input.source {
        let (stage, p) = prepare(app, &id, &path, input.entry.as_deref())?;
        (Some(stage), p)
    } else {
        let url = crate::content::normalize_url(input.url.as_deref().unwrap())
            .map_err(|e| Failure::new("invalidSource", "URL 不合法").detail(e))?;
        (
            None,
            Prepared {
                source: PageSource::Url { url: url.clone() },
                preset: None,
                presets: vec![],
                storage: None,
                digest: pinpage_skills::digest(url.as_bytes()),
            },
        )
    };
    let mut request = json!({"source":prepared.digest,"entry":prepared.source,"name":input.name,"closed":input.closed,"preset":input.apply_package_preset});
    // 默认导入列表时维持旧摘要，升级后的既有 client key 仍可去重。
    if !input.import_package_presets {
        request["presets"] = json!(false);
    }
    let digest = pinpage_skills::digest(&serde_json::to_vec(&request).unwrap());
    let state = app.state::<AppState>();
    let _lock = state.operation.lock().unwrap();
    state.ensure_mutable().map_err(source::native)?;
    let existing = input.client_key.as_ref().and_then(|key| {
        state
            .settings
            .lock()
            .unwrap()
            .automation_clients
            .get(key)
            .cloned()
    });
    if let Some(existing) = existing {
        if existing["digest"] != digest {
            return Err(Failure::new(
                "clientKeyConflict",
                "同一 client key 已用于不同安装请求",
            ));
        }
        let page = existing["pageId"]
            .as_str()
            .ok_or_else(|| Failure::new("clientKeyConflict", "去重映射损坏"))?;
        let mut result = outcome(app, page)?;
        result["deduplicated"] = json!(true);
        return Ok(result);
    }
    let mut pin = new_pin(
        id.clone(),
        input.name.as_deref().unwrap_or(""),
        prepared.source,
    );
    pin.enabled = !input.closed;
    if input.import_package_presets {
        pin.presets = prepared.presets;
    }
    if input.apply_package_preset {
        if let Some(preset) = prepared.preset {
            crate::windows::presets::initialize(app, &mut pin, preset);
        }
    }
    crate::platform::validate_pin(crate::platform::capabilities(), &pin).map_err(source::native)?;
    let previous = state.settings.lock().unwrap().clone();
    let tx = begin_stage(app, &id, stage.as_ref())?;
    if let Some(storage) = prepared.storage {
        let payload: Value =
            serde_json::from_str(&storage).map_err(|e| Failure::new("invalidSource", e))?;
        let entry = match &pin.source {
            PageSource::Bundled { entry } => entry,
            _ => unreachable!(),
        };
        std::fs::write(
            crate::import::pages_root(app)
                .map_err(source::native)?
                .join(&id)
                .join(crate::import::PENDING_DATA),
            json!({"entry":entry,"payload":payload}).to_string(),
        )
        .map_err(|e| Failure::new("executionFailed", e))?;
    }
    {
        let mut settings = state.settings.lock().unwrap();
        settings.pins.push(pin.clone());
        if let Some(tx) = &tx {
            tx.mark(&mut settings, &id);
        }
        if let Some(key) = input.client_key {
            settings
                .automation_clients
                .insert(key, json!({"digest":digest,"pageId":id}));
        }
    }
    if let Err(error) = state.save() {
        *state.settings.lock().unwrap() = previous;
        return Err(source::native(error));
    }
    if let Some(tx) = tx {
        tx.committed();
    }
    let mut result = outcome(app, &id)?;
    result["loadState"] = json!(if pin.enabled {
        "loading"
    } else {
        "notRequested"
    });
    automation::record_commit(app, operation, result.clone());
    if pin.enabled {
        crate::windows::open_pin(app, &pin)
            .map_err(|e| source::native(e).context(result.clone()))?;
        result = outcome(app, &id)?;
    }
    state::notify(app);
    crate::tray::refresh_or_report(app);
    Ok(result)
}
/// 更新只接受已导入页，准备结束后在提交锁内重新检查实例及当前修订。
pub fn update(
    app: &AppHandle,
    input: Update,
    expected: Option<&str>,
    operation: &str,
) -> Result<Value, Failure> {
    let initial = automation::pin(app, &input.id)?;
    if !matches!(
        input.presets,
        crate::windows::presets::validation::SyncPolicy::Keep
    ) && !input
        .source
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pinpage"))
    {
        return Err(Failure::new(
            "invalidArgument",
            "预设同步只接受 .pinpage 页面包",
        ));
    }
    if !matches!(initial.source, PageSource::Bundled { .. }) {
        return Err(Failure::new("invalidSource", "只能更新本地导入页"));
    }
    let (stage, prepared) = prepare(app, &input.id, &input.source, input.entry.as_deref())?;
    let state = app.state::<AppState>();
    let _lock = state.operation.lock().unwrap();
    state.ensure_mutable().map_err(source::native)?;
    let previous_pin = automation::pin(app, &input.id)?;
    if expected
        != Some(
            app.state::<automation::AutomationState>()
                .instance_id
                .as_str(),
        )
        || automation::revision(app, &previous_pin) != input.expected_revision
    {
        return Err(Failure::new("revisionConflict", "实例或页面修订已变化"));
    }
    let previous = state.settings.lock().unwrap().clone();
    let mut updated_pin = previous_pin.clone();
    crate::windows::presets::validation::sync(&mut updated_pin, &prepared.presets, input.presets)
        .map_err(source::native)?;
    let tx = begin_stage(app, &input.id, Some(&stage))?.unwrap();
    // 更新忽略包快照与布局，包括尚未打开的新安装包遗留快照。
    let pending = crate::import::pages_root(app)
        .map_err(source::native)?
        .join(&input.id)
        .join(crate::import::PENDING_DATA);
    if pending.exists() {
        std::fs::remove_file(pending).map_err(|e| Failure::new("executionFailed", e))?;
    }
    {
        let mut settings = state.settings.lock().unwrap();
        let pin = settings.pins.iter_mut().find(|p| p.id == input.id).unwrap();
        pin.source = prepared.source;
        pin.presets = updated_pin.presets;
        pin.preset = updated_pin.preset;
        pin.folder_root = None;
        tx.mark(&mut settings, &input.id);
    }
    if let Err(error) = state.save() {
        *state.settings.lock().unwrap() = previous;
        return Err(source::native(error));
    }
    tx.committed();
    automation::touch(app, &input.id);
    let mut result = outcome(app, &input.id)?;
    result["loadState"] = json!(if previous_pin.enabled {
        "loading"
    } else {
        "notRequested"
    });
    automation::record_commit(app, operation, result.clone());
    if app.get_webview_window(&input.id).is_some() {
        let pin = automation::pin(app, &input.id)?;
        crate::windows::navigate(
            app,
            &input.id,
            crate::windows::initial_url(app, &pin).map_err(source::native)?,
        )
        .map_err(source::native)?;
        result = outcome(app, &input.id)?;
    }
    state::notify(app);
    Ok(result)
}
/// 提交事实与加载结果分开表达；关闭页不会因更新而开启。
pub fn outcome(app: &AppHandle, id: &str) -> Result<Value, Failure> {
    let pin = automation::pin(app, id)?;
    let inspected = automation::inspect(app, id)?;
    Ok(
        json!({"pageId":id,"revision":inspected["revision"],"loadId":if pin.enabled{inspected["loadId"].clone()}else{Value::Null},"loadState":if pin.enabled{inspected["loadState"].clone()}else{json!("notRequested")},"committed":true,"phase":"committed"}),
    )
}
