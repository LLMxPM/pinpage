// 功能：实现本机页面安装和同 ID 更新，协调修订、去重与可恢复提交。
use super::{
    source::{self, Prepared},
    transaction::SiteTransaction,
};
use crate::{
    automation,
    models::{PageSource, PinConfig, WindowMode},
    state::{self, AppState},
};
use pinpage_protocol::Failure;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

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
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Update {
    id: String,
    source: PathBuf,
    entry: Option<String>,
    expected_revision: u64,
}
struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        if let Some(parent) = self.0.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}
/// 源码准备在操作锁之外执行；每次独立暂存，互不覆盖并发请求。
fn prepare(
    app: &AppHandle,
    id: &str,
    path: &std::path::Path,
    entry: Option<&str>,
) -> Result<(Stage, Prepared), Failure> {
    let root = crate::import::site_root(app, id)
        .map_err(source::native)?
        .parent()
        .unwrap()
        .to_owned();
    let stage = Stage(root.join(format!("prepared-{}", uuid::Uuid::new_v4())));
    let prepared = source::prepare(path, entry, &stage.0)?;
    Ok((stage, prepared))
}
/// 页面初始值由管理命令和自动化服务共用；来源和授权由各入口先行校验。
pub fn new_pin(id: String, name: &str, source: PageSource) -> PinConfig {
    let fallback = match &source {
        PageSource::Url { url } => url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .unwrap_or_else(|| "网页".into()),
        PageSource::Bundled { entry } => std::path::Path::new(entry)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    };
    PinConfig {
        id,
        name: if name.trim().is_empty() {
            fallback
        } else {
            name.trim().chars().take(80).collect()
        },
        source,
        width: 420.,
        height: 360.,
        x: None,
        y: None,
        scale: 1.,
        always_on_top: false,
        decorations: true,
        enabled: true,
        mode: WindowMode::Floating,
        rounded: false,
        transparent_background: false,
        position_locked: false,
        size_locked: false,
        page_context_menu: false,
        preset: None,
        presets: vec![],
        folder_root: None,
    }
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
                storage: None,
                digest: pinpage_skills::digest(url.as_bytes()),
            },
        )
    };
    let digest=pinpage_skills::digest(&serde_json::to_vec(&json!({"source":prepared.digest,"entry":prepared.source,"name":input.name,"closed":input.closed,"preset":input.apply_package_preset})).unwrap());
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
/// 在提交锁内把唯一暂存目录移入固定 incoming，然后创建持久交换记录。
fn begin_stage(
    app: &AppHandle,
    id: &str,
    stage: Option<&Stage>,
) -> Result<Option<SiteTransaction>, Failure> {
    let Some(stage) = stage else { return Ok(None) };
    let incoming = crate::import::incoming_root(app, id).map_err(source::native)?;
    if incoming.exists() {
        return Err(Failure::new("transactionFailed", "已有未恢复的暂存副本"));
    }
    std::fs::rename(&stage.0, &incoming).map_err(|e| Failure::new("transactionFailed", e))?;
    SiteTransaction::begin(incoming.parent().unwrap().to_owned()).map(Some)
}
/// 提交事实与加载结果分开表达；关闭页不会因更新而开启。
pub fn outcome(app: &AppHandle, id: &str) -> Result<Value, Failure> {
    let pin = automation::pin(app, id)?;
    let inspected = automation::inspect(app, id)?;
    Ok(
        json!({"pageId":id,"revision":inspected["revision"],"loadId":if pin.enabled{inspected["loadId"].clone()}else{Value::Null},"loadState":if pin.enabled{inspected["loadState"].clone()}else{json!("notRequested")},"committed":true,"phase":"committed"}),
    )
}
