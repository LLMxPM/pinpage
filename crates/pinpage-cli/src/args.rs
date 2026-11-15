// 功能：定义 CLI 的中英文帮助入口、严格参数和互斥来源。
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
mod presets;
pub use presets::*;

#[derive(Parser)]
#[command(version, about = "PinPage 本机页面控制 / Local page control")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true, default_value = "com.pinpage.desktop")]
    pub instance: String,
    #[arg(long, global = true)]
    pub expected_instance: Option<String>,
    #[arg(long, global = true, default_value_t = 30000, value_parser = clap::value_parser!(u64).range(1..=300000))]
    pub timeout_ms: u64,
    #[arg(long, global = true, default_value = "zh-CN", value_parser = ["zh-CN", "en-US"])]
    pub locale: String,
    #[command(subcommand)]
    pub command: Command,
}
#[derive(Subcommand)]
pub enum Command {
    /// 检查连接与能力 / Check connection and capabilities
    Doctor {
        #[arg(long)]
        capture_probe: bool,
    },
    /// 显式启动应用 / Start app explicitly
    App {
        #[command(subcommand)]
        command: App,
    },
    /// 离线管理公开技能 / Manage portable skill offline
    Skill {
        #[command(subcommand)]
        command: Skill,
    },
    /// 管理页面 / Manage pages
    Pages {
        #[command(subcommand)]
        command: Pages,
    },
    /// 局部修改窗口 / Patch window settings
    Window {
        #[command(subcommand)]
        command: Window,
    },
    /// 查询已受理操作 / Inspect accepted operations
    Operations {
        #[command(subcommand)]
        command: Operations,
    },
    /// 枚举物理显示器 / List physical monitors
    Monitors,
    /// 采集真实桌面 / Capture composed desktop
    Screenshot(Capture),
    /// 按页诊断 / Page diagnostics
    Debug {
        #[command(subcommand)]
        command: Debug,
    },
}
#[derive(Subcommand)]
pub enum App {
    Start {
        #[arg(long)]
        app_path: Option<PathBuf>,
    },
}
#[derive(Subcommand)]
pub enum Skill {
    Install(SkillTarget),
    Update(SkillTarget),
    Export {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        overwrite: bool,
    },
}
#[derive(Args)]
pub struct SkillTarget {
    #[arg(long)]
    pub dir: Option<PathBuf>,
    #[arg(long)]
    pub source: Option<PathBuf>,
    #[arg(long)]
    pub replace_modified: bool,
}
#[derive(Args)]
pub struct Source {
    #[arg(long, conflicts_with = "url", required_unless_present = "url")]
    pub source: Option<PathBuf>,
    #[arg(long, conflicts_with = "source", required_unless_present = "source")]
    pub url: Option<String>,
    #[arg(long, requires = "source")]
    pub entry: Option<String>,
}
#[derive(Subcommand)]
pub enum Pages {
    List,
    Inspect {
        id: String,
    },
    Install {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        client_key: Option<String>,
        #[arg(long)]
        closed: bool,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        apply_package_preset: bool,
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        import_package_presets: bool,
        #[arg(long)]
        no_wait: bool,
    },
    Update {
        id: String,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        entry: Option<String>,
        #[arg(long)]
        expected_revision: u64,
        #[arg(long, value_enum, default_value = "keep")]
        presets: PresetSync,
        #[arg(long)]
        no_wait: bool,
    },
    Open(Action),
    /// 导出包含命名预设的页面包 / Export a page package with presets
    Export(PageExport),
    Close(Action),
    Reload(Action),
    Focus(Action),
    Wait {
        id: String,
        #[arg(long)]
        load_id: String,
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u64).range(0..=30000))]
        settle_ms: u64,
    },
}
#[derive(Args)]
pub struct Action {
    pub id: String,
    #[arg(long)]
    pub no_wait: bool,
}
#[derive(Subcommand)]
pub enum Window {
    Set(Patch),
    /// 管理页面自己的命名预设 / Manage named page presets
    Presets {
        #[command(subcommand)]
        command: Presets,
    },
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Mode {
    Floating,
    Desktop,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum Frame {
    System,
    Borderless,
    Rounded,
}
#[derive(Args)]
pub struct Patch {
    pub id: String,
    #[arg(long)]
    pub width: Option<f64>,
    #[arg(long)]
    pub height: Option<f64>,
    #[arg(long, allow_hyphen_values = true)]
    pub x: Option<i32>,
    #[arg(long, allow_hyphen_values = true)]
    pub y: Option<i32>,
    #[arg(long, value_enum)]
    pub mode: Option<Mode>,
    #[arg(long, value_enum)]
    pub frame: Option<Frame>,
    #[arg(long, action = clap::ArgAction::Set)]
    pub transparent: Option<bool>,
    #[arg(long, action = clap::ArgAction::Set)]
    pub always_on_top: Option<bool>,
    #[arg(long, action = clap::ArgAction::Set)]
    pub position_locked: Option<bool>,
    #[arg(long, action = clap::ArgAction::Set)]
    pub size_locked: Option<bool>,
    #[arg(long)]
    pub no_wait: bool,
}
#[derive(Subcommand)]
pub enum Operations {
    Inspect { id: String },
}
#[derive(Subcommand)]
pub enum Debug {
    Start {
        id: String,
        #[arg(long)]
        reload: bool,
    },
    Logs {
        id: String,
        #[arg(long, default_value_t = 0)]
        cursor: u64,
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=500))]
        limit: u32,
        #[arg(long, value_parser = ["debug", "info", "warning", "error"])]
        level: Option<String>,
    },
    Stop {
        id: String,
    },
}
#[derive(Args)]
pub struct Capture {
    #[arg(required_unless_present = "monitor", conflicts_with = "monitor")]
    pub id: Option<String>,
    #[arg(long)]
    pub monitor: Option<String>,
    #[arg(long, default_value_t = 32, value_parser = clap::value_parser!(u32).range(0..=512))]
    pub padding: u32,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long)]
    pub metadata_out: Option<PathBuf>,
    #[arg(long)]
    pub overwrite: bool,
    #[arg(long, requires = "id")]
    pub load_id: Option<String>,
    #[arg(long, requires = "id")]
    pub bring_to_front: bool,
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(0..=30000))]
    pub settle_ms: u64,
}
