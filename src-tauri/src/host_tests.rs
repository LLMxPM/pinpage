// 功能：验证宿主能力表、速率换算与电池标志解析的纯逻辑部分。
use super::*;

#[test]
/// 能力表包含已发布能力且版本为正整数，键名与前端契约一致。
fn capabilities_are_stable() {
    let table = capabilities();
    assert_eq!(table.get("folderRead"), Some(&1));
    assert_eq!(table.get("systemStats"), Some(&1));
}

#[test]
/// 速率按经过时间换算，极短间隔不被放大到失真。
fn rate_uses_elapsed() {
    assert_eq!(rate(1000, 1.0), 1000);
    assert_eq!(rate(1000, 2.0), 500);
    assert_eq!(rate(1000, 0.01), 2000);
    assert_eq!(rate(0, 1.0), 0);
}

#[test]
/// 宿主信息序列化为驼峰键，供页面直接消费。
fn host_info_serializes_camel_case() {
    let info = HostInfo {
        app_version: "0.2.0".into(),
        platform: "windows",
        locale: AppLocale::ZhCn,
        capabilities: capabilities(),
    };
    let value = serde_json::to_value(&info).unwrap();
    assert!(value.get("appVersion").is_some());
    assert!(value.get("capabilities").is_some());
}

#[cfg(windows)]
#[test]
/// 无电池、电量未知与充电状态按标志位正确解析。
fn battery_flag_parsing() {
    assert!(battery_from_flag(128, 0).is_none());
    assert!(battery_from_flag(0, 255).is_none());
    let charging = battery_from_flag(8, 60).unwrap();
    assert_eq!(charging.percent, 60);
    assert!(charging.charging);
    let discharging = battery_from_flag(0, 45).unwrap();
    assert!(!discharging.charging);
}
