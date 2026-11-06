// 功能：校验网页来源并为隔离的本地工具页安全提供静态资源。
use crate::{models::PageSource, state::AppState};
use percent_encoding::percent_decode_str;
use std::path::{Component, Path, PathBuf};
use tauri::{
    http::{Request, Response, StatusCode},
    AppHandle, Manager,
};
use url::Url;

/// 补全常见网址，只接受 HTTP/HTTPS，禁止内嵌凭据和文件协议。
pub fn normalize_url(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() || input.chars().any(char::is_whitespace) {
        return Err("请输入有效的网址".into());
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
    let url = Url::parse(&candidate).map_err(|_| "网址格式无效")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("仅支持不含内嵌账号密码的 HTTP/HTTPS 地址".into());
    }
    Ok(url.to_string())
}

/// 验证 HTML 入口和选定根目录，保存规范化的绝对路径。
pub fn local_source(entry: &Path, root: &Path) -> Result<PageSource, String> {
    let entry = entry
        .canonicalize()
        .map_err(|_| "HTML 文件不存在或无法读取")?;
    let root = root.canonicalize().map_err(|_| "资源根目录不存在")?;
    let extension = entry.extension().and_then(|v| v.to_str()).unwrap_or("");
    if !entry.is_file() || !matches!(extension.to_lowercase().as_str(), "html" | "htm") {
        return Err("请选择 HTML 或 HTM 文件".into());
    }
    if !root.is_dir() || !entry.starts_with(&root) {
        return Err("HTML 入口必须位于资源根目录内".into());
    }
    Ok(PageSource::Local {
        entry: entry.to_string_lossy().into(),
        root: root.to_string_lossy().into(),
    })
}

/// 从编码路径解析资源，拒绝目录穿越、盘符、ADS 和越界链接。
pub fn resolve_path(root: &Path, encoded: &str) -> Result<PathBuf, String> {
    let decoded = percent_decode_str(encoded)
        .decode_utf8()
        .map_err(|_| "资源路径编码无效")?;
    if decoded.contains(['\\', ':', '\0']) {
        return Err("资源路径包含非法字符".into());
    }
    let relative = Path::new(decoded.trim_start_matches('/'));
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err("不允许读取根目录以外的资源".into());
    }
    let root = root.canonicalize().map_err(|_| "资源目录已失效")?;
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|_| "资源不存在")?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err("资源不在授权目录内".into());
    }
    Ok(path)
}

/// 为每个窗口生成独立主机名，本地页面使用独立来源与持久化存储。
pub fn source_url(id: &str, source: &PageSource) -> Result<Url, String> {
    match source {
        PageSource::Url { url } => Url::parse(&normalize_url(url)?).map_err(|e| e.to_string()),
        PageSource::Example => {
            Url::parse(&format!("pinpage://{id}/index.html")).map_err(|e| e.to_string())
        }
        PageSource::Local { entry, root } => {
            let source = local_source(Path::new(entry), Path::new(root))?;
            let PageSource::Local { entry, root } = source else {
                unreachable!()
            };
            let relative = Path::new(&entry)
                .strip_prefix(&root)
                .map_err(|_| "入口越界")?;
            let mut url = Url::parse(&format!("pinpage://{id}/")).map_err(|e| e.to_string())?;
            url.path_segments_mut()
                .map_err(|_| "无法构建本地地址")?
                .extend(relative.iter().map(|v| v.to_string_lossy()));
            Ok(url)
        }
    }
}

/// 只允许外部网络来源，或当前窗口所属的本地协议主机。
pub fn navigation_allowed(id: &str, source: &PageSource, url: &Url) -> bool {
    match source {
        PageSource::Url { .. } => {
            matches!(url.scheme(), "http" | "https")
                && !url.host_str().unwrap_or("").starts_with("pinpage.")
        }
        _ => {
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
            return Err("仅支持读取静态资源".into());
        }
        let url = Url::parse(&request.uri().to_string()).map_err(|_| "资源 URL 无效")?;
        let state = app.state::<AppState>();
        let pin = state
            .settings
            .lock()
            .unwrap()
            .pins
            .iter()
            .find(|pin| pin.id == label)
            .cloned()
            .ok_or("窗口未授权")?;
        if matches!(pin.source, PageSource::Url { .. }) || url.host_str() != Some(label) {
            return Err("资源来源未授权".into());
        }
        match &pin.source {
            PageSource::Example => example_resource(url.path()),
            PageSource::Local { root, .. } => {
                let path = resolve_path(Path::new(root), url.path())?;
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                let mime = mime_guess::from_path(&path)
                    .first_or_octet_stream()
                    .to_string();
                Ok((mime, bytes))
            }
            _ => Err("资源来源未授权".into()),
        }
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
        Err(error) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(error.into_bytes())
            .unwrap(),
    }
}

/// 内嵌计时样例不依赖安装目录，按静态资源提供真实模块与 JSON 请求。
fn example_resource(path: &str) -> Result<(String, Vec<u8>), String> {
    let (mime, bytes): (&str, &[u8]) = match path {
        "/index.html" | "/" => (
            "text/html; charset=utf-8",
            include_bytes!("../../examples/timer/index.html"),
        ),
        "/style.css" => ("text/css", include_bytes!("../../examples/timer/style.css")),
        "/timer.js" => (
            "text/javascript",
            include_bytes!("../../examples/timer/timer.js"),
        ),
        "/presets.json" => (
            "application/json",
            include_bytes!("../../examples/timer/presets.json"),
        ),
        _ => return Err("示例资源不存在".into()),
    };
    Ok((mime.into(), bytes.to_vec()))
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
