// 功能：验证公开 skill 清单，并协调离线安装、更新和导出。
mod files;
mod install;
pub use install::{export, install, status};
use pinpage_protocol::Failure;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub const ID: &str = "pinpage-pages";
pub const VERSION: &str = "1.0.4";
pub const MANIFEST: &str = "pinpage-skill.json";
pub const RECORD: &str = ".pinpage-install.json";
pub type Bundle = BTreeMap<String, Vec<u8>>;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub skill_version: String,
    pub minimum_protocol: u32,
    #[serde(default)]
    pub minimum_cli_capabilities: BTreeMap<String, u32>,
    pub files: BTreeMap<String, String>,
}

/// 内置资源编译进入 CLI 和应用，脱离仓库也可离线使用。
pub fn builtin() -> Bundle {
    let mut bundle: Bundle = [
        (
            "SKILL.md",
            include_str!("../../../skills/pinpage-pages/SKILL.md"),
        ),
        (
            "references/cli.md",
            include_str!("../../../skills/pinpage-pages/references/cli.md"),
        ),
        (
            "references/page-development.md",
            include_str!("../../../skills/pinpage-pages/references/page-development.md"),
        ),
        (
            "references/host-api.md",
            include_str!("../../../skills/pinpage-pages/references/host-api.md"),
        ),
    ]
    .into_iter()
    .map(|(p, s)| (p.into(), s.as_bytes().to_vec()))
    .collect();
    let manifest = Manifest {
        id: ID.into(),
        skill_version: VERSION.into(),
        minimum_protocol: 1,
        minimum_cli_capabilities: [
            ("pageInstall".into(), 1),
            ("pageUpdate".into(), 1),
            ("pageDiagnostics".into(), 1),
            ("skillManagement".into(), 1),
        ]
        .into(),
        files: bundle.iter().map(|(p, b)| (p.clone(), digest(b))).collect(),
    };
    bundle.insert(
        MANIFEST.into(),
        serde_json::to_vec_pretty(&manifest).expect("内置清单可序列化"),
    );
    bundle
}
/// 摘要用于识别本地修改，不是第三方发行者的签名认证。
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
/// 外部来源只读取目录或本地 ZIP，不下载或执行任何脚本。
pub fn load(source: Option<&Path>) -> Result<Bundle, Failure> {
    let bundle = match source {
        None => builtin(),
        Some(path) if path.is_dir() => files::read_tree(path)?,
        Some(path) => files::read_zip(path)?,
    };
    validate(&bundle)?;
    Ok(bundle)
}
/// 文件集、摘要、技能标识及最低协议必须全部匹配，未知文件同样拒绝。
pub fn validate(bundle: &Bundle) -> Result<Manifest, Failure> {
    let manifest: Manifest = serde_json::from_slice(
        bundle
            .get(MANIFEST)
            .ok_or_else(|| invalid("缺少资源清单"))?,
    )
    .map_err(invalid)?;
    if manifest.id != ID || !bundle.contains_key("SKILL.md") {
        return Err(invalid("技能标识或入口不匹配"));
    }
    semver::Version::parse(&manifest.skill_version).map_err(invalid)?;
    if manifest.minimum_protocol > pinpage_protocol::VERSION {
        return Err(Failure::new(
            "skillIncompatible",
            "请更新 PinPage 后再安装该技能",
        ));
    }
    let mut capabilities = pinpage_protocol::capabilities();
    capabilities.insert("skillManagement".into(), 1);
    if manifest
        .minimum_cli_capabilities
        .iter()
        .any(|(name, version)| capabilities.get(name).is_none_or(|actual| actual < version))
    {
        return Err(Failure::new(
            "skillIncompatible",
            "本机 CLI 缺少技能所需能力",
        ));
    }
    if bundle.len() != manifest.files.len() + 1
        || manifest
            .files
            .iter()
            .any(|(p, hash)| bundle.get(p).is_none_or(|b| digest(b) != *hash))
    {
        return Err(invalid("文件集合或摘要不匹配"));
    }
    for path in bundle.keys() {
        files::safe_path(path)?;
    }
    if bundle
        .keys()
        .map(|path| path.to_lowercase())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != bundle.len()
    {
        return Err(invalid("资源含大小写冲突的路径"));
    }
    Ok(manifest)
}
/// 默认目录始终基于当前用户，不记忆上次自定义目录。
pub fn root(dir: Option<&Path>) -> Result<PathBuf, Failure> {
    let path = match dir {
        Some(path) => path.to_owned(),
        None => PathBuf::from(
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .ok_or_else(|| invalid("无法定位用户目录"))?,
        )
        .join(".agents/skills"),
    };
    std::path::absolute(path).map_err(invalid)
}
/// 非法资源属于输入错误，不把路径错误当作兼容性问题。
pub(crate) fn invalid(error: impl ToString) -> Failure {
    Failure::new("invalidArgument", error)
}

#[cfg(test)]
mod tests;
