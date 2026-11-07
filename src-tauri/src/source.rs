// 功能：把管理界面提交的来源输入兑换为受校验的页面来源，创建与更新来源共用同一实现。
//
// 网址来源只做纯校验，不接触磁盘；导入来源必须携带原生选择器或内置来源签发的一次性票据，
// 入口需在签发时记录的候选之内，新副本先落进暂存目录再与站点副本交换，
// 因此替换失败时页面仍指向原来完好的副本。复制成功后票据作废。
use crate::{
    content,
    errors::AppMessage,
    examples, import,
    models::{ImportMode, PageSource, SourceInput},
    package,
    state::AppState,
};
use std::path::Path;
use tauri::{AppHandle, Manager};

/// 解析来源输入，产出可直接写入配置的 `PageSource`。
///
/// `id` 决定站点副本的位置。更新已有页面时旧副本整体作废，新目录不会残留上次导入的文件，
/// 入口相对路径也重新按新来源确认；创建时目录尚不存在，暂存与交换同样适用。
pub fn prepare(app: &AppHandle, id: &str, input: &SourceInput) -> Result<PageSource, AppMessage> {
    match input {
        SourceInput::Url { address } => Ok(PageSource::Url {
            url: content::normalize_url(address)?,
        }),
        SourceInput::Import { ticket, entry } => redeem(app, id, ticket, entry),
    }
}

/// 用票据把选中的本地来源换成页面副本，返回导入来源的入口相对路径。
fn redeem(app: &AppHandle, id: &str, ticket: &str, entry: &str) -> Result<PageSource, AppMessage> {
    let state_ref = app.state::<AppState>();
    let grant = state_ref
        .grants
        .lock()
        .unwrap()
        .get(ticket)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.grantExpired"))?;
    verify_entry(&grant, entry)?;
    let incoming = import::incoming_root(app, id)?;
    // 上一次中断留下的暂存目录先清掉，避免新旧文件混在同一份副本里。
    let _ = std::fs::remove_dir_all(&incoming);
    let prepared = match grant.mode {
        ImportMode::Directory => {
            import::copy_site(&grant.path, &incoming).map(|_| (entry.to_string(), None))
        }
        ImportMode::File => import::copy_entry(&grant.path, &incoming).map(|name| (name, None)),
        ImportMode::Package => package::extract(&grant.path, &incoming)
            .map(|extracted| (extracted.entry, extracted.storage)),
        ImportMode::Example => {
            examples::write_site(&incoming, grant.example).map(|name| (name, None))
        }
    };
    let (entry, storage) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            discard(app, id, &incoming);
            return Err(error);
        }
    };
    if let Err(error) = import::swap_site(app, id) {
        discard(app, id, &incoming);
        return Err(error);
    }
    // 随包数据只用于首次打开：包装入口后交给引导页，读取即删除。
    if let Some(storage) = storage {
        if let Err(error) = write_pending(app, id, &entry, &storage) {
            discard(app, id, &incoming);
            return Err(error);
        }
    }
    // 成功后作废票据，重复提交同一预览会被拒绝而不是再复制一份。
    state_ref.grants.lock().unwrap().remove(ticket);
    Ok(PageSource::Bundled { entry })
}

/// 校验入口确实是票据签发时记录的候选之一，防止前端伪造路径。
fn verify_entry(grant: &import::ImportGrant, entry: &str) -> Result<(), AppMessage> {
    match grant.mode {
        ImportMode::Directory => import::verify_entry(&grant.path, entry, &grant.candidates),
        ImportMode::File => {
            let selected = grant.path.file_name().and_then(|value| value.to_str());
            if selected != Some(entry) || !grant.path.is_file() {
                return Err(AppMessage::new("errors.entryMismatch"));
            }
            Ok(())
        }
        ImportMode::Package | ImportMode::Example => {
            if !grant.candidates.iter().any(|candidate| candidate == entry) {
                return Err(AppMessage::new("errors.entryMismatchSource"));
            }
            Ok(())
        }
    }
}

/// 丢弃半成品副本：清掉暂存目录；页面已没有任何站点副本时（新建场景）连页面目录一起删除。
fn discard(app: &AppHandle, id: &str, incoming: &Path) {
    let _ = std::fs::remove_dir_all(incoming);
    let has_site = import::site_root(app, id)
        .map(|site| site.exists())
        .unwrap_or(false);
    if !has_site {
        let _ = import::remove_page(app, id);
    }
}

/// 保存页面包携带的存储快照，入口路径随快照一起写入，引导页据此跳转。
fn write_pending(app: &AppHandle, id: &str, entry: &str, storage: &str) -> Result<(), AppMessage> {
    let payload: serde_json::Value = serde_json::from_str(storage)
        .map_err(|error| AppMessage::new("errors.pageDataInvalid").detail(error))?;
    let document = serde_json::json!({ "entry": entry, "payload": payload });
    let pending = import::pages_root(app)?.join(id).join(import::PENDING_DATA);
    std::fs::write(&pending, document.to_string())
        .map_err(|error| AppMessage::new("errors.writePageDataFailed").detail(error))
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
