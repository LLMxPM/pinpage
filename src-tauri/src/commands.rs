// 功能：提供经过管理窗口身份校验的 IPC 命令与文件选择入口。
use crate::{
    content,
    models::{CreatePin, LocalChoice, PageSource, PinConfig, SettingsInput, Snapshot, SourceInput},
    state::{self, AppState},
    windows,
};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

/// 读取管理快照，只向受信任的管理窗口返回路径及会话配置。
#[tauri::command]
pub async fn snapshot(window: WebviewWindow, app: AppHandle) -> Result<Snapshot, String> {
    windows::ensure_manager(&window)?;
    Ok(app.state::<AppState>().snapshot())
}

/// 通过原生选择器授权 HTML 入口，可选自定义资源根目录；取消不创建授权。
#[tauri::command]
pub async fn pick_source(
    window: WebviewWindow,
    app: AppHandle,
    custom_root: bool,
) -> Result<Option<LocalChoice>, String> {
    windows::ensure_manager(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let selected = app
            .dialog()
            .file()
            .set_title("选择本地 HTML 入口")
            .add_filter("网页文件", &["html", "htm"])
            .blocking_pick_file();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let entry = selected.into_path().map_err(|_| "无法读取所选文件路径")?;
        let parent = entry.parent().ok_or("文件目录无效")?;
        let root = if custom_root {
            let selected = app
                .dialog()
                .file()
                .set_title("选择静态网站资源根目录")
                .set_directory(parent)
                .blocking_pick_folder();
            let Some(selected) = selected else {
                return Ok(None);
            };
            selected.into_path().map_err(|_| "无法读取目录路径")?
        } else {
            parent.to_path_buf()
        };
        let source = content::local_source(&entry, &root)?;
        let ticket = uuid::Uuid::new_v4().to_string();
        let PageSource::Local { entry, root } = &source else {
            unreachable!()
        };
        let choice = LocalChoice {
            ticket: ticket.clone(),
            entry: entry.clone(),
            root: root.clone(),
        };
        let state = app.state::<AppState>();
        let mut grants = state.grants.lock().unwrap();
        grants.clear();
        grants.insert(ticket, source);
        Ok(Some(choice))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 创建悬浮窗；本地来源必须来自原生选择器签发的授权票据。
#[tauri::command]
pub async fn create_pin(
    window: WebviewWindow,
    app: AppHandle,
    input: CreatePin,
) -> Result<(), String> {
    windows::ensure_manager(&window)?;
    let state = app.state::<AppState>();
    let source = match input.source {
        SourceInput::Url { address } => PageSource::Url {
            url: content::normalize_url(&address)?,
        },
        SourceInput::Local { ticket } => state
            .grants
            .lock()
            .unwrap()
            .get(&ticket)
            .cloned()
            .ok_or("文件授权已失效，请重新选择")?,
        SourceInput::Example => PageSource::Example,
    };
    let name = if input.name.trim().is_empty() {
        match &source {
            PageSource::Url { url } => url::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(String::from))
                .unwrap_or("网页".into()),
            PageSource::Local { entry, .. } => std::path::Path::new(entry)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            PageSource::Example => "专注计时器".into(),
        }
    } else {
        input.name.trim().chars().take(80).collect()
    };
    let pin = PinConfig {
        id: format!("p-{}", uuid::Uuid::new_v4().simple()),
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
