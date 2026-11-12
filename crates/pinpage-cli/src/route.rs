// 功能：把明确的命令映射为有限方法表，并处理离线功能及显式启动。
use crate::{args::*, client};
use pinpage_protocol::{Failure, Response};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 将本地路径转换为绝对路径；服务端仍再次规范化和验证类型。
fn absolute(path: &Path) -> Result<PathBuf, Failure> {
    std::path::absolute(path).map_err(|e| Failure::new("invalidArgument", e))
}
/// 本地命令不连接应用；远程变更统一返回 operationId 后按需等待。
pub async fn run(cli: &Cli) -> Result<Response, Failure> {
    let local = match &cli.command {
        Command::Skill { command } => Some(match command {
            Skill::Install(t) => pinpage_skills::install(
                t.dir.as_deref(),
                t.source.as_deref(),
                false,
                t.replace_modified,
            )?,
            Skill::Update(t) => pinpage_skills::install(
                t.dir.as_deref(),
                t.source.as_deref(),
                true,
                t.replace_modified,
            )?,
            Skill::Export { out, overwrite } => pinpage_skills::export(out, *overwrite)?,
        }),
        Command::Monitors => Some(crate::capture::monitors()?),
        Command::Screenshot(options) => Some(crate::capture::run(cli, options).await?),
        Command::App {
            command: App::Start { app_path },
        } => Some(start(cli, app_path.as_deref()).await?),
        _ => None,
    };
    if let Some(data) = local {
        let instance_id = data
            .get("instanceId")
            .or_else(|| {
                data.get("target")
                    .and_then(|target| target.get("instanceId"))
            })
            .and_then(Value::as_str)
            .map(str::to_owned);
        return Ok(Response::result(
            uuid::Uuid::new_v4().to_string(),
            instance_id,
            Ok(data),
        ));
    }
    if let Command::Doctor { capture_probe } = cli.command {
        let mut response = match client::call(cli, "doctor", json!({})).await {
            Ok(response) => response,
            Err(error) => Response::result(uuid::Uuid::new_v4().to_string(), None, Err(error)),
        };
        response.data = if response.data.is_object() {
            response.data
        } else {
            json!({})
        };
        response.data["cliVersion"] = json!(env!("CARGO_PKG_VERSION"));
        response.data["connected"] = json!(response.ok);
        response.data["localCapabilities"] =
            json!({"desktopCapture":if cfg!(windows){1}else{0},"skillManagement":1});
        if response.ok {
            response.data["cliVersion"] = json!(env!("CARGO_PKG_VERSION"));
            response.data["localCapabilities"] =
                json!({"desktopCapture":if cfg!(windows) {1} else {0},"skillManagement":1});
        }
        if capture_probe {
            response.data["captureProbe"] = match crate::capture::probe() {
                Ok(v) => v,
                Err(e) => json!({"ok":false,"error":e}),
            };
        }
        return Ok(response);
    }
    let (method, params, no_wait) = remote(cli)?;
    let mut response = client::call(cli, method, params).await?;
    if !no_wait && response.ok {
        if let Some(operation) = response
            .data
            .get("operationId")
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            let deadline =
                tokio::time::Instant::now() + std::time::Duration::from_millis(cli.timeout_ms);
            loop {
                let status =
                    client::call(cli, "operations.inspect", json!({"id":operation})).await?;
                let done = status.data["done"].as_bool().unwrap_or(false);
                response = status;
                if done || !response.ok {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(Failure::new(
                        "timeout",
                        "操作等待超时；请使用 operations inspect 查询",
                    )
                    .detail(response.data));
                }
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }
        }
    }
    Ok(response)
}
/// 有限方法映射不允许传递任意 IPC 或 CDP 名称。
fn remote(cli: &Cli) -> Result<(&'static str, Value, bool), Failure> {
    let result = match &cli.command {
        Command::Pages { command } => match command {
            Pages::List => ("pages.list", json!({}), true),
            Pages::Inspect { id } => ("pages.inspect", json!({"id":id}), true),
            Pages::Install {
                source,
                name,
                client_key,
                closed,
                apply_package_preset,
                no_wait,
            } => (
                "pages.install",
                json!({"source":source.source.as_ref().map(|p|absolute(p)).transpose()?,"url":source.url,"entry":source.entry,"name":name,"clientKey":client_key,"closed":closed,"applyPackagePreset":apply_package_preset}),
                *no_wait,
            ),
            Pages::Update {
                id,
                source,
                entry,
                expected_revision,
                no_wait,
            } => {
                if cli.expected_instance.is_none() {
                    return Err(Failure::new(
                        "invalidArgument",
                        "update 必须同时提供 --expected-instance 与 --expected-revision",
                    ));
                }
                (
                    "pages.update",
                    json!({"id":id,"source":absolute(source)?,"entry":entry,"expectedRevision":expected_revision}),
                    *no_wait,
                )
            }
            Pages::Open(a) => ("pages.open", json!({"id":a.id}), a.no_wait),
            Pages::Close(a) => ("pages.close", json!({"id":a.id}), a.no_wait),
            Pages::Reload(a) => ("pages.reload", json!({"id":a.id}), a.no_wait),
            Pages::Focus(a) => ("pages.focus", json!({"id":a.id}), a.no_wait),
            Pages::Wait {
                id,
                load_id,
                settle_ms,
            } => (
                "pages.wait",
                json!({"id":id,"loadId":load_id,"settleMs":settle_ms}),
                false,
            ),
        },
        Command::Window {
            command: Window::Set(p),
        } => {
            if p.width.is_some_and(|v| !v.is_finite()) || p.height.is_some_and(|v| !v.is_finite()) {
                return Err(Failure::new("invalidArgument", "尺寸必须是有限数值"));
            }
            let mut patch = json!({});
            for (key, value) in [
                ("width", json!(p.width)),
                ("height", json!(p.height)),
                ("x", json!(p.x)),
                ("y", json!(p.y)),
                ("alwaysOnTop", json!(p.always_on_top)),
                ("transparentBackground", json!(p.transparent)),
                ("positionLocked", json!(p.position_locked)),
                ("sizeLocked", json!(p.size_locked)),
            ] {
                if !value.is_null() {
                    patch[key] = value;
                }
            }
            if let Some(mode) = p.mode {
                patch["mode"] = json!(match mode {
                    Mode::Floating => "floating",
                    Mode::Desktop => "desktop",
                });
            }
            if let Some(frame) = p.frame {
                patch["decorations"] = json!(matches!(frame, Frame::System));
                patch["rounded"] = json!(matches!(frame, Frame::Rounded));
            }
            ("window.set", json!({"id":p.id,"patch":patch}), p.no_wait)
        }
        Command::Operations {
            command: Operations::Inspect { id },
        } => ("operations.inspect", json!({"id":id}), true),
        Command::Debug { command } => match command {
            Debug::Start { id, reload } => ("debug.start", json!({"id":id,"reload":reload}), true),
            Debug::Logs {
                id,
                cursor,
                limit,
                level,
            } => (
                "debug.logs",
                json!({"id":id,"cursor":cursor,"limit":limit,"level":level}),
                true,
            ),
            Debug::Stop { id } => ("debug.stop", json!({"id":id}), true),
        },
        _ => return Err(Failure::new("invalidArgument", "命令不可路由")),
    };
    Ok(result)
}
/// 使用参数数组启动同目录应用，绝不拼接 shell 命令或隐式开启第二实例。
async fn start(cli: &Cli, path: Option<&Path>) -> Result<Value, Failure> {
    match client::data(cli, "doctor", json!({})).await {
        Ok(value) => return Ok(value),
        Err(error) if error.code == "appNotRunning" => {}
        Err(error) => return Err(error),
    }
    let path = match path {
        Some(path) => absolute(path)?,
        None if cli.instance == pinpage_protocol::DEFAULT_APP => std::env::current_exe()
            .map_err(|e| Failure::new("appNotRunning", e))?
            .with_file_name("pinpage.exe"),
        None => return Err(Failure::new("invalidArgument", "独立实例启动需 --app-path")),
    };
    crate::app_start::spawn(&path)?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(cli.timeout_ms);
    loop {
        if let Ok(data) = client::data(cli, "doctor", json!({})).await {
            return Ok(data);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(Failure::new(
                "appNotRunning",
                "启动结果与目标标识不匹配，或连接超时",
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
