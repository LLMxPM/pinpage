// 功能：将选择器票据兑换为受校验来源，复用可恢复站点事务并区分首次恢复与更新。
use crate::{
    content,
    errors::AppMessage,
    examples, import,
    models::{ImportMode, PageSource, PresetFields, SourceInput},
    services::{source, transaction::SiteTransaction},
    state::AppState,
};
use tauri::{AppHandle, Manager};

pub struct PreparedSource {
    pub source: PageSource,
    pub preset: Option<PresetFields>,
    transaction: Option<SiteTransaction>,
}
impl PreparedSource {
    /// 配置提交前写入标记，保存成功后才能清理旧副本。
    pub fn mark(&self, app: &AppHandle, id: &str) {
        if let Some(tx) = &self.transaction {
            tx.mark(&mut app.state::<AppState>().settings.lock().unwrap(), id);
        }
    }
    /// 配置已经持久化，文件与快照不再回滚。
    pub fn committed(self) {
        if let Some(tx) = self.transaction {
            tx.committed();
        }
    }
}
/// 更新忽略包内快照及布局，旧 WebView 保持同源身份。
pub fn prepare(
    app: &AppHandle,
    id: &str,
    input: &SourceInput,
) -> Result<PreparedSource, AppMessage> {
    prepare_inner(app, id, input, false)
}
/// 新建允许恢复合法随包存储，布局交给用户选择。
pub fn prepare_with_window(
    app: &AppHandle,
    id: &str,
    input: &SourceInput,
) -> Result<PreparedSource, AppMessage> {
    prepare_inner(app, id, input, true)
}
/// 调用方必须验证工作台，票据只来自原生选择器和内置示例。
fn prepare_inner(
    app: &AppHandle,
    id: &str,
    input: &SourceInput,
    restore: bool,
) -> Result<PreparedSource, AppMessage> {
    let SourceInput::Import { ticket, entry } = input else {
        let SourceInput::Url { address } = input else {
            unreachable!()
        };
        return Ok(PreparedSource {
            source: PageSource::Url {
                url: content::normalize_url(address)?,
            },
            preset: None,
            transaction: None,
        });
    };
    let state = app.state::<AppState>();
    let grant = state
        .grants
        .lock()
        .unwrap()
        .get(ticket)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.grantExpired"))?;
    verify_entry(&grant, entry)?;
    let incoming = import::incoming_root(app, id)?;
    if incoming.exists() {
        return Err(AppMessage::new("errors.sourceReplaceFailed").detail("存在未恢复的暂存目录"));
    }
    let prepared = if grant.mode == ImportMode::Example {
        examples::extract(&incoming, grant.example).map(|p| source::Prepared {
            source: PageSource::Bundled { entry: p.entry },
            preset: p.window_preset,
            storage: p.storage,
            digest: String::new(),
        })
    } else {
        source::prepare(
            &grant.path,
            if grant.mode == ImportMode::Directory {
                Some(entry)
            } else {
                None
            },
            &incoming,
        )
        .map_err(failed)
    };
    let prepared = match prepared {
        Ok(p) => p,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&incoming);
            return Err(error);
        }
    };
    let transaction =
        SiteTransaction::begin(incoming.parent().unwrap().to_owned()).map_err(failed)?;
    let pending = incoming.parent().unwrap().join(import::PENDING_DATA);
    if restore {
        if let Some(storage) = prepared.storage {
            let payload: serde_json::Value = serde_json::from_str(&storage)
                .map_err(|e| AppMessage::new("errors.pageDataInvalid").detail(e))?;
            let entry = match &prepared.source {
                PageSource::Bundled { entry } => entry,
                _ => unreachable!(),
            };
            std::fs::write(
                &pending,
                serde_json::json!({"entry":entry,"payload":payload}).to_string(),
            )
            .map_err(|e| AppMessage::new("errors.writePageDataFailed").detail(e))?;
        }
    } else if pending.exists() {
        std::fs::remove_file(&pending)
            .map_err(|e| AppMessage::new("errors.writePageDataFailed").detail(e))?;
    }
    state.grants.lock().unwrap().remove(ticket);
    Ok(PreparedSource {
        source: prepared.source,
        preset: prepared.preset,
        transaction: Some(transaction),
    })
}
/// 验证票据入口不能被前端改为另一个文件或路径。
fn verify_entry(grant: &import::ImportGrant, entry: &str) -> Result<(), AppMessage> {
    match grant.mode {
        ImportMode::Directory => import::verify_entry(&grant.path, entry, &grant.candidates),
        ImportMode::File => {
            if grant.path.file_name().and_then(|s| s.to_str()) != Some(entry)
                || !grant.path.is_file()
            {
                Err(AppMessage::new("errors.entryMismatch"))
            } else {
                Ok(())
            }
        }
        ImportMode::Package | ImportMode::Example => {
            if grant.candidates.iter().any(|c| c == entry) {
                Ok(())
            } else {
                Err(AppMessage::new("errors.entryMismatchSource"))
            }
        }
    }
}
/// 原生界面沿用消息码，CLI 原因保留在详情中。
fn failed(e: pinpage_protocol::Failure) -> AppMessage {
    AppMessage::new("errors.sourceReplaceFailed").detail(e)
}
#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
