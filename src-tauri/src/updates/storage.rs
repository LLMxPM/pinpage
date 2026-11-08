// 功能：持久化升级偏好与配置备份，不与页面配置混用版本或写入锁。
use super::model::Preferences;
use crate::{errors::AppMessage, state};
use std::path::Path;

/// 旧安装缺少升级偏好时使用默认值；无效文件保留原样并报告错误。
pub fn load(path: &Path) -> Result<Preferences, AppMessage> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| AppMessage::new("errors.updatePreferencesFailed").detail(error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::default()),
        Err(error) => Err(AppMessage::new("errors.updatePreferencesFailed").detail(error)),
    }
}

/// 同目录临时写入并原子替换，调用者持有升级状态锁以保证顺序。
pub fn save(path: &Path, preferences: &Preferences) -> Result<(), AppMessage> {
    let write = || -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(path.parent().ok_or("升级偏好目录无效")?)?;
        let temporary = path.with_extension("tmp");
        let bytes = serde_json::to_vec_pretty(preferences)?;
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        state::replace_file(&temporary, path)?;
        Ok(())
    };
    write().map_err(|error| AppMessage::new("errors.updatePreferencesFailed").detail(error))
}
