// 功能：以 WebView2 导航 ID 绑定加载代次，忽略旧回调并区分导航与渲染进程失败。
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tauri::{AppHandle, Manager, WebviewWindow};
use webview2_com::{
    Microsoft::Web::WebView2::Win32::*, NavigationCompletedEventHandler,
    NavigationStartingEventHandler, ProcessFailedEventHandler,
};
use windows::core::{BOOL, PWSTR};
type Navigations = Rc<RefCell<HashMap<u64, String>>>;

/// 注册失败同样结束等待，不能依靠 Tauri 的无成功标志回调误报 loaded。
pub fn attach(window: &WebviewWindow) {
    let app = window.app_handle().clone();
    let id = window.label().to_owned();
    let callback_app = app.clone();
    let callback_id = id.clone();
    let result = window.with_webview(move |platform| {
        let result = unsafe { platform.controller().CoreWebView2() }
            .and_then(|view| register(view, callback_app.clone(), callback_id.clone()));
        if let Err(error) = result {
            super::load_error(&callback_app, &callback_id, failed(error));
        }
    });
    if let Err(error) = result {
        super::load_error(&app, &id, failed(error));
    }
}

/// COM 事件留在所属 UI 线程，随 WebView 销毁释放，跨线程只传应用句柄和归一化状态。
fn register(view: ICoreWebView2, app: AppHandle, id: String) -> windows::core::Result<()> {
    let navigation: Navigations = Rc::default();
    let start_app = app.clone();
    let start_id = id.clone();
    let start_map = navigation.clone();
    let starting = NavigationStartingEventHandler::create(Box::new(move |_, args| {
        if let Some(args) = args {
            let mut native_id = 0;
            let mut uri = PWSTR::null();
            unsafe {
                args.NavigationId(&mut native_id)?;
                args.Uri(&mut uri)?;
            }
            let uri = webview2_com::CoTaskMemPWSTR::from(uri).to_string();
            started(&start_app, &start_id, native_id, &uri, &start_map);
        }
        Ok(())
    }));
    let completed_app = app.clone();
    let completed_id = id.clone();
    let complete_map = navigation.clone();
    let complete = NavigationCompletedEventHandler::create(Box::new(move |sender, args| {
        if let Some(args) = args {
            let mut native_id = 0;
            let mut success = BOOL(0);
            let mut status = COREWEBVIEW2_WEB_ERROR_STATUS::default();
            unsafe {
                args.NavigationId(&mut native_id)?;
                args.IsSuccess(&mut success)?;
                args.WebErrorStatus(&mut status)?;
            }
            if let Some(sender) = sender {
                let mut uri = PWSTR::null();
                unsafe {
                    sender.Source(&mut uri)?;
                }
                let uri = webview2_com::CoTaskMemPWSTR::from(uri).to_string();
                if is_bridge(&uri, &completed_id) {
                    return Ok(());
                }
            }
            completed(
                &completed_app,
                &completed_id,
                native_id,
                success.as_bool(),
                status.0,
                &complete_map,
            );
        }
        Ok(())
    }));
    let process_failed = ProcessFailedEventHandler::create(Box::new(move |_, _| {
        super::load_error(&app, &id, failed("WebView2 内容进程异常"));
        super::diagnostics::host(&app, &id, "processFailed", "error");
        super::diagnostics::close_page(&app, &id);
        Ok(())
    }));
    let mut token = 0;
    unsafe {
        view.add_NavigationStarting(&starting, &mut token)?;
        view.add_NavigationCompleted(&complete, &mut token)?;
        view.add_ProcessFailed(&process_failed, &mut token)?;
    }
    Ok(())
}

/// 重定向沿用 native_id；宿主预留的 loadId 只由一次真实入口导航消费。
fn started(app: &AppHandle, id: &str, native_id: u64, uri: &str, navigation: &Navigations) {
    if is_bridge(uri, id) || navigation.borrow().contains_key(&native_id) {
        return;
    }
    super::navigation(app, id, true);
    let load = app
        .state::<super::AutomationState>()
        .loads
        .lock()
        .unwrap()
        .get(id)
        .cloned();
    if let Some(load) = load {
        let mut map = navigation.borrow_mut();
        if map.len() >= 16 {
            map.clear();
        }
        map.insert(native_id, load.id);
    }
}

/// 完成事件必须仍对应当前代次；首次建窗可能已发起导航，只有 pending 可接纳该回调。
fn completed(
    app: &AppHandle,
    id: &str,
    native_id: u64,
    success: bool,
    status: i32,
    navigation: &Navigations,
) {
    let current = app
        .state::<super::AutomationState>()
        .loads
        .lock()
        .unwrap()
        .get(id)
        .cloned();
    let mapped = navigation.borrow_mut().remove(&native_id);
    let Some(current) = current else {
        return;
    };
    if mapped.as_ref().is_some_and(|value| *value != current.id)
        || mapped.is_none() && !current.pending
    {
        return;
    }
    if success {
        super::navigation(app, id, false);
    } else {
        super::load_error(app, id, failed(format!("WebView2 导航失败：{status}")));
        super::diagnostics::host(app, id, "navigationFailed", "error");
    }
}

/// 仅跳过本页的真实恢复桥接路径，不把查询串或远程同名路径识别为桥接文档。
fn is_bridge(uri: &str, id: &str) -> bool {
    url::Url::parse(uri).is_ok_and(|url| {
        url.path() == crate::data::RESTORE_PAGE
            && (url.scheme() == "pinpage" && url.host_str() == Some(id)
                || url.scheme() == "http"
                    && url.host_str() == Some(format!("pinpage.{id}").as_str()))
    })
}

/// 页面加载与脚本异常使用不同错误域。
fn failed(error: impl ToString) -> pinpage_protocol::Failure {
    pinpage_protocol::Failure::new("loadFailed", error)
}
