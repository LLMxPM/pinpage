// 功能：把仓库中的标准示例源码打包到 dist/examples，便于独立导入和分发。
#[path = "../build/examples.rs"]
mod examples;

/// 复用应用构建使用的打包器，输出标准 .pinpage 文件，不启动桌面应用。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for (_, target) in examples::pack_all(&root.join("examples"), &root.join("dist/examples"))? {
        println!("{}", target.display());
    }
    Ok(())
}
