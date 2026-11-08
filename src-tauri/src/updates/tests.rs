// 功能：验证更新来源边界、检查节流、状态互斥与偏好原子保存。
use super::{model::*, storage};

#[test]
/// 仅本仓库具体正式版的 HTTPS 安装附件可下载。
fn source_rejects_untrusted_urls_and_prereleases() {
    let url =
        "https://github.com/LLMxPM/pinpage/releases/download/v0.2.0/PinPage_0.2.0_x64-setup.exe";
    assert!(validate_release("0.2.0", &url.parse().unwrap()).is_ok());
    for (version, candidate) in [
        ("0.2.0", url.replace("https:", "http:")),
        ("0.2.0", url.replace("github.com", "github.com.example.org")),
        ("0.2.0", url.replace("LLMxPM/pinpage", "other/pinpage")),
        ("0.2.0", url.replace("download/v0.2.0", "latest/download")),
        ("0.2.0", url.replace("github.com/", "user@github.com/")),
        ("0.2.0", format!("{url}?redirect=evil")),
        ("0.2.0", format!("{url}#fragment")),
        ("0.2.0", url.replace("v0.2.0/", "v0.1.0/")),
        ("0.2.0", url.replace("PinPage_0.2.0", "PinPage_0.1.0")),
        (
            "0.2.0",
            url.replace("PinPage_0.2.0", "%2e%2e%2fPinPage_0.2.0"),
        ),
        ("0.2.0-beta.1", url.to_string()),
        ("0.2.0+build", url.to_string()),
        ("invalid", url.to_string()),
    ] {
        assert!(
            validate_release(version, &candidate.parse().unwrap()).is_err(),
            "{candidate}"
        );
    }
}

#[test]
/// 失败也节流到每日一次，关闭检查与未来时间戳分别遵守明确规则。
fn automatic_checks_are_throttled() {
    let mut prefs = Preferences::default();
    assert!(due(&prefs, 10));
    prefs.last_check = Some(100);
    assert!(!due(&prefs, 86_499));
    assert!(due(&prefs, 86_500));
    assert!(due(&prefs, 50));
    prefs.auto_check = false;
    assert!(!due(&prefs, 1_000_000));
}

#[test]
/// 就绪包不能被后台检查替换，任务失败或用户忽略后允许重试。
fn task_status_preserves_ready_package() {
    for status in [
        Status::Checking,
        Status::Downloading,
        Status::Verifying,
        Status::Ready,
        Status::Installing,
    ] {
        assert!(status.occupied());
    }
    for status in [
        Status::Idle,
        Status::Error,
        Status::Ignored,
        Status::Available,
        Status::UpToDate,
    ] {
        assert!(!status.occupied());
    }
}

#[test]
/// 偏好保存跨进程保留，损坏文件不会被静默重写。
fn preferences_persist_and_reject_corruption() {
    let directory = std::env::temp_dir().join(format!("pinpage-updates-{}", uuid::Uuid::new_v4()));
    let path = directory.join("updates.json");
    assert!(storage::load(&path).unwrap().auto_check);
    let preferences = Preferences {
        auto_check: false,
        last_check: Some(123),
        ignored_version: Some("0.2.0".into()),
    };
    storage::save(&path, &preferences).unwrap();
    let loaded = storage::load(&path).unwrap();
    assert!(!loaded.auto_check);
    assert_eq!(loaded.ignored_version, preferences.ignored_version);
    std::fs::write(&path, b"broken").unwrap();
    assert!(storage::load(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
    std::fs::remove_dir_all(directory).unwrap();
}
