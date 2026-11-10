// 功能：校验网页来源并为隔离的本地工具页安全提供静态资源。
use crate::{
    errors::AppMessage,
    i18n,
    models::PageSource,
    state::{self, AppState},
};
use percent_encoding::percent_decode_str;
use std::path::{Component, Path, PathBuf};
use tauri::{
    http::{Request, Response, StatusCode},
    AppHandle, Manager,
};
use url::Url;

/// 补全常见网址，只接受 HTTP/HTTPS，禁止内嵌凭据和文件协议。
pub fn normalize_url(input: &str) -> Result<String, AppMessage> {
    let input = input.trim();
    if input.is_empty() || input.chars().any(char::is_whitespace) {
        return Err(AppMessage::new("errors.invalidUrl"));
    }
    let candidate = if input.contains("://") {
        input.to_string()
    } else if input.starts_with("localhost")
        || input.starts_with("127.0.0.1")
        || input.starts_with("[::1]")
    {
        format!("http://{input}")
    } else {
        format!("https://{input}")
    };
    let url = Url::parse(&candidate).map_err(|_| AppMessage::new("errors.urlInvalid"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppMessage::new("errors.urlUnsupported"));
    }
    Ok(url.to_string())
}

/// 从编码路径解析资源，拒绝目录穿越、盘符、ADS 和越界链接。
pub fn resolve_path(root: &Path, encoded: &str) -> Result<PathBuf, AppMessage> {
    let decoded = percent_decode_str(encoded)
        .decode_utf8()
        .map_err(|_| AppMessage::new("errors.resourceEncoding"))?;
    if decoded.contains(['\\', ':', '\0']) {
        return Err(AppMessage::new("errors.resourceChars"));
    }
    let relative = Path::new(decoded.trim_start_matches('/'));
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(AppMessage::new("errors.resourceOutside"));
    }
    let root = root
        .canonicalize()
        .map_err(|_| AppMessage::new("errors.resourceRootMissing"))?;
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|_| AppMessage::new("errors.resourceNotFound"))?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err(AppMessage::new("errors.resourceNotAuthorized"));
    }
    Ok(path)
}

/// 为每个窗口生成独立主机名，导入页面使用独立来源与持久化存储。
pub fn source_url(id: &str, source: &PageSource) -> Result<Url, AppMessage> {
    match source {
        PageSource::Url { url } => Url::parse(&normalize_url(url)?)
            .map_err(|error| AppMessage::new("errors.urlInvalid").detail(error)),
        PageSource::Bundled { entry } => entry_url(id, Path::new(entry)),
    }
}

/// 把入口相对路径拼成以窗口 ID 为主机的本地地址，中文与空格由 URL 层编码。
fn entry_url(id: &str, relative: &Path) -> Result<Url, AppMessage> {
    if relative.as_os_str().is_empty()
        || relative.to_string_lossy().contains(['\\', ':', '\0'])
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err(AppMessage::new("errors.entryInvalid"));
    }
    let mut url = Url::parse(&format!("pinpage://{id}/"))
        .map_err(|error| AppMessage::new("errors.localUrlInvalid").detail(error))?;
    url.path_segments_mut()
        .map_err(|_| AppMessage::new("errors.localUrlInvalid"))?
        .extend(relative.iter().map(|value| value.to_string_lossy()));
    Ok(url)
}

/// 只允许外部网络来源，或当前窗口所属的本地协议主机。
pub fn navigation_allowed(id: &str, source: &PageSource, url: &Url) -> bool {
    match source {
        PageSource::Url { .. } => {
            matches!(url.scheme(), "http" | "https")
                && !url.host_str().unwrap_or("").starts_with("pinpage.")
        }
        PageSource::Bundled { .. } => {
            (url.scheme() == "pinpage" && url.host_str() == Some(id))
                || (url.scheme() == "http"
                    && url.host_str() == Some(format!("pinpage.{id}").as_str()))
        }
    }
}

/// 按请求所属 WebView 再次校验来源，禁止远程窗口读取本地资源。
pub fn respond(app: &AppHandle, label: &str, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let result = (|| {
        if request.method() != "GET" && request.method() != "HEAD" {
            return Err(AppMessage::new("errors.methodNotAllowed"));
        }
        let url = Url::parse(&request.uri().to_string())
            .map_err(|error| AppMessage::new("errors.resourceUrlInvalid").detail(error))?;
        let state = app.state::<AppState>();
        let pin = state
            .settings
            .lock()
            .unwrap()
            .pins
            .iter()
            .find(|pin| pin.id == label)
            .cloned()
            .ok_or_else(|| AppMessage::new("errors.windowNotAuthorized"))?;
        if !matches!(pin.source, PageSource::Bundled { .. }) || url.host_str() != Some(label) {
            return Err(AppMessage::new("errors.sourceNotAuthorized"));
        }
        // 引导页与随包数据由应用直接生成，不属于站点副本内的文件。
        match url.path() {
            crate::data::RESTORE_PAGE => {
                return Ok((
                    "text/html; charset=utf-8".into(),
                    crate::data::restore_page(state::locale(app))
                        .as_bytes()
                        .to_vec(),
                ))
            }
            crate::data::RESTORE_DATA => return crate::data::take_pending(app, label),
            _ => {}
        }
        // 导入页只读取站点副本，原始目录已不再参与加载。
        let root = crate::import::site_root(app, label)?;
        read_static(&root, url.path())
    })();
    match result {
        Ok((mime, bytes)) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime)
            .header("X-Content-Type-Options", "nosniff")
            .header("Cache-Control", "no-cache")
            .body(if request.method() == "HEAD" {
                Vec::new()
            } else {
                bytes
            })
            .unwrap(),
        Err(message) => {
            // 内容页属于不受信任来源：只返回当前语言的简短说明，诊断细节留在日志。
            eprintln!("{}", i18n::log_text(&message));
            let body = i18n::render(state::locale(app), &message).into_bytes();
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain; charset=utf-8")
                .header("X-PinPage-Error", &message.code)
                .body(body)
                .unwrap()
        }
    }
}

/// 在已授权的站点根目录内读取静态资源，路径校验与 MIME 判定只保留一处实现。
fn read_static(root: &Path, request_path: &str) -> Result<(String, Vec<u8>), AppMessage> {
    let path = resolve_path(root, request_path)?;
    let bytes = std::fs::read(&path)
        .map_err(|error| AppMessage::new("errors.resourceRead").detail(error))?;
    let mime = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();
    Ok((mime, bytes))
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
