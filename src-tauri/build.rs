// 功能：生成 Tauri 构建资源及应用命令权限。
fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "snapshot",
            "create_pin",
            "pick_source",
            "pin_action",
            "update_settings",
            "quit_app",
        ]),
    ))
    .expect("Tauri 构建资源生成失败");
}
