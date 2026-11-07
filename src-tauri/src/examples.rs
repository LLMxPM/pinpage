// 功能：把内置示例网页写入站点副本，使示例与用户导入页走同一条加载路径。
use std::path::Path;

/// 内置示例的静态文件清单，编译进二进制，不依赖安装目录。
const FILES: &[(&str, &[u8])] = &[
    (
        "index.html",
        include_bytes!("../../examples/timer/index.html"),
    ),
    (
        "style.css",
        include_bytes!("../../examples/timer/style.css"),
    ),
    ("timer.js", include_bytes!("../../examples/timer/timer.js")),
    (
        "presets.json",
        include_bytes!("../../examples/timer/presets.json"),
    ),
];

/// 示例入口固定为根目录的 index.html。
pub const ENTRY: &str = "index.html";

/// 示例在导入预览中展示的名称。
pub const LABEL: &str = "专注计时器";

/// 导入前预览用的文件数与体积，口径与目录导出一致。
pub fn measure() -> (u64, u64) {
    let bytes = FILES.iter().map(|(_, data)| data.len() as u64).sum();
    (FILES.len() as u64, bytes)
}

/// 把示例写成站点副本，返回相对入口；示例目录与用户导入页完全同构。
pub fn write_site(target: &Path) -> Result<String, String> {
    std::fs::create_dir_all(target).map_err(|error| format!("无法创建示例目录：{error}"))?;
    for (name, data) in FILES {
        std::fs::write(target.join(name), data)
            .map_err(|error| format!("写入示例文件 {name} 失败：{error}"))?;
    }
    Ok(ENTRY.to_string())
}
