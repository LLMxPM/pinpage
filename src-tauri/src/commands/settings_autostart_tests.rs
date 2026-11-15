// 功能：验证自启注册和全局配置的事务边界，不修改真实系统启动项。
use super::*;
use std::{cell::Cell, path::PathBuf};

struct Fixture {
    path: PathBuf,
    state: AppState,
    candidate: WorkspaceSettings,
    original: Vec<u8>,
}

impl Fixture {
    /// 保存一份真实旧配置；临时目录只属于本用例，清理不触及应用会话。
    fn new() -> Self {
        let path = std::env::temp_dir()
            .join(format!("pinpage-autostart-{}", uuid::Uuid::new_v4()))
            .join("settings.json");
        let state = AppState::load(path.clone());
        state.save().unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut candidate = WorkspaceSettings::from_settings(&state.settings.lock().unwrap());
        candidate.restore_session = false;
        Self {
            path,
            state,
            candidate,
            original,
        }
    }

    /// 阻止临时文件创建，使原子写入失败，原配置文件仍然可读。
    fn block_save(&self) {
        std::fs::create_dir(self.path.with_extension("tmp")).unwrap();
    }

    /// 确认失败后内存与磁盘都保留原有会话恢复设置。
    fn assert_unchanged(&self) {
        assert!(self.state.settings.lock().unwrap().restore_session);
        assert_eq!(std::fs::read(&self.path).unwrap(), self.original);
    }
}

impl Drop for Fixture {
    /// 回收由随机 ID 创建的测试目录。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.path.parent().unwrap());
    }
}

#[test]
/// 开启、关闭和重复保存均以系统实际状态为准，不把系统注册写入可迁移配置。
fn commits_startup_and_settings_together() {
    for before in [false, true] {
        for desired in [false, true] {
            let fixture = Fixture::new();
            let system = Cell::new(before);
            let mut writes = Vec::new();
            save_transaction(
                &fixture.state,
                &fixture.candidate,
                Some(desired),
                || Ok(system.get()),
                |enabled| {
                    writes.push(enabled);
                    system.set(enabled);
                    Ok(())
                },
                |_| panic!("成功路径不应报告回滚错误"),
            )
            .unwrap();
            assert_eq!(system.get(), desired);
            assert_eq!(writes.len(), usize::from(before != desired));
            let disk: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&fixture.path).unwrap()).unwrap();
            assert_eq!(disk["restoreSession"], false);
            assert!(disk.get("autostart").is_none());
            assert!(!fixture.state.settings.lock().unwrap().restore_session);
        }
    }
}

#[test]
/// 不编辑自启时跳过所有系统访问；系统读取失败也不会阻断其他设置保存。
fn unrelated_settings_do_not_touch_startup() {
    let fixture = Fixture::new();
    save_transaction(
        &fixture.state,
        &fixture.candidate,
        None,
        || panic!("未编辑时不得读取系统注册"),
        |_| panic!("未编辑时不得写入系统注册"),
        |_| panic!("成功路径不应报告错误"),
    )
    .unwrap();
    assert!(
        !AppState::load(fixture.path.clone())
            .settings
            .lock()
            .unwrap()
            .restore_session
    );
}

#[test]
/// 无法确认当前系统状态时不写入系统，也不提交其余全局设置。
fn read_failure_preserves_existing_settings() {
    let fixture = Fixture::new();
    let error = save_transaction(
        &fixture.state,
        &fixture.candidate,
        Some(true),
        || Err(AppMessage::new("errors.autostartReadFailed")),
        |_| panic!("读取失败后不得写入"),
        |_| panic!("尚未开始事务，不需回滚"),
    )
    .unwrap_err();
    assert_eq!(error.code, "errors.autostartReadFailed");
    fixture.assert_unchanged();
}

#[test]
/// 注册可能部分成功；失败时恢复原系统开关，其他设置不提交。
fn partial_registration_failure_rolls_back_system() {
    let fixture = Fixture::new();
    let system = Cell::new(false);
    let mut writes = Vec::new();
    let error = save_transaction(
        &fixture.state,
        &fixture.candidate,
        Some(true),
        || Ok(system.get()),
        |enabled| {
            writes.push(enabled);
            system.set(enabled);
            if enabled {
                Err(AppMessage::new("errors.autostartWriteFailed"))
            } else {
                Ok(())
            }
        },
        |_| panic!("回滚成功不应报告二次错误"),
    )
    .unwrap_err();
    assert_eq!(error.code, "errors.autostartWriteFailed");
    assert_eq!(writes, [true, false]);
    assert!(!system.get());
    fixture.assert_unchanged();
}

#[test]
/// 持久化失败时，开启与关闭两个方向都恢复原系统状态和旧配置。
fn config_failure_rolls_back_both_startup_directions() {
    for before in [false, true] {
        let fixture = Fixture::new();
        fixture.block_save();
        let system = Cell::new(before);
        let mut notices = Vec::new();
        let error = save_transaction(
            &fixture.state,
            &fixture.candidate,
            Some(!before),
            || Ok(system.get()),
            |enabled| {
                system.set(enabled);
                Ok(())
            },
            |error| notices.push(error),
        )
        .unwrap_err();
        assert_eq!(error.code, "errors.saveConfigFailed");
        assert_eq!(system.get(), before);
        fixture.assert_unchanged();
        assert!(!notices
            .iter()
            .any(|error| error.code == "errors.autostartRollbackFailed"));
    }
}

#[test]
/// 系统回滚失败需另行报告，同时保留最初的保存错误及内存、磁盘原配置。
fn failed_rollback_is_reported_without_hiding_save_error() {
    let fixture = Fixture::new();
    fixture.block_save();
    let mut notices = Vec::new();
    let error = save_transaction(
        &fixture.state,
        &fixture.candidate,
        Some(true),
        || Ok(false),
        |enabled| {
            if enabled {
                Ok(())
            } else {
                Err(AppMessage::new("errors.autostartWriteFailed"))
            }
        },
        |error| notices.push(error),
    )
    .unwrap_err();
    assert_eq!(error.code, "errors.saveConfigFailed");
    assert!(notices
        .iter()
        .any(|error| error.code == "errors.autostartRollbackFailed"));
    fixture.assert_unchanged();
}

#[test]
/// 未来版本只读保护在任何系统读写之前生效。
fn read_only_config_cannot_change_system_startup() {
    let mut fixture = Fixture::new();
    fixture.state.read_only = true;
    let error = save_transaction(
        &fixture.state,
        &fixture.candidate,
        Some(true),
        || panic!("只读配置不得读取系统以开始事务"),
        |_| panic!("只读配置不得修改系统"),
        |_| panic!("尚未开始事务，不需回滚"),
    )
    .unwrap_err();
    assert_eq!(error.code, "errors.configNewer");
    fixture.assert_unchanged();
}
