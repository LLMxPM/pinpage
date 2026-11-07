// 功能：提供经过管理窗口身份校验的 IPC 命令入口，并协调窗口创建与状态通知。
pub mod session;
pub mod settings;

use crate::{
    content,
    errors::AppMessage,
    examples, i18n, import,
    models::{CreatePin, PageSource, PinConfig, Snapshot, SourceInput},
    package,
    state::{self, AppState},
    tray, windows,
};
use tauri::{AppHandle, Manager, WebviewWindow};

/// 读取管理快照，只向受信任的管理窗口返回路径及会话配置。
#[tauri::command]
pub async fn snapshot(window: WebviewWindow, app: AppHandle) -> Result<Snapshot, AppMessage> {
    windows::ensure_manager(&window)?;
    Ok(app.state::<AppState>().snapshot())
}

/// 创建悬浮窗；导入来源必须来自原生选择器或内置来源签发的授权票据。
#[tauri::command]
pub async fn create_pin(
    window: WebviewWindow,
    app: AppHandle,
    input: CreatePin,
) -> Result<(), AppMessage> {
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
                .ok_or_else(|| AppMessage::new("errors.grantExpired"))?;
            match grant.mode {
                crate::models::ImportMode::Directory => {
                    import::verify_entry(&grant.path, &entry, &grant.candidates)?
                }
                crate::models::ImportMode::File => {
                    let selected = grant.path.file_name().and_then(|value| value.to_str());
                    if selected != Some(entry.as_str()) || !grant.path.is_file() {
                        return Err(AppMessage::new("errors.entryMismatch"));
                    }
                }
                crate::models::ImportMode::Package | crate::models::ImportMode::Example => {
                    if !grant.candidates.iter().any(|candidate| candidate == &entry) {
                        return Err(AppMessage::new("errors.entryMismatchSource"));
                    }
                }
            }
            let site = import::pages_root(&app)?.join(&id).join("site");
            let prepared = match grant.mode {
                crate::models::ImportMode::Directory => {
                    import::copy_site(&grant.path, &site).map(|_| (entry.clone(), None))
                }
                crate::models::ImportMode::File => {
                    import::copy_entry(&grant.path, &site).map(|entry| (entry, None))
                }
                crate::models::ImportMode::Package => package::extract(&grant.path, &site)
                    .map(|extracted| (extracted.entry, extracted.storage)),
                crate::models::ImportMode::Example => {
                    examples::write_site(&site, grant.example).map(|entry| (entry, None))
                }
            };
            match prepared {
                Ok((entry, storage)) => {
                    // 随包数据只用于首次打开：包装入口后交给引导页，读取即删除。
                    if let Some(storage) = storage {
                        let payload: serde_json::Value =
                            serde_json::from_str(&storage).map_err(|error| {
                                AppMessage::new("errors.pageDataInvalid").detail(error)
                            })?;
                        let document = serde_json::json!({ "entry": &entry, "payload": payload });
                        let pending = import::pages_root(&app)?
                            .join(&id)
                            .join(import::PENDING_DATA);
                        if let Err(error) = std::fs::write(&pending, document.to_string()) {
                            let _ = import::remove_page(&app, &id);
                            return Err(AppMessage::new("errors.writePageDataFailed").detail(error));
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
    let name = match &source {
        PageSource::Url { url } => user_name(&input.name, || {
            url::Url::parse(url)
                .ok()
                .and_then(|parsed| parsed.host_str().map(String::from))
                .unwrap_or_else(|| {
                    i18n::text(state::locale(&app), "names.webPage", &Default::default())
                })
        }),
        PageSource::Bundled { entry } => user_name(&input.name, || {
            std::path::Path::new(entry)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        }),
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
    tray::refresh_or_report(&app);
    state::notify(&app);
    result
}

/// 从管理界面操作窗口，Rust 层再次校验目标 ID 与操作类型。
#[tauri::command]
pub async fn pin_action(
    window: WebviewWindow,
    app: AppHandle,
    id: String,
    action: String,
) -> Result<(), AppMessage> {
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
) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    windows::update(&app, &id, input)
}

/// 显式退出保留会话，与关闭单个悬浮窗的删除语义分离。
#[tauri::command]
pub async fn quit_app(window: WebviewWindow, app: AppHandle) -> Result<(), AppMessage> {
    windows::ensure_manager(&window)?;
    windows::quit(&app);
    Ok(())
}

/// 用户给定的名称优先，留空时使用来源推导的兜底名称。
fn user_name(input: &str, fallback: impl FnOnce() -> String) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        fallback()
    } else {
        trimmed.chars().take(80).collect()
    }
}
