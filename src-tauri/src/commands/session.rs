// 功能：提供原生选择器导入、页面包导出与内容页数据回传三类会话命令。
use crate::{
    errors::AppMessage,
    examples, i18n, import,
    models::{ExportResult, ImportMode, ImportPreview, PageSource},
    package,
    state::{self, AppState},
    windows,
};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// 单页回传数据的体积上限，容纳 IndexedDB 与内嵌的二进制内容。
const MAX_PAGE_DATA: usize = 24 * 1024 * 1024;
/// 选择保存位置之后等待页面回传的余量，通常数据已经到达。
const DATA_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

/// 导入预览：目录、单文件与页面包走原生选择器，内置示例由应用直接提供。
///
/// 选择器标题与过滤器名称在打开时按当前语言渲染，之后切换语言不影响已打开的对话框。
#[tauri::command]
pub async fn import_page(
    window: WebviewWindow,
    app: AppHandle,
    mode: ImportMode,
) -> Result<Option<ImportPreview>, AppMessage> {
    windows::ensure_manager(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let locale = state::locale(&app);
        let name = |key: &str| i18n::text(locale, key, &Default::default());
        let dialog = app.dialog().file();
        let (path, candidates, label, files, bytes) = match mode {
            ImportMode::Directory => {
                let Some(selected) = dialog
                    .set_title(name("picker.folderTitle"))
                    .blocking_pick_folder()
                else {
                    return Ok(None);
                };
                let root = selected
                    .into_path()
                    .map_err(|error| AppMessage::new("errors.folderPathInvalid").detail(error))?;
                let candidates = import::entry_candidates(&root)?;
                if candidates.is_empty() {
                    return Err(AppMessage::new("errors.noRootHtml"));
                }
                // 目录里常混入与页面无关的大文件，超限时提示更可行的替代做法。
                let (files, bytes) = import::measure(&root).map_err(|error| {
                    AppMessage::new("errors.measureFailed").detail(error.detail.unwrap_or_default())
                })?;
                let label = file_label(&root, &name("picker.folderLabel"));
                (root, candidates, label, files, bytes)
            }
            ImportMode::File => {
                let Some(selected) = dialog
                    .set_title(name("picker.fileTitle"))
                    .add_filter(name("picker.filterWeb"), &["html", "htm"])
                    .blocking_pick_file()
                else {
                    return Ok(None);
                };
                let file = selected
                    .into_path()
                    .map_err(|error| AppMessage::new("errors.filePathInvalid").detail(error))?;
                let file_name = file
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| AppMessage::new("errors.fileNameInvalid"))?
                    .to_string();
                let bytes = std::fs::metadata(&file)
                    .map_err(|_| AppMessage::new("errors.readFileFailed"))?
                    .len();
                (file, vec![file_name.clone()], file_name, 1, bytes)
            }
            ImportMode::Package => {
                let Some(selected) = dialog
                    .set_title(name("picker.packageTitle"))
                    .add_filter(name("picker.filterPackage"), &["pinpage"])
                    .blocking_pick_file()
                else {
                    return Ok(None);
                };
                let file = selected
                    .into_path()
                    .map_err(|error| AppMessage::new("errors.filePathInvalid").detail(error))?;
                let manifest = package::read_manifest(&file)?;
                let bytes = std::fs::metadata(&file)
                    .map_err(|_| AppMessage::new("errors.readFileFailed"))?
                    .len();
                let label = file_label(&file, &name("picker.packageLabel"));
                (file, vec![manifest.entry], label, manifest.files, bytes)
            }
            // 示例使用随包资源，不需要选择器，也不产生外部授权路径。
            ImportMode::Example => {
                let (files, bytes) = examples::measure();
                (
                    std::path::PathBuf::new(),
                    vec![examples::ENTRY.to_string()],
                    name(examples::LABEL_KEY),
                    files,
                    bytes,
                )
            }
        };
        let ticket = uuid::Uuid::new_v4().to_string();
        let state = app.state::<AppState>();
        let mut grants = state.grants.lock().unwrap();
        grants.clear();
        grants.insert(
            ticket.clone(),
            import::ImportGrant {
                path,
                mode,
                candidates: candidates.clone(),
            },
        );
        Ok(Some(ImportPreview {
            ticket,
            mode,
            label,
            candidates,
            files,
            bytes,
        }))
    })
    .await
    .map_err(|error| AppMessage::new("errors.unknown").detail(error))?
}

/// 把导入页的站点副本与页面数据导出为页面包，取消保存时返回空。
#[tauri::command]
pub async fn export_page(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
) -> Result<Option<ExportResult>, AppMessage> {
    windows::ensure_manager(&window)?;
    let pin = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|pin| pin.id == id)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))?;
    if !matches!(pin.source, PageSource::Bundled { .. }) {
        return Err(AppMessage::new("errors.exportNeedsImport"));
    }
    // 数据只能从运行中的页面读取，DOM 与脚本状态本就不在导出范围内。
    let page = app
        .get_webview_window(&id)
        .ok_or_else(|| AppMessage::new("errors.exportNeedsOpen"))?;
    app.state::<AppState>()
        .export_pending
        .lock()
        .unwrap()
        .insert(id.clone());
    // 清掉上一次超时后迟到或未取走的数据，避免串到本次导出。
    app.state::<AppState>()
        .export_data
        .lock()
        .unwrap()
        .remove(&id);
    if let Err(error) = page.eval(crate::data::collect_script(&id)) {
        app.state::<AppState>()
            .export_pending
            .lock()
            .unwrap()
            .remove(&id);
        return Err(AppMessage::new("errors.collectDataFailed").detail(error));
    }

    // 先让用户选择保存位置，页面数据通常在这段时间内已经回传完成。
    let save_app = app.clone();
    let suggested = format!("{}.pinpage", safe_file_name(&pin.name));
    let target = tauri::async_runtime::spawn_blocking(move || {
        let locale = state::locale(&save_app);
        let name = |key: &str| i18n::text(locale, key, &Default::default());
        save_app
            .dialog()
            .file()
            .set_title(name("picker.exportTitle"))
            .set_file_name(&suggested)
            .add_filter(name("picker.filterPackage"), &["pinpage"])
            .blocking_save_file()
    })
    .await
    .map_err(|error| AppMessage::new("errors.unknown").detail(error))?;
    let Some(target) = target else {
        app.state::<AppState>()
            .export_pending
            .lock()
            .unwrap()
            .remove(&id);
        return Ok(None);
    };

    let wait_app = app.clone();
    let wait_id = id.clone();
    let payload = tauri::async_runtime::spawn_blocking(move || {
        let deadline = std::time::Instant::now() + DATA_TIMEOUT;
        while std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if let Some(value) = wait_app
                .state::<AppState>()
                .export_data
                .lock()
                .unwrap()
                .remove(&wait_id)
            {
                return Some(value);
            }
        }
        None
    })
    .await
    .map_err(|error| AppMessage::new("errors.unknown").detail(error))?;
    app.state::<AppState>()
        .export_pending
        .lock()
        .unwrap()
        .remove(&id);

    let target = target
        .into_path()
        .map_err(|error| AppMessage::new("errors.savePathInvalid").detail(error))?;
    let path_text = target.to_string_lossy().into_owned();
    let with_data = payload.is_some();
    let data_bytes = payload
        .as_ref()
        .map(|value| value.len() as u64)
        .unwrap_or(0);
    let pack_app = app.clone();
    let files = tauri::async_runtime::spawn_blocking(move || {
        package::write(&pack_app, &pin, payload.as_deref(), &target)
    })
    .await
    .map_err(|error| AppMessage::new("errors.unknown").detail(error))??;
    Ok(Some(ExportResult {
        path: path_text,
        files,
        with_data,
        data_bytes,
    }))
}

/// 内容页在导出会话中回传自身 Web 存储，其他时间一律拒绝。
#[tauri::command]
pub async fn submit_page_data(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    payload: String,
) -> Result<(), AppMessage> {
    if window.label() != id {
        return Err(AppMessage::new("errors.dataIdentity"));
    }
    if payload.len() > MAX_PAGE_DATA {
        return Err(AppMessage::new("errors.dataTooLarge"));
    }
    let state = app.state::<AppState>();
    if !state.export_pending.lock().unwrap().contains(&id) {
        return Err(AppMessage::new("errors.noExportSession"));
    }
    state.export_data.lock().unwrap().insert(id, payload);
    Ok(())
}

/// 取路径的最后一段作为展示名，路径异常时使用兜底文字。
fn file_label(path: &std::path::Path, fallback: &str) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(fallback)
        .to_string()
}

/// 去掉文件名中的非法字符，避免保存对话框收到无效默认名。
fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|value| {
            if value.is_control() || "/\\:*?\"<>|".contains(value) {
                '_'
            } else {
                value
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "pinpage".into()
    } else {
        trimmed.chars().take(60).collect()
    }
}
