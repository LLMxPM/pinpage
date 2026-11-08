// 功能：生成 Tauri 构建资源及应用命令权限。
fn main() {
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
