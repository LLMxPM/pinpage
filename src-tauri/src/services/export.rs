// 功能：供工作台与 CLI 共用页面包导出，在串行快照后原子发布文件并清理采集会话。
use crate::{
    automation,
    errors::AppMessage,
    models::{ExportResult, PageSource},
    package,
    state::AppState,
};
use pinpage_protocol::Failure;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Export {
    id: String,
    out: PathBuf,
    overwrite: bool,
    with_data: bool,
}

/// 无选择器入口只能由本机鉴权调用方使用，返回文件事实而不等待页面导航。
pub fn run(app: &AppHandle, input: Export, operation: &str) -> Result<Value, Failure> {
    let result = write(app, &input.id, &input.out, input.with_data, input.overwrite)
        .map_err(super::source::native)?;
    let mut value = serde_json::to_value(result).unwrap();
    value["pageId"] = json!(input.id);
    value["committed"] = json!(true);
    value["phase"] = json!("exported");
    automation::record_commit(app, operation, value.clone());
    Ok(value)
}

/// 有界等待页面采集结果；所有退出路径均清除待回传标记，关闭页不会被自动开启。
fn collect(app: &AppHandle, id: &str) -> Result<Option<String>, AppMessage> {
    let page = app
        .get_webview_window(id)
        .ok_or_else(|| AppMessage::new("errors.exportNeedsOpen"))?;
    let state = app.state::<AppState>();
    state.export_data.lock().unwrap().remove(id);
    state.export_pending.lock().unwrap().insert(id.into());
    let result = (|| {
        page.eval(crate::data::collect_script(id))
            .map_err(|e| AppMessage::new("errors.collectDataFailed").detail(e))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(data) = state.export_data.lock().unwrap().remove(id) {
                return Ok(Some(data));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    })();
    state.export_pending.lock().unwrap().remove(id);
    state.export_data.lock().unwrap().remove(id);
    result
}

/// 输出限于已存在目录中的普通 .pinpage 文件，拒绝写入应用数据目录或重解析点。
fn destination(app: &AppHandle, path: &Path, overwrite: bool) -> Result<PathBuf, AppMessage> {
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pinpage"))
    {
        return Err(AppMessage::new("errors.savePathInvalid"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| AppMessage::new("errors.savePathInvalid"))?
        .canonicalize()
        .map_err(|e| AppMessage::new("errors.savePathInvalid").detail(e))?;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| AppMessage::new("errors.savePathInvalid").detail(e))?
        .canonicalize()
        .map_err(|e| AppMessage::new("errors.savePathInvalid").detail(e))?;
    if parent.starts_with(app_data) {
        return Err(AppMessage::new("errors.savePathInvalid"));
    }
    let target = parent.join(
        path.file_name()
            .ok_or_else(|| AppMessage::new("errors.savePathInvalid"))?,
    );
    check_target(&target, overwrite)?;
    Ok(target)
}

/// 检查覆盖选项与文件类型；缺失目标允许写入，其余读取错误不能当成不存在。
fn check_target(target: &Path, overwrite: bool) -> Result<(), AppMessage> {
    match std::fs::symlink_metadata(target) {
        Ok(meta) => {
            if !overwrite {
                return Err(AppMessage::new("errors.exportExists"));
            }
            if !meta.is_file() || meta.file_type().is_symlink() {
                return Err(AppMessage::new("errors.savePathInvalid"));
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err(AppMessage::new("errors.savePathInvalid"));
                }
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppMessage::new("errors.savePathInvalid").detail(error)),
    }
}

/// 同卷发布默认拒绝覆盖；Windows 使用不带替换标记的移动，兼容不支持硬链接的移动盘。
#[cfg(windows)]
fn move_new(stage: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let source: Vec<u16> = stage.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        windows_sys::Win32::Storage::FileSystem::MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// 其他平台通过同卷硬链接原子占用新路径，随后清理暂存名称。
#[cfg(not(windows))]
fn move_new(stage: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::hard_link(stage, target)?;
    std::fs::remove_file(stage)
}

/// 写入临时同卷文件后才发布，不覆盖模式避免检查与落盘之间的覆盖竞态。
fn publish(stage: &Path, target: &Path, overwrite: bool) -> Result<(), AppMessage> {
    check_target(target, overwrite)?;
    let result = if overwrite {
        crate::state::replace_file(stage, target)
    } else {
        move_new(stage, target)
    };
    result.map_err(|e| {
        AppMessage::new(if e.kind() == std::io::ErrorKind::AlreadyExists {
            "errors.exportExists"
        } else {
            "errors.writePackageFailed"
        })
        .detail(e)
    })
}

/// 调用方运行在工作线程；操作锁保证站点、预设与默认布局属于同一次配置快照。
pub fn write(
    app: &AppHandle,
    id: &str,
    path: &Path,
    with_data: bool,
    overwrite: bool,
) -> Result<ExportResult, AppMessage> {
    let state = app.state::<AppState>();
    let _lock = state.operation.lock().unwrap();
    let pin = state
        .settings
        .lock()
        .unwrap()
        .pins
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| AppMessage::new("errors.pinNotFound"))?;
    if !matches!(pin.source, PageSource::Bundled { .. }) {
        return Err(AppMessage::new("errors.exportNeedsImport"));
    }
    let target = destination(app, path, overwrite)?;
    let payload = if with_data { collect(app, id)? } else { None };
    if payload
        .as_ref()
        .is_some_and(|value| value.len() as u64 > package::MAX_STORAGE)
    {
        return Err(AppMessage::new("errors.packageDataTooLarge"));
    }
    let stage = target.with_file_name(format!(".pinpage-export-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let files = package::write(app, &pin, payload.as_deref(), &stage)?;
        publish(&stage, &target, overwrite)?;
        Ok(ExportResult {
            path: target.to_string_lossy().into_owned(),
            files,
            with_data: payload.is_some(),
            data_bytes: payload.as_ref().map_or(0, |s| s.len() as u64),
            preset_count: pin.presets.len(),
            revision: automation::revision(app, &pin),
        })
    })();
    let _ = std::fs::remove_file(stage);
    result
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;
