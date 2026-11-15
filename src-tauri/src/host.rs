// 功能：提供宿主信息与共享系统采样，内容页只读访问聚合数据。
use crate::{models::AppLocale, state::AppState};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::atomic::Ordering,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{Disks, Networks, System};
use tauri::{AppHandle, Manager};

/// 采样间隔；所有页面共享同一份缓存，采样成本不随页面数量增加。
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuStats {
    pub usage_percent: f32,
    pub cores: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStats {
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskStats {
    pub label: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryStats {
    pub percent: u8,
    pub charging: bool,
}

/// 一次系统采样的聚合快照；不含进程列表、主机名与 MAC 等隐私字段。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub sampled_at: u64,
    pub cpu: CpuStats,
    pub memory: MemoryStats,
    pub network: NetworkStats,
    pub disks: Vec<DiskStats>,
    pub battery: Option<BatteryStats>,
    pub uptime_secs: u64,
}

/// 页面能力表：名称到契约版本，只增不改；页面据此做特性探测与降级。
pub fn capabilities() -> BTreeMap<&'static str, u32> {
    BTreeMap::from([("folderRead", 1), ("systemStats", 1)])
}

/// 宿主信息：版本、平台、语言与能力表。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub app_version: String,
    pub platform: &'static str,
    pub locale: AppLocale,
    pub capabilities: BTreeMap<&'static str, u32>,
}

/// 汇总宿主信息；语言取 Rust 已解析的有效值，前端与页面不再猜测。
pub fn info(app: &AppHandle) -> HostInfo {
    HostInfo {
        app_version: app.package_info().version.to_string(),
        platform: if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        },
        locale: crate::state::locale(app),
        capabilities: capabilities(),
    }
}

/// 按实际经过时间把累计字节换算为每秒速率，elapsed 下限防止首帧除零放大。
fn rate(bytes: u64, elapsed: f64) -> u64 {
    (bytes as f64 / elapsed.max(0.5)) as u64
}

/// 解析 Windows 电源状态标志：128 无电池、255 电量未知均返回 None，8 表示充电中。
#[cfg(windows)]
fn battery_from_flag(flag: u8, percent: u8) -> Option<BatteryStats> {
    if flag & 128 != 0 || percent == 255 {
        return None;
    }
    Some(BatteryStats {
        percent,
        charging: flag & 8 != 0,
    })
}

/// 读取系统电池状态；台式机或查询失败返回 None。
#[cfg(windows)]
fn battery() -> Option<BatteryStats> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return None;
    }
    battery_from_flag(status.BatteryFlag, status.BatteryLifePercent)
}

#[cfg(not(windows))]
fn battery() -> Option<BatteryStats> {
    None
}

/// 启动共享采样线程；应用退出标记置位后自然结束，不阻塞退出流程。
///
/// CPU 使用率需要两次间隔采样，启动时先预热一次；网络速率用相邻两次
/// 刷新的累计字节差分。快照写入 AppState，命令只读缓存不触发采样。
pub fn start_sampler(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let mut system = System::new();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        system.refresh_cpu_usage();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        let mut last = Instant::now();
        loop {
            if app.state::<AppState>().exiting.load(Ordering::SeqCst) {
                break;
            }
            system.refresh_cpu_usage();
            system.refresh_memory();
            networks.refresh(true);
            disks.refresh(true);
            let elapsed = last.elapsed().as_secs_f64();
            last = Instant::now();
            let (rx, tx) = networks.iter().fold((0u64, 0u64), |(rx, tx), (_, data)| {
                (rx + data.received(), tx + data.transmitted())
            });
            let stats = SystemStats {
                sampled_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|value| value.as_millis() as u64)
                    .unwrap_or(0),
                cpu: CpuStats {
                    usage_percent: system.global_cpu_usage(),
                    cores: system.cpus().len(),
                },
                memory: MemoryStats {
                    total_bytes: system.total_memory(),
                    used_bytes: system.used_memory(),
                },
                network: NetworkStats {
                    rx_bytes_per_sec: rate(rx, elapsed),
                    tx_bytes_per_sec: rate(tx, elapsed),
                },
                disks: disks
                    .list()
                    .iter()
                    .map(|disk| DiskStats {
                        label: disk.mount_point().to_string_lossy().into_owned(),
                        total_bytes: disk.total_space(),
                        used_bytes: disk.total_space().saturating_sub(disk.available_space()),
                    })
                    .collect(),
                battery: battery(),
                uptime_secs: System::uptime(),
            };
            *app.state::<AppState>().stats.lock().unwrap() = Some(stats);
            std::thread::sleep(SAMPLE_INTERVAL);
        }
    });
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
