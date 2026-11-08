// 功能：验证升级保存备份、配置冻结失败回退与未来配置版本的只读保护。
use super::*;

#[test]
/// 未来配置完全保留在原路径，保存和安装均拒绝，不降级覆盖未知字段。
fn future_config_is_never_overwritten() {
    let directory = std::env::temp_dir().join(format!("pinpage-future-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("settings.json");
    let bytes = br#"{"schemaVersion":99,"futureField":{"keep":true}}"#;
    std::fs::write(&path, bytes).unwrap();
    let state = AppState::load(path.clone());
    assert!(state.read_only);
    assert_eq!(state.save().unwrap_err().code, "errors.configNewer");
    assert!(state.prepare_update().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
/// 最终保存与备份完全一致，冻结后普通写入失败且不改变磁盘内容。
fn update_saves_and_freezes_configuration() {
    let directory = std::env::temp_dir().join(format!("pinpage-freeze-{}", uuid::Uuid::new_v4()));
    let path = directory.join("settings.json");
    let state = AppState::load(path.clone());
    *state.settings.lock().unwrap() = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1, "restoreSession": false,
        "pins": ([true, false].into_iter().enumerate().map(|(index, enabled)| serde_json::json!({
            "id": format!("p-{index}"), "name": "页面", "source": {"kind":"url", "url":"https://example.com"},
            "width":420, "height":360, "x":-20, "y":20, "scale":1.25,
            "alwaysOnTop":true, "decorations":false, "enabled":enabled
        })).collect::<Vec<_>>())
    })).unwrap();
    state.prepare_update().unwrap();
    let saved = std::fs::read(&path).unwrap();
    assert_eq!(
        std::fs::read(path.with_extension("pre-update.json")).unwrap(),
        saved
    );
    assert!(
        !serde_json::from_slice::<Settings>(&saved)
            .unwrap()
            .restore_session
    );
    let restored: Settings = serde_json::from_slice(&saved).unwrap();
    assert!(restored.pins[0].enabled);
    assert!(!restored.pins[1].enabled);
    assert_eq!(restored.pins[0].x, Some(-20));
    assert_eq!(state.save().unwrap_err().code, "errors.updateBusy");
    assert!(state.ensure_mutable().is_err());
    state.installing.store(false, Ordering::SeqCst);
    assert!(state.save().is_ok());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
/// 保存失败时解除冻结，安装调用者可保留旧进程运行和重试。
fn failed_update_save_unfreezes_configuration() {
    let directory =
        std::env::temp_dir().join(format!("pinpage-save-failure-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let obstacle = directory.join("obstacle");
    std::fs::write(&obstacle, b"file").unwrap();
    let state = AppState::load(obstacle.join("settings.json"));
    assert!(state.prepare_update().is_err());
    assert!(!state.installing.load(Ordering::SeqCst));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
/// 备份目标不可替换时中止安装并解除冻结，不能跳过备份继续升级。
fn failed_update_backup_unfreezes_configuration() {
    let directory =
        std::env::temp_dir().join(format!("pinpage-backup-failure-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(directory.join("settings.pre-update.json")).unwrap();
    let state = AppState::load(directory.join("settings.json"));
    assert_eq!(
        state.prepare_update().unwrap_err().code,
        "errors.updateBackupFailed"
    );
    assert!(!state.installing.load(Ordering::SeqCst));
    assert!(state.ensure_mutable().is_ok());
    std::fs::remove_dir_all(directory).unwrap();
}
