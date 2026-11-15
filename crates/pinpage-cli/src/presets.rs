// 功能：读取有界模板 JSON，并将命名预设命令映射到带修订约束的协议请求。
use crate::args::{Cli, Presets};
use pinpage_protocol::Failure;
use serde_json::{json, Value};
use std::io::Read;

/// 读取模板只允许 UTF-8 JSON，兼容 PowerShell 写出的 BOM，最大 64 KiB。
fn read(path: &std::path::Path) -> Result<Value, Failure> {
    let mut bytes = vec![];
    std::fs::File::open(path)
        .and_then(|f| f.take(65537).read_to_end(&mut bytes))
        .map_err(|e| Failure::new("invalidArgument", e))?;
    if bytes.len() > 65536 {
        return Err(Failure::new("invalidArgument", "预设文件超过 64 KiB"));
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    serde_json::from_slice(bytes).map_err(|e| Failure::new("invalidArgument", e))
}

/// 写操作必须绑定实例和最新修订；查询不会要求绑定，也不会修改运行窗口。
pub fn remote(cli: &Cli, command: &Presets) -> Result<(&'static str, Value, bool), Failure> {
    let (method, target, preset_id, input, name) = match command {
        Presets::List { page_id } => {
            return Ok(("window.presets.list", json!({"id":page_id}), true))
        }
        Presets::Put {
            target,
            preset_id,
            file,
        } => (
            "window.presets.put",
            target,
            preset_id,
            Some(read(file)?),
            None,
        ),
        Presets::Save {
            target,
            preset_id,
            name,
        } => ("window.presets.save", target, preset_id, None, Some(name)),
        Presets::Apply { target, preset_id } => {
            ("window.presets.apply", target, preset_id, None, None)
        }
        Presets::Delete { target, preset_id } => {
            ("window.presets.delete", target, preset_id, None, None)
        }
    };
    if cli.expected_instance.is_none() {
        return Err(Failure::new(
            "invalidArgument",
            "预设变更必须提供 --expected-instance 与 --expected-revision",
        ));
    }
    Ok((
        method,
        json!({"id":target.page_id,"presetId":preset_id,"expectedRevision":target.expected_revision,"input":input,"name":name}),
        target.no_wait,
    ))
}
