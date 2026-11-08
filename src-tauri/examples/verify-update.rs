// 功能：发布前离线验证真实安装包、公钥、签名及签名绑定版本，并检查篡改会被拒绝。
use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use std::{error::Error, path::PathBuf};

/// 使用与 updater 相同的 Minisign 校验器，校验包内容及受签名保护的版本字段。
fn verify(
    bytes: &[u8],
    key: &PublicKey,
    signature: &Signature,
    version: &str,
) -> Result<(), Box<dyn Error>> {
    key.verify(bytes, signature, true)?;
    let signed_version = signature
        .trusted_comment()
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"));
    if signed_version != Some(version) {
        return Err("签名绑定版本与应用版本不一致".into());
    }
    Ok(())
}

/// 从参数读取安装包，读取项目公钥，不接触私钥或启动安装程序。
fn main() -> Result<(), Box<dyn Error>> {
    let installer = PathBuf::from(std::env::args().nth(1).ok_or("需要传入 NSIS 安装包路径")?);
    let config: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))?;
    let version = config["version"].as_str().ok_or("配置缺少版本")?;
    let encoded_key = config["plugins"]["updater"]["pubkey"]
        .as_str()
        .ok_or("配置缺少公钥")?;
    let key = PublicKey::decode(&String::from_utf8(STANDARD.decode(encoded_key)?)?)?;
    let encoded_signature = std::fs::read_to_string(format!("{}.sig", installer.display()))?;
    let signature = Signature::decode(&String::from_utf8(
        STANDARD.decode(encoded_signature.trim())?,
    )?)?;
    let mut bytes = std::fs::read(&installer)?;
    verify(&bytes, &key, &signature, version)?;
    if verify(&bytes, &key, &signature, "999.0.0").is_ok() {
        return Err("错误版本未被拒绝".into());
    }
    let first = bytes.first_mut().ok_or("安装包为空")?;
    *first ^= 1;
    if verify(&bytes, &key, &signature, version).is_ok() {
        return Err("被篡改安装包未被拒绝".into());
    }
    println!("签名、版本绑定验证通过；篡改包及错误版本均被拒绝：v{version}");
    Ok(())
}
