// 功能：内容页文件夹能力命令入口，校验调用方身份后委托授权与只读访问。
use crate::{errors::AppMessage, folder, i18n, state, windows};
use tauri::{AppHandle, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// 内容页发起原生文件夹选择器，确认后保存逐页授权，只返回目录显示名。
/// 取消选择返回 null；授权随配置持久化，更换页面来源时由内容保存清除。
#[tauri::command]
pub async fn folder_pick(
    window: WebviewWindow,
    app: AppHandle,
) -> Result<Option<String>, AppMessage> {
    let pin = windows::ensure_content(&window, &app)?;
    let title = i18n::text(
        state::locale(&app),
        "picker.folderAccessTitle",
        &Default::default(),
    );
    let dialog_app = app.clone();
    let parent = window.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title(&title)
            .set_parent(&parent)
            .blocking_pick_folder()
    })
    .await
    .map_err(|error| AppMessage::new("errors.unknown").detail(error))?;
    let Some(selected) = picked else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| AppMessage::new("errors.folderPathInvalid").detail(error))?;
    let label = folder::save_root(&app, &pin.id, &path)?;
    Ok(Some(label))
}

/// 列举授权根目录内的相对路径；大目录在原生侧截断，页面按标记提示。
#[tauri::command]
pub async fn folder_list(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
) -> Result<folder::FolderListing, AppMessage> {
    let pin = windows::ensure_content(&window, &app)?;
    tauri::async_runtime::spawn_blocking(move || folder::list(&pin, &path))
        .await
        .map_err(|error| AppMessage::new("errors.unknown").detail(error))?
}

/// 以系统默认程序打开授权根目录内的文件；越界与缺失在校验阶段拒绝。
#[tauri::command]
pub async fn folder_open(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
) -> Result<(), AppMessage> {
    let pin = windows::ensure_content(&window, &app)?;
    let target = folder::open_target(&pin, &path)?;
    app.opener()
        .open_path(target.to_string_lossy(), None::<&str>)
        .map_err(|error| AppMessage::new("errors.folderOpenFailed").detail(error))
}

/// 在资源管理器中打开条目所在目录，不执行带参数的 shell 命令。
#[tauri::command]
pub async fn folder_reveal(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
) -> Result<(), AppMessage> {
    let pin = windows::ensure_content(&window, &app)?;
    let target = folder::reveal_target(&pin, &path)?;
    app.opener()
        .open_path(target.to_string_lossy(), None::<&str>)
        .map_err(|error| AppMessage::new("errors.folderOpenFailed").detail(error))
}
