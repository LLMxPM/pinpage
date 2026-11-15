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

/// 导入预览：目录、单文件与页面包走原生选择器，内置示例由应用直接提供。
///
/// 选择器标题与过滤器名称在打开时按当前语言渲染，之后切换语言不影响已打开的对话框。
#[tauri::command]
pub async fn import_page(
    window: WebviewWindow,
    app: AppHandle,
    mode: ImportMode,
    example: Option<examples::ExampleId>,
) -> Result<Option<ImportPreview>, AppMessage> {
    windows::ensure_manager(&window)?;
    let example = example.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        let locale = state::locale(&app);
        let name = |key: &str| i18n::text(locale, key, &Default::default());
        let dialog = app.dialog().file();
        let mut window_preset = None;
        let mut window_presets = 0;
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
                window_preset = manifest.window_preset;
                window_presets = manifest.window_presets.len();
                let bytes = std::fs::metadata(&file)
                    .map_err(|_| AppMessage::new("errors.readFileFailed"))?
                    .len();
                let label = manifest.name;
                (file, vec![manifest.entry], label, manifest.files, bytes)
            }
            // 示例使用随包资源，不需要选择器，也不产生外部授权路径。
            ImportMode::Example => {
                let (manifest, bytes) = examples::preview(example)?;
                window_preset = manifest.window_preset;
                window_presets = manifest.window_presets.len();
                (
                    std::path::PathBuf::new(),
                    vec![manifest.entry],
                    name(examples::label_key(example)),
                    manifest.files,
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
                example,
            },
        );
        Ok(Some(ImportPreview {
            ticket,
            mode,
            label,
            candidates,
            files,
            bytes,
            window_preset,
            window_presets,
        }))
    })
    .await
    .map_err(|error| AppMessage::new("errors.unknown").detail(error))?
}

/// 选择保存位置后调用与 CLI 共用的串行导出服务，取消选择不采集页面数据。
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
    if app.get_webview_window(&id).is_none() {
        return Err(AppMessage::new("errors.exportNeedsOpen"));
    }
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
        return Ok(None);
    };
    let target = target
        .into_path()
        .map_err(|error| AppMessage::new("errors.savePathInvalid").detail(error))?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::export::write(&app, &id, &target, true, true)
    })
    .await
    .map_err(|error| AppMessage::new("errors.unknown").detail(error))?
    .map(Some)
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
