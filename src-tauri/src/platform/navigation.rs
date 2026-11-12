// 功能：把自定义来源转换为平台 WebView 的实际导航地址，保持建窗和更新同源。
use crate::errors::AppMessage;
/// Windows 建窗由 Wry 转为 http://协议.主机；后续 navigate 不做转换，必须显式保持一致。
pub fn runtime_url(url: url::Url) -> Result<url::Url, AppMessage> {
    #[cfg(windows)]
    if url.scheme() == "pinpage" {
        let host = url
            .host_str()
            .ok_or_else(|| AppMessage::new("errors.localUrlInvalid"))?;
        let mut target = url::Url::parse(&format!("http://pinpage.{host}{}", url.path()))
            .map_err(|e| AppMessage::new("errors.localUrlInvalid").detail(e))?;
        target.set_query(url.query());
        target.set_fragment(url.fragment());
        return Ok(target);
    }
    Ok(url)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_page_origin_and_encoded_path() {
        let result = runtime_url(
            url::Url::parse("pinpage://p-id/%E4%B8%AD%20%E6%96%87.html?q=1#h").unwrap(),
        )
        .unwrap();
        #[cfg(windows)]
        assert_eq!(
            result.as_str(),
            "http://pinpage.p-id/%E4%B8%AD%20%E6%96%87.html?q=1#h"
        );
        #[cfg(not(windows))]
        assert_eq!(result.scheme(), "pinpage");
    }
}
