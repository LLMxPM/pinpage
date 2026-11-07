// 功能：维护应用状态，合并窗口变化并串行持久化配置。
use crate::{
    import::ImportGrant,
    models::{PinRuntime, Settings, Snapshot},
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager};

pub struct AppState {
    pub operation: Mutex<()>,
    pub settings: Mutex<Settings>,
    pub runtime: Mutex<HashMap<String, PinRuntime>>,
    /// 原生选择器签发的一次性导入授权，创建成功后不再复用。
    pub grants: Mutex<HashMap<String, ImportGrant>>,
    /// 正在导出的页面 ID，只有这些页面可以回传数据。
    pub export_pending: Mutex<HashSet<String>>,
    /// 已回传的页面数据，导出命令取走后立即清空。
    pub export_data: Mutex<HashMap<String, String>>,
    pub notices: Mutex<Vec<String>>,
    pub exiting: AtomicBool,
    revision: AtomicU64,
    saving: AtomicBool,
    writer: Mutex<()>,
    path: PathBuf,
}

impl AppState {
    /// 加载配置；损坏或来自更新版本的配置保留为备份，并反馈通知。
    pub fn load(path: PathBuf) -> Self {
        let mut notices = Vec::new();
        let settings = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(value) if value.schema_version == 1 => value,
                _ => {
                    let backup =
                        path.with_extension(format!("backup-{}.json", uuid::Uuid::new_v4()));
                    match std::fs::rename(&path, &backup) {
                        Ok(_) => {
                            notices.push(format!("配置无法读取，已备份至 {}", backup.display()))
                        }
                        Err(error) => notices.push(format!("配置无法读取，备份失败：{error}")),
                    }
                    Settings::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(error) => {
                notices.push(format!("读取配置失败：{error}"));
                Settings::default()
            }
        };
        Self {
            operation: Mutex::new(()),
            settings: Mutex::new(settings),
            runtime: Mutex::new(HashMap::new()),
            grants: Mutex::new(HashMap::new()),
            export_pending: Mutex::new(HashSet::new()),
            export_data: Mutex::new(HashMap::new()),
            notices: Mutex::new(notices),
            exiting: AtomicBool::new(false),
            revision: AtomicU64::new(0),
            saving: AtomicBool::new(false),
            writer: Mutex::new(()),
            path,
        }
    }

    /// 汇总配置及短期运行态，管理界面重建后可重新读取。
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            settings: self.settings.lock().unwrap().clone(),
            runtime: self.runtime.lock().unwrap().clone(),
            notices: self.notices.lock().unwrap().clone(),
        }
    }

    /// 在同目录写入临时文件后原子替换，避免半写入的配置被加载。
    pub fn save(&self) -> Result<(), String> {
        let _writer = self.writer.lock().unwrap();
        let config = self.settings.lock().unwrap().clone();
        let parent = self.path.parent().ok_or("配置目录无效")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
        let temporary = self.path.with_extension("tmp");
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
        replace_file(&temporary, &self.path).map_err(|e| format!("保存配置失败：{e}"))
    }
}

/// Windows 的 rename 不覆盖已有文件，使用系统原子替换接口。
#[cfg(windows)]
fn replace_file(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        windows_sys::Win32::Storage::FileSystem::MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Unix 的同文件系统 rename 提供原子替换。
#[cfg(not(windows))]
fn replace_file(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::rename(source, target)
}

/// 通知管理界面刷新，不向内容 WebView 广播配置或本地路径。
pub fn notify(app: &AppHandle) {
    let _ = app.emit_to("main", "pins-changed", ());
}

/// 合并 500ms 内连续的窗口事件，只有最后一个修订触发落盘。
pub fn schedule_save(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.revision.fetch_add(1, Ordering::SeqCst);
    if state.saving.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        loop {
            let revision = state.revision.load(Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(500));
            if state.revision.load(Ordering::SeqCst) != revision {
                continue;
            }
            if let Err(error) = state.save() {
                report(&app, error);
            }
            state.saving.store(false, Ordering::SeqCst);
            // 检查落盘与释放标记之间到来的事件，避免最后一个修订丢失。
            if state.revision.load(Ordering::SeqCst) != revision {
                schedule_save(&app);
            }
            break;
        }
    });
}

/// 记录可见错误，并通知管理界面；限制通知数量避免长期增长。
pub fn report(app: &AppHandle, message: String) {
    eprintln!("{message}");
    let state = app.state::<AppState>();
    let mut notices = state.notices.lock().unwrap();
    if notices.last() != Some(&message) {
        notices.push(message);
    }
    if notices.len() > 10 {
        notices.remove(0);
    }
    drop(notices);
    notify(app);
}
