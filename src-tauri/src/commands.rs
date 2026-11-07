// 功能：提供经过管理窗口身份校验的 IPC 命令、导入导出入口与数据回传通道。
use crate::{
    content, data, examples, import,
    models::{
        CreatePin, ExportResult, ImportMode, ImportPreview, PageSource, PinConfig, SettingsInput,
        Snapshot, SourceInput,
    },
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

/// 读取管理快照，只向受信任的管理窗口返回路径及会话配置。
#[tauri::command]
pub async fn snapshot(window: WebviewWindow, app: AppHandle) -> Result<Snapshot, String> {
    windows::ensure_manager(&window)?;
    Ok(app.state::<AppState>().snapshot())
}

/// 导入预览：目录、单文件与页面包走原生选择器，内置示例由应用直接提供。
#[tauri::command]
pub async fn import_page(
    window: WebviewWindow,
    app: AppHandle,
    mode: ImportMode,
) -> Result<Option<ImportPreview>, String> {
    windows::ensure_manager(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let dialog = app.dialog().file();
        let (path, candidates, label, files, bytes) = match mode {
            ImportMode::Directory => {
                let Some(selected) = dialog
                    .set_title("选择网页目录，入口取根目录下的 HTML")
                    .blocking_pick_folder()
                else {
                    return Ok(None);
                };
                let root = selected.into_path().map_err(|_| "无法读取所选目录路径")?;
                let candidates = import::entry_candidates(&root)?;
                if candidates.is_empty() {
                    return Err("所选目录根目录下没有 HTML 文件，请改用导入单文件".into());
                }
                // 目录里常混入与页面无关的大文件，超限时提示更可行的替代做法。
                let (files, bytes) = import::measure(&root)
                    .map_err(|error| format!("{error}；也可以改用导入单文件"))?;
                let label = file_label(&root, "网页目录");
                (root, candidates, label, files, bytes)
            }
            ImportMode::File => {
                let Some(selected) = dialog
                    .set_title("选择 HTML 文件")
                    .add_filter("网页文件", &["html", "htm"])
                    .blocking_pick_file()
                else {
                    return Ok(None);
                };
                let file = selected.into_path().map_err(|_| "无法读取所选文件路径")?;
                let name = file
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or("所选文件名无效")?
                    .to_string();
                let bytes = std::fs::metadata(&file)
                    .map_err(|_| "无法读取所选文件")?
                    .len();
                (file, vec![name.clone()], name, 1, bytes)
            }
            ImportMode::Package => {
                let Some(selected) = dialog
                    .set_title("选择 PinPage 页面包")
                    .add_filter("PinPage 页面包", &["pinpage"])
                    .blocking_pick_file()
                else {
                    return Ok(None);
                };
                let file = selected.into_path().map_err(|_| "无法读取所选文件路径")?;
                let manifest = package::read_manifest(&file)?;
                let bytes = std::fs::metadata(&file)
                    .map_err(|_| "无法读取所选文件")?
                    .len();
                let label = file_label(&file, "页面包");
                (file, vec![manifest.entry], label, manifest.files, bytes)
            }
            // 示例使用随包资源，不需要选择器，也不产生外部授权路径。
            ImportMode::Example => {
                let (files, bytes) = examples::measure();
                (
                    std::path::PathBuf::new(),
                    vec![examples::ENTRY.to_string()],
                    examples::LABEL.to_string(),
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
    .map_err(|error| error.to_string())?
}

/// 创建悬浮窗；导入来源必须来自原生选择器或内置来源签发的授权票据。
#[tauri::command]
pub async fn create_pin(
    window: WebviewWindow,
    app: AppHandle,
    input: CreatePin,
) -> Result<(), String> {
    windows::ensure_manager(&window)?;
    let state = app.state::<AppState>();
    // 站点副本以页面 ID 命名，先分配 ID 再复制，避免复制完成后还要改名。
    let id = format!("p-{}", uuid::Uuid::new_v4().simple());
    let source = match input.source {
        SourceInput::Url { address } => PageSource::Url {
            url: content::normalize_url(&address)?,
        },
        SourceInput::Import { ticket, entry } => {
            let grant = state
                .grants
                .lock()
                .unwrap()
                .get(&ticket)
                .cloned()
                .ok_or("导入授权已失效，请重新选择")?;
            match grant.mode {
                ImportMode::Directory => {
                    import::verify_entry(&grant.path, &entry, &grant.candidates)?
                }
                ImportMode::File => {
                    let selected = grant.path.file_name().and_then(|value| value.to_str());
                    if selected != Some(entry.as_str()) || !grant.path.is_file() {
                        return Err("入口与所选文件不一致，请重新选择".into());
                    }
                }
                ImportMode::Package | ImportMode::Example => {
                    if !grant.candidates.iter().any(|candidate| candidate == &entry) {
                        return Err("入口与导入来源不一致，请重新选择".into());
                    }
                }
            }
            let site = import::pages_root(&app)?.join(&id).join("site");
            let prepared = match grant.mode {
                ImportMode::Directory => {
                    import::copy_site(&grant.path, &site).map(|_| (entry.clone(), None))
                }
                ImportMode::File => {
                    import::copy_entry(&grant.path, &site).map(|entry| (entry, None))
                }
                ImportMode::Package => package::extract(&grant.path, &site)
                    .map(|extracted| (extracted.entry, extracted.storage)),
                ImportMode::Example => examples::write_site(&site).map(|entry| (entry, None)),
            };
            match prepared {
                Ok((entry, storage)) => {
                    // 随包数据只用于首次打开：包装入口后交给引导页，读取即删除。
                    if let Some(storage) = storage {
                        let payload: serde_json::Value = serde_json::from_str(&storage)
                            .map_err(|_| "页面数据格式无效".to_string())?;
                        let document = serde_json::json!({ "entry": &entry, "payload": payload });
                        let pending = import::pages_root(&app)?
                            .join(&id)
                            .join(import::PENDING_DATA);
                        if let Err(error) = std::fs::write(&pending, document.to_string()) {
                            let _ = import::remove_page(&app, &id);
                            return Err(format!("写入页面数据失败：{error}"));
                        }
                    }
                    PageSource::Bundled { entry }
                }
                Err(error) => {
                    // 复制或解包中途失败时清理半成品，避免留下无法识别的目录。
                    let _ = import::remove_page(&app, &id);
                    return Err(error);
                }
            }
        }
    };
    let name = if input.name.trim().is_empty() {
        match &source {
            PageSource::Url { url } => url::Url::parse(url)
                .ok()
                .and_then(|host| host.host_str().map(String::from))
                .unwrap_or("网页".into()),
            PageSource::Bundled { entry } => std::path::Path::new(entry)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
        }
    } else {
        input.name.trim().chars().take(80).collect()
    };
    let pin = PinConfig {
        id,
        name,
        source,
        width: 420.,
        height: 360.,
        x: None,
        y: None,
        scale: 1.,
        always_on_top: input.always_on_top,
        decorations: input.decorations,
        enabled: true,
        mode: input.mode,
        rounded: false,
        position_locked: false,
        size_locked: false,
    };
    state.settings.lock().unwrap().pins.push(pin.clone());
    state::schedule_save(&app);
    let result = windows::open_pin(&app, &pin);
    if let Err(error) = &result {
        crate::windows::show_manager(&app);
        state::report(&app, error.clone());
    }
    crate::tray::refresh(&app);
    state::notify(&app);
    result
}

/// 把导入页的站点副本与页面数据导出为页面包，取消保存时返回空。
#[tauri::command]
pub async fn export_page(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
) -> Result<Option<ExportResult>, String> {
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
        .ok_or("页面记录不存在")?;
    if !matches!(pin.source, PageSource::Bundled { .. }) {
        return Err("只有导入页面可以导出为页面包".into());
    }
    // 数据只能从运行中的页面读取，DOM 与脚本状态本就不在导出范围内。
    let page = app.get_webview_window(&id).ok_or("请先开启页面再导出")?;
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
    if let Err(error) = page.eval(data::collect_script(&id)) {
        app.state::<AppState>()
            .export_pending
            .lock()
            .unwrap()
            .remove(&id);
        return Err(format!("无法读取页面数据：{error}"));
    }

    // 先让用户选择保存位置，页面数据通常在这段时间内已经回传完成。
    let save_app = app.clone();
    let suggested = format!("{}.pinpage", safe_file_name(&pin.name));
    let target = tauri::async_runtime::spawn_blocking(move || {
        save_app
            .dialog()
            .file()
            .set_title("导出页面包")
            .set_file_name(&suggested)
            .add_filter("PinPage 页面包", &["pinpage"])
            .blocking_save_file()
    })
    .await
    .map_err(|error| error.to_string())?;
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
    .map_err(|error| error.to_string())?;
    app.state::<AppState>()
        .export_pending
        .lock()
        .unwrap()
        .remove(&id);

    let target = target.into_path().map_err(|_| "无法读取保存路径")?;
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
    .map_err(|error| error.to_string())??;
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
) -> Result<(), String> {
    if window.label() != id {
        return Err("页面身份不匹配".into());
    }
    if payload.len() > MAX_PAGE_DATA {
        return Err("页面数据过大".into());
    }
    let state = app.state::<AppState>();
    if !state.export_pending.lock().unwrap().contains(&id) {
        return Err("没有进行中的导出".into());
    }
    state.export_data.lock().unwrap().insert(id, payload);
    Ok(())
}

/// 从管理界面操作窗口，Rust 层再次校验目标 ID 与操作类型。
#[tauri::command]
pub async fn pin_action(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    action: String,
) -> Result<(), String> {
    windows::ensure_manager(&window)?;
    windows::action(&app, &id, &action)
}

/// 更新单个页面设置，关闭的页面只保存配置，运行中的页面立即应用。
#[tauri::command]
pub async fn update_pin(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    input: crate::models::PinSettingsInput,
) -> Result<(), String> {
    windows::ensure_manager(&window)?;
    windows::update(&app, &id, input)
}

/// 修改恢复设置与快捷键；注册失败则保留原设置和原快捷键。
#[tauri::command]
pub async fn update_settings(
    window: WebviewWindow,
    app: AppHandle,
    input: SettingsInput,
) -> Result<(), String> {
    windows::ensure_manager(&window)?;
    crate::shortcuts::configure(&app, &input.manager_shortcut, &input.recover_shortcut)?;
    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().unwrap();
    settings.restore_session = input.restore_session;
    settings.manager_shortcut = input.manager_shortcut;
    settings.recover_shortcut = input.recover_shortcut;
    drop(settings);
    state.save()?;
    state::notify(&app);
    Ok(())
}

/// 显式退出保留会话，与关闭单个悬浮窗的删除语义分离。
#[tauri::command]
pub async fn quit_app(window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    windows::ensure_manager(&window)?;
    windows::quit(&app);
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
