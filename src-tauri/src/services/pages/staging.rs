// 功能：准备可恢复的站点暂存事务，并为工作台和 CLI 提供一致的页面默认值。
use crate::{
    models::{PageSource, PinConfig, WindowMode},
    services::{
        source::{self, Prepared},
        transaction::SiteTransaction,
    },
};
use pinpage_protocol::Failure;
use std::path::PathBuf;
use tauri::AppHandle;

pub(super) struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        if let Some(parent) = self.0.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}
/// 源码准备在操作锁之外执行；每次独立暂存，互不覆盖并发请求。
pub(super) fn prepare(
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
/// 在提交锁内把唯一暂存目录移入固定 incoming，然后创建持久交换记录。
pub(super) fn begin_stage(
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
