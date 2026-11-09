// 功能：在不重载内容页的情况下切换 WebView2 背景，并更新后续导航使用的外观脚本。
use crate::{errors::AppMessage, models::PinConfig};
#[cfg(windows)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::WebviewWindow;

/// 在工作线程等待 WebView2 异步回调；UI 线程只执行 COM 调用，禁止阻塞消息循环。
#[cfg(windows)]
pub fn apply(window: &WebviewWindow, pin: &PinConfig) -> Result<(), AppMessage> {
    use crate::state::AppState;
    use tauri::Manager;
    let state = window.state::<AppState>();
    let previous = state.appearances.lock().unwrap().get(&pin.id).cloned();
    if previous.as_ref().is_some_and(|(enabled, mode, _)| {
        *enabled == pin.transparent_background && *mode == pin.mode
    }) {
        return Ok(());
    }
    // 出错后必须允许回滚重新注册，不能把已移除的脚本误认为仍然有效。
    state.appearances.lock().unwrap().remove(&pin.id);
    let (sender, receiver) = std::sync::mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let callback_cancelled = cancelled.clone();
    let enabled = pin.transparent_background;
    window
        .with_webview(move |view| {
            if callback_cancelled.load(Ordering::SeqCst) {
                return;
            }
            if let Err(error) = configure(
                view.controller(),
                previous.map(|(_, _, id)| id),
                enabled,
                sender.clone(),
                callback_cancelled,
            ) {
                let _ = sender.send(Err(error.to_string()));
            }
        })
        .map_err(failed)?;
    let result = receiver.recv_timeout(std::time::Duration::from_secs(15));
    // 超时后迟到的注册回调只能清理自身，不能覆盖随后执行的事务回滚。
    if result.is_err() {
        cancelled.store(true, Ordering::SeqCst);
    }
    let id = result.map_err(failed)?.map_err(failed)?;
    state
        .appearances
        .lock()
        .unwrap()
        .insert(pin.id.clone(), (enabled, pin.mode, id));
    Ok(())
}

/// 原生透明只使用 alpha 0/255；半透明面板由网页 CSS 绘制，保持文字不透明。
#[cfg(windows)]
fn configure(
    controller: webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller,
    previous: Option<String>,
    enabled: bool,
    sender: std::sync::mpsc::Sender<Result<String, String>>,
    cancelled: Arc<AtomicBool>,
) -> windows::core::Result<()> {
    use webview2_com::{
        AddScriptToExecuteOnDocumentCreatedCompletedHandler, ExecuteScriptCompletedHandler,
        Microsoft::Web::WebView2::Win32::{ICoreWebView2Controller2, COREWEBVIEW2_COLOR},
    };
    use windows::core::{Interface, HSTRING};
    let controller2: ICoreWebView2Controller2 = controller.cast()?;
    let script = HSTRING::from(script(enabled));
    unsafe {
        if enabled {
            // 已经是 alpha 0 时再次写 0 可能不触发重合成；显式转换确保挂载变化后刷新背景。
            controller2.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR {
                R: 0,
                G: 0,
                B: 0,
                A: 255,
            })?;
        }
        controller2.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR {
            R: if enabled { 0 } else { 255 },
            G: if enabled { 0 } else { 255 },
            B: if enabled { 0 } else { 255 },
            A: if enabled { 0 } else { 255 },
        })?;
        // 父窗口或祖先挂载关系改变后通知 WebView2，不能只依赖透明开关的值。
        controller.NotifyParentWindowPositionChanged()?;
        let webview = controller.CoreWebView2()?;
        if let Some(id) = previous {
            webview.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(id))?;
        }
        let apply_view = webview.clone();
        let apply_script = script.clone();
        let complete = AddScriptToExecuteOnDocumentCreatedCompletedHandler::create(Box::new(
            move |result, id| {
                if let Err(error) = result {
                    let _ = sender.send(Err(error.to_string()));
                    return Ok(());
                }
                if cancelled.load(Ordering::SeqCst) {
                    let _ = apply_view.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(id));
                    return Ok(());
                }
                let done_sender = sender.clone();
                let cleanup_view = apply_view.clone();
                let cleanup_id = id.clone();
                let immediate_cleanup_id = id.clone();
                let done = ExecuteScriptCompletedHandler::create(Box::new(move |result, _| {
                    if result.is_err() {
                        let _ = cleanup_view
                            .RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(cleanup_id));
                    }
                    if done_sender
                        .send(
                            result
                                .map(|_| id.clone())
                                .map_err(|error| error.to_string()),
                        )
                        .is_err()
                    {
                        let _ =
                            cleanup_view.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(id));
                    }
                    Ok(())
                }));
                if let Err(error) = apply_view.ExecuteScript(&apply_script, &done) {
                    let _ = apply_view.RemoveScriptToExecuteOnDocumentCreated(&HSTRING::from(
                        immediate_cleanup_id,
                    ));
                    let _ = sender.send(Err(error.to_string()));
                }
                Ok(())
            },
        ));
        webview.AddScriptToExecuteOnDocumentCreated(&script, &complete)?;
    }
    Ok(())
}

/// 仅为顶层文档提供无权限的 CSS 变量；新文档等待根节点出现，避免覆盖网站样式或 iframe。
#[cfg(windows)]
fn script(enabled: bool) -> String {
    format!(
        r#"(() => {{
  if (window.top !== window) return;
  const apply = () => {{
    if (!document.documentElement) return false;
    document.documentElement.style.setProperty('--pinpage-background-opacity', '{}');
    return true;
  }};
  if (!apply()) {{
    const observer = new MutationObserver(() => {{ if (apply()) observer.disconnect(); }});
    observer.observe(document, {{ childList: true }});
  }}
}})();"#,
        if enabled { 0 } else { 1 }
    )
}

/// 保留底层错误详情，并复用窗口事务的回滚与错误反馈。
#[cfg(windows)]
fn failed(error: impl std::fmt::Display) -> AppMessage {
    AppMessage::new("errors.windowApplyFailed").detail(error)
}
