// 功能：调度本地命令与已鉴权请求，保证 JSON 模式只输出一个完整对象。
mod app_start;
mod args;
mod capture;
mod client;
mod route;
use args::Cli;
use clap::Parser;
use pinpage_protocol::{Failure, Response};

/// Ctrl+C 只取消等待；应用已提交的修改保持可查询。
#[tokio::main]
async fn main() {
    let raw: Vec<_> = std::env::args_os().collect();
    let wants_json = raw.iter().any(|s| s == "--json");
    let cli = match Cli::try_parse_from(raw) {
        Ok(value) => value,
        Err(error) => {
            let code = if error.use_stderr() { 2 } else { 0 };
            if wants_json {
                output(
                    true,
                    Response::result(
                        uuid::Uuid::new_v4().to_string(),
                        None,
                        if code == 0 {
                            Ok(serde_json::json!({"help":error.to_string()}))
                        } else {
                            Err(Failure::new("invalidArgument", error))
                        },
                    ),
                );
            } else {
                let _ = error.print();
            }
            std::process::exit(code);
        }
    };
    #[cfg(windows)]
    unsafe {
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
    let result = tokio::select! {
        value = route::run(&cli) => value,
        _ = tokio::signal::ctrl_c() => Err(Failure::new("cancelled", "用户中断；已提交操作未撤销")),
    };
    let mut response = result.unwrap_or_else(|error| {
        Response::result(uuid::Uuid::new_v4().to_string(), None, Err(error))
    });
    if cli.locale == "en-US" {
        localize(&mut response);
    }
    let code = response.error.as_ref().map_or(0, Failure::exit_code);
    output(cli.json, response);
    std::process::exit(code);
}

/// 稳定机器码不翻译；英文入口提供可读概述，并在 details 保留原始原因。
fn localize(response: &mut Response) {
    if let Some(error) = &mut response.error {
        let message = match error.code.as_str() {
            "appNotRunning" => "The selected PinPage app is not running.",
            "controlDisabled" => "Local CLI control is disabled in PinPage settings.",
            "authenticationFailed" => "The local instance identity could not be verified.",
            "protocolMismatch" => "Protocol mismatch. Update PinPage.",
            "revisionConflict" => {
                "The page revision or app instance has changed. Inspect it again."
            }
            "invalidArgument" => "Invalid command arguments.",
            "invalidSource" => "Invalid page source. Check the path, entry and file limits.",
            "loadFailed" => {
                "The page navigation failed. Check committed and operationId before retrying."
            }
            "timeout" => "The wait timed out. Query the operation before retrying.",
            "captureUnavailable" => "Desktop capture is unavailable in this session.",
            "captureTargetChanged" => "The page or monitor changed during capture.",
            "outputExists" => "The output already exists. Choose another path or use --overwrite.",
            "skillNotInstalled" => "The skill is not installed at this target.",
            "skillModified" => {
                "The installed skill has local changes. Review them before replacing."
            }
            "skillTargetConflict" => {
                "The target is occupied by another version or unrecognized content."
            }
            "skillIncompatible" => {
                "The skill version is incompatible or would downgrade the installation."
            }
            "pageNotFound" => "The page does not exist.",
            "pageNotVisible" => "The page is hidden or minimized. Use --bring-to-front explicitly.",
            "cancelled" => "Interrupted. Already committed changes were not reverted.",
            _ => "The operation failed. Check the error code and details.",
        };
        let original = std::mem::replace(&mut error.message, message.into());
        let mut details = error.details.take().unwrap_or(serde_json::json!({}));
        if !details.is_object() {
            details = serde_json::json!({"cause":details});
        }
        details["reason"] = serde_json::json!(original);
        error.details = Some(details);
    }
}
/// 进度只写 stderr，stdout 保持可被 ConvertFrom-Json 一次解析。
fn output(json: bool, response: Response) {
    if json {
        println!(
            "{}",
            serde_json::to_string(&response).expect("结果可序列化")
        );
    } else if let Some(error) = response.error {
        eprintln!("{error}");
        if !response.data.is_null() {
            println!("{}", serde_json::to_string_pretty(&response.data).unwrap());
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&response.data).unwrap());
    }
}
