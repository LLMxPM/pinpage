// 功能：定义命名窗口预设命令、模板同步策略和页面包导出参数。
use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Presets {
    /// 查询页面自己的预设 / List page presets
    List { page_id: String },
    /// 按稳定 ID 创建或更新，不切换窗口 / Upsert a preset without applying it
    Put {
        #[command(flatten)]
        target: PresetTarget,
        #[arg(long = "id")]
        preset_id: String,
        #[arg(long)]
        file: PathBuf,
    },
    /// 捕获当前窗口为模板，不切换窗口 / Save current window as a preset
    Save {
        #[command(flatten)]
        target: PresetTarget,
        #[arg(long = "id")]
        preset_id: String,
        #[arg(long)]
        name: String,
    },
    /// 应用命名模板 / Apply a preset
    Apply {
        #[command(flatten)]
        target: PresetTarget,
        preset_id: String,
    },
    /// 删除模板，保留当前窗口 / Delete a preset without changing the window
    Delete {
        #[command(flatten)]
        target: PresetTarget,
        preset_id: String,
    },
}

#[derive(Args)]
pub struct PresetTarget {
    pub page_id: String,
    #[arg(long)]
    pub expected_revision: u64,
    #[arg(long)]
    pub no_wait: bool,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum PresetSync {
    Keep,
    Merge,
    Replace,
}

#[derive(Args)]
pub struct PageExport {
    pub id: String,
    #[arg(long)]
    pub out: PathBuf,
    #[arg(long)]
    pub overwrite: bool,
    /// 显式包含运行页的数据 / Include storage from an open page
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    pub with_data: bool,
    #[arg(long)]
    pub no_wait: bool,
}
