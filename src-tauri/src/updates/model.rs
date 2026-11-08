// 功能：定义升级状态、正式版来源校验与自动检查节流规则。
use crate::errors::AppMessage;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

pub const REPOSITORY: &str = "LLMxPM/pinpage";
pub const ENDPOINT: &str = "https://github.com/LLMxPM/pinpage/releases/latest/download/latest.json";
pub const RELEASES: &str = "https://github.com/LLMxPM/pinpage/releases";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Verifying,
    Ready,
    Installing,
    Ignored,
    Error,
}

impl Status {
    /// 占用升级任务时拒绝重复请求，安装包就绪后也不允许后台检查替换它。
    pub fn occupied(self) -> bool {
        matches!(
            self,
            Self::Checking | Self::Downloading | Self::Verifying | Self::Ready | Self::Installing
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub auto_check: bool,
    pub last_check: Option<u64>,
    pub ignored_version: Option<String>,
}

impl Default for Preferences {
    /// 默认每日检查一次；不自动下载或安装。
    fn default() -> Self {
        Self {
            auto_check: true,
            last_check: None,
            ignored_version: None,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub current_version: String,
    pub supported: bool,
    pub status: Status,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub error: Option<AppMessage>,
    pub preferences: Preferences,
}

/// 返回 Unix 秒数；系统时钟异常时回退零，节流规则仍可安全工作。
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 未来时间戳不永久阻止检查；失败也记入间隔，避免断网时持续请求。
pub fn due(preferences: &Preferences, time: u64) -> bool {
    preferences.auto_check
        && preferences
            .last_check
            .is_none_or(|last| time < last || time - last >= 86_400)
}

/// 安装地址必须来自本仓库具体正式版的 Release 附件，不接受外站、凭据或可变 latest 地址。
pub fn validate_release(version: &str, url: &Url) -> Result<(), AppMessage> {
    let parsed = semver::Version::parse(version)
        .map_err(|_| AppMessage::new("errors.updateSourceInvalid"))?;
    let prefix = format!("/{REPOSITORY}/releases/download/v{version}/");
    let asset = url.path().strip_prefix(&prefix).unwrap_or_default();
    if !parsed.pre.is_empty()
        || !parsed.build.is_empty()
        || url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || asset != format!("PinPage_{version}_x64-setup.exe")
    {
        return Err(AppMessage::new("errors.updateSourceInvalid"));
    }
    Ok(())
}
