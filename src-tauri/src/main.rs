// 功能：启动 PinPage 桌面应用，发布构建隐藏额外控制台。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// 进入桌面应用主循环。
fn main() {
    pinpage_lib::run();
}
