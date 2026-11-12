// 功能：在 WebView 所属 UI 线程注册与注销内部 CDP 事件，不开放远程调试端口。
use pinpage_protocol::Failure;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tauri::{AppHandle, Manager, WebviewWindow};
use webview2_com::{
    CallDevToolsProtocolMethodCompletedHandler, DevToolsProtocolEventReceivedEventHandler,
    Microsoft::Web::WebView2::Win32::*,
};
use windows::core::HSTRING;

struct Attached {
    view: ICoreWebView2,
    receivers: Vec<(ICoreWebView2DevToolsProtocolEventReceiver, i64)>,
}
impl Drop for Attached {
    /// 包括部分注册失败和应用退出，所有已取得的事件令牌都必须注销。
    fn drop(&mut self) {
        unsafe {
            for (receiver, token) in self.receivers.drain(..) {
                let _ = receiver.remove_DevToolsProtocolEventReceived(token);
            }
            for name in ["Runtime.disable", "Network.disable"] {
                let done =
                    CallDevToolsProtocolMethodCompletedHandler::create(Box::new(|_, _| Ok(())));
                let _ = self.view.CallDevToolsProtocolMethod(
                    &HSTRING::from(name),
                    &HSTRING::from("{}"),
                    &done,
                );
            }
        }
    }
}
thread_local! {static ATTACHED:RefCell<HashMap<String,Attached>>=RefCell::new(HashMap::new());}
/// 回调全部完成才返回；通道等待位于工作线程，COM 对象留在 UI 线程。
pub async fn attach(window: &WebviewWindow, id: &str) -> Result<(), Failure> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let id = id.to_owned();
    let cleanup = id.clone();
    let app = window.app_handle().clone();
    window
        .with_webview(move |platform| {
            let tx = Rc::new(RefCell::new(Some(tx)));
            let result = (|| -> windows::core::Result<()> {
                unsafe {
                    let view = platform.controller().CoreWebView2()?;
                    let mut attached = Attached {
                        view: view.clone(),
                        receivers: vec![],
                    };
                    for name in [
                        "Runtime.consoleAPICalled",
                        "Runtime.exceptionThrown",
                        "Network.loadingFailed",
                        "Network.responseReceived",
                    ] {
                        let receiver =
                            view.GetDevToolsProtocolEventReceiver(&HSTRING::from(name))?;
                        let event_app = app.clone();
                        let session = id.clone();
                        let attached_at = pinpage_protocol::now();
                        let handler = DevToolsProtocolEventReceivedEventHandler::create(Box::new(
                            move |_, args| {
                                if let Some(args) = args {
                                    let mut text = windows::core::PWSTR::null();
                                    args.ParameterObjectAsJson(&mut text)?;
                                    let text = webview2_com::CoTaskMemPWSTR::from(text);
                                    let text = text.to_string();
                                    if text.len() <= 1024 * 1024 {
                                        if let Ok(value) =
                                            serde_json::from_str::<serde_json::Value>(&text)
                                        {
                                            if name.starts_with("Runtime.")
                                                && value["timestamp"].as_f64().is_some_and(
                                                    |timestamp| timestamp < attached_at as f64,
                                                )
                                            {
                                                return Ok(());
                                            }
                                            if let Some(record) =
                                                super::normalize::event(name, &value)
                                            {
                                                super::push(&event_app, &session, record);
                                            }
                                        }
                                    }
                                }
                                Ok(())
                            },
                        ));
                        let mut token = 0;
                        receiver.add_DevToolsProtocolEventReceived(&handler, &mut token)?;
                        attached.receivers.push((receiver, token));
                    }
                    ATTACHED.with(|items| items.borrow_mut().insert(id.clone(), attached));
                    let pending = Rc::new(std::cell::Cell::new(2));
                    for name in ["Runtime.enable", "Network.enable"] {
                        let sender = tx.clone();
                        let remaining = pending.clone();
                        let done = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                            move |result, _| {
                                if let Err(error) = result {
                                    if let Some(sender) = sender.borrow_mut().take() {
                                        let _ = sender.send(Err(failed(error)));
                                    }
                                }
                                let left = remaining.get() - 1;
                                remaining.set(left);
                                if left == 0 {
                                    if let Some(sender) = sender.borrow_mut().take() {
                                        let _ = sender.send(Ok(()));
                                    }
                                }
                                Ok(())
                            },
                        ));
                        view.CallDevToolsProtocolMethod(
                            &HSTRING::from(name),
                            &HSTRING::from("{}"),
                            &done,
                        )?;
                    }
                    Ok(())
                }
            })();
            if let Err(error) = result {
                if let Some(sender) = tx.borrow_mut().take() {
                    let _ = sender.send(Err(failed(error)));
                }
            }
        })
        .map_err(failed)?;
    let result = match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => Err(failed(error)),
        Err(_) => Err(failed("诊断接入超时")),
    };
    if result.is_err() {
        detach(window.app_handle(), &cleanup);
    }
    result
}
/// 移除所有事件令牌，再禁用本会话启用的 domain，迟到回调不会再进入缓存。
pub fn detach(app: &AppHandle, id: &str) {
    let id = id.to_owned();
    let _ = app.run_on_main_thread(move || {
        ATTACHED.with(|items| {
            items.borrow_mut().remove(&id);
        });
    });
}
/// COM/HRESULT 保留原因，缺失诊断不能伪报首屏完整覆盖。
fn failed(error: impl ToString) -> Failure {
    Failure::new("diagnosticsUnavailable", error)
}
