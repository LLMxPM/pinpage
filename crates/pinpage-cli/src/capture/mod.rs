// 功能：将真实桌面帧按物理区域拼接，复核目标并原子保存 PNG 及元数据。
#[cfg(windows)]
mod native;
use crate::{
    args::{Capture, Cli},
    client,
};
use pinpage_protocol::Failure;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
mod output;
use output::atomic_write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Rect {
    /// 用 i64 做差避免恶意几何引起溢出，再按像素预算限制图像。
    fn size(self) -> Result<(u32, u32), Failure> {
        let (w, h) = (
            self.right as i64 - self.left as i64,
            self.bottom as i64 - self.top as i64,
        );
        if w <= 0 || h <= 0 || w > 32768 || h > 32768 || w * h > 64 * 1024 * 1024 {
            return Err(Failure::new(
                "captureUnavailable",
                "截图超过尺寸或 64M 像素预算",
            ));
        }
        Ok((w as u32, h as u32))
    }
    /// 交集坐标统一采用虚拟桌面物理像素。
    fn intersect(self, other: Self) -> Option<Self> {
        let r = Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        (r.right > r.left && r.bottom > r.top).then_some(r)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: String,
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale: f64,
    pub primary: bool,
}
pub struct Frame {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub at: u64,
}

/// 枚举不要求应用运行，ID 只对当前拓扑有效。
pub fn monitors() -> Result<Value, Failure> {
    Ok(json!({"monitors":enumerate()?}))
}
#[cfg(windows)]
fn enumerate() -> Result<Vec<Monitor>, Failure> {
    native::enumerate()
}
#[cfg(not(windows))]
fn enumerate() -> Result<Vec<Monitor>, Failure> {
    Err(Failure::new("unsupported", "当前平台不支持桌面采集"))
}
/// 显式探针只采集一个显示器，不写图片或保留采集资源。
pub fn probe() -> Result<Value, Failure> {
    let screens = enumerate()?;
    let monitor = screens
        .first()
        .ok_or_else(|| Failure::new("captureUnavailable", "无可用显示器"))?;
    let frame = grab(monitor, 1000)?;
    Ok(
        json!({"ok":true,"backend":"desktopDuplication","width":frame.width,"height":frame.height,"sampledAt":frame.at}),
    )
}
#[cfg(windows)]
fn grab(monitor: &Monitor, timeout: u32) -> Result<Frame, Failure> {
    native::grab(monitor, timeout)
}
#[cfg(not(windows))]
fn grab(_: &Monitor, _: u32) -> Result<Frame, Failure> {
    Err(Failure::new("unsupported", "不支持桌面采集"))
}

/// 截图前后比较修订、加载和几何；变化只重试一次，不输出过期证据。
pub async fn run(cli: &Cli, options: &Capture) -> Result<Value, Failure> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(cli.timeout_ms);
    let output = std::path::absolute(&options.out).map_err(save_error)?;
    if output.exists() && !options.overwrite {
        return Err(Failure::new("outputExists", "图片已存在"));
    }
    if options
        .metadata_out
        .as_ref()
        .is_some_and(|p| std::path::absolute(p).ok().as_ref() == Some(&output))
    {
        return Err(Failure::new("invalidArgument", "图片和元数据路径必须不同"));
    }
    if let Some(id) = &options.id {
        if options.bring_to_front {
            let accepted = client::data(cli, "pages.focus", json!({"id":id})).await?;
            wait_operation(cli, &accepted).await?;
        }
        if let Some(load) = &options.load_id {
            let accepted = client::data(
                cli,
                "pages.wait",
                json!({"id":id,"loadId":load,"settleMs":0}),
            )
            .await?;
            wait_operation(cli, &accepted).await?;
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(options.settle_ms)).await;
    for attempt in 0..2 {
        let screens = enumerate()?;
        let target = if let Some(id) = &options.id {
            Some(client::data(cli, "capture.target", json!({"id":id})).await?)
        } else {
            None
        };
        if let (Some(target), Some(load)) = (&target, &options.load_id) {
            if target["loadId"] != *load {
                return Err(Failure::new("captureTargetChanged", "加载代次已变化"));
            }
        }
        let rect = if let Some(id) = &options.monitor {
            screens
                .iter()
                .find(|m| m.id == *id)
                .ok_or_else(|| Failure::new("captureUnavailable", "显示器已变化，请重新枚举"))?
                .bounds
        } else {
            let r: Rect = serde_json::from_value(target.as_ref().unwrap()["bounds"].clone())
                .map_err(save_error)?;
            let p = options.padding as i32;
            Rect {
                left: r.left.saturating_sub(p),
                top: r.top.saturating_sub(p),
                right: r.right.saturating_add(p),
                bottom: r.bottom.saturating_add(p),
            }
        };
        let (width, height) = rect.size()?;
        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        // 未连接显示器的空隙以透明像素标记，不伪造桌面内容。
        let mut records = vec![];
        let mut covered = 0u64;
        for monitor in &screens {
            let Some(area) = rect.intersect(monitor.bounds) else {
                continue;
            };
            let remaining = deadline
                .saturating_duration_since(tokio::time::Instant::now())
                .as_millis()
                .min(30000) as u32;
            if remaining == 0 {
                return Err(Failure::new("timeout", "截图超时"));
            }
            let frame = grab(monitor, remaining)?;
            for y in area.top..area.bottom {
                let sx = (area.left - monitor.bounds.left) as usize;
                let sy = (y - monitor.bounds.top) as usize;
                let dx = (area.left - rect.left) as usize;
                let dy = (y - rect.top) as usize;
                let count = (area.right - area.left) as usize * 4;
                let source = (sy * frame.width as usize + sx) * 4;
                let dest = (dy * width as usize + dx) * 4;
                rgba[dest..dest + count].copy_from_slice(&frame.rgba[source..source + count]);
            }
            covered += (area.right - area.left) as u64 * (area.bottom - area.top) as u64;
            records.push(json!({"monitor":monitor,"sampledAt":frame.at}));
        }
        if records.is_empty() {
            return Err(Failure::new("captureUnavailable", "目标不在当前桌面内"));
        }
        let after = if let Some(id) = &options.id {
            Some(client::data(cli, "capture.target", json!({"id":id})).await?)
        } else {
            None
        };
        if target != after || screens != enumerate()? {
            if attempt == 0 {
                continue;
            }
            return Err(Failure::new(
                "captureTargetChanged",
                "采集期间目标或显示器变化",
            ));
        }
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .map_err(save_error)?
                .write_image_data(&rgba)
                .map_err(save_error)?;
        }
        atomic_write(&output, &bytes, options.overwrite)?;
        let mut result = json!({"path":output,"width":width,"height":height,"capturedAt":pinpage_protocol::now(),"captureKind":"desktop","backend":"desktopDuplication","target":target,"crop":rect,"padding":if options.id.is_some(){options.padding}else{0},"monitors":records,"clipped":covered<width as u64*height as u64,"gapFill":"transparent","broughtToFront":options.bring_to_front});
        if let Some(path) = &options.metadata_out {
            match atomic_write(
                path,
                &serde_json::to_vec_pretty(&result).map_err(save_error)?,
                options.overwrite,
            ) {
                Ok(()) => {
                    result["metadataPath"] = json!(std::path::absolute(path).map_err(save_error)?)
                }
                Err(error) => return Err(error.detail(json!({"imageSaved":true,"result":result}))),
            }
        }
        return Ok(result);
    }
    unreachable!()
}
/// 组合命令等待同一个操作，不会在超时后再次创建页面。
async fn wait_operation(cli: &Cli, accepted: &Value) -> Result<(), Failure> {
    let id = accepted["operationId"]
        .as_str()
        .ok_or_else(|| Failure::new("connectionFailed", "操作 ID 缺失"))?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(cli.timeout_ms);
    loop {
        let value = client::data(cli, "operations.inspect", json!({"id":id})).await?;
        if value["done"] == true {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(Failure::new("timeout", "等待截图目标超时").detail(value));
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
/// 保存错误保留路径与系统原因，但不改变图片成功与否的事实。
fn save_error(error: impl ToString) -> Failure {
    Failure::new("captureSaveFailed", error)
}
