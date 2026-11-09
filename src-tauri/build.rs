// 功能：生成 Tauri 构建资源及应用命令权限。
#[path = "build/examples.rs"]
mod examples;

/// 内嵌标准页面包索引，新增资源自动随站点目录打包，不再维护逐文件 include_bytes 清单。
fn build_examples() {
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("缺少构建输出目录"));
    println!("cargo:rerun-if-changed=../examples");
    println!("cargo:rerun-if-changed=build/examples.rs");
    let packages = examples::pack_all(
        std::path::Path::new("../examples"),
        &output.join("examples"),
    )
    .expect("示例页面包生成失败");
    let mut index = String::from(
        "// 功能：提供构建生成的内置标准页面包索引。\nconst PACKAGES: &[(&str, &[u8])] = &[\n",
    );
    for (id, path) in packages {
        index.push_str(&format!(
            "({id:?}, include_bytes!({:?})),\n",
            path.to_string_lossy()
        ));
    }
    index.push_str("];\n");
    std::fs::write(output.join("example_packages.rs"), index).expect("示例页面包索引写入失败");
}

/// 生成示例包及 Tauri 资源，测试程序沿用原生窗口所需的 manifest。
fn main() {
    build_examples();
    // Rust 测试可执行文件也需要启用 Common Controls 6，以加载窗口子类化接口。
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-tests=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "snapshot",
            "create_pin",
            "import_page",
            "export_page",
            "submit_page_data",
            "pin_action",
            "update_pin",
            "update_window",
            "update_content",
            "write_preset",
            "save_preset",
            "delete_preset",
            "update_settings",
            "updates_snapshot",
            "update_action",
            "quit_app",
        ]),
    ))
    .expect("Tauri 构建资源生成失败");
}
