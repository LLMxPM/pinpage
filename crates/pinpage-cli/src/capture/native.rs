// 功能：按需使用 DXGI Desktop Duplication 读取 SDR 桌面，转换旋转和 BGRA。
use super::{Frame, Monitor, Rect};
use pinpage_protocol::Failure;
use windows::{
    core::Interface,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::*,
            Direct3D11::*,
            Dxgi::{Common::*, *},
            Gdi::*,
        },
        UI::{HiDpi::*, WindowsAndMessaging::MONITORINFOF_PRIMARY},
    },
};

/// 枚举所有适配器的桌面输出，断开的输出不作为截图目标。
fn outputs() -> Result<Vec<(IDXGIAdapter1, IDXGIOutput, DXGI_OUTPUT_DESC)>, Failure> {
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().map_err(failed)?;
        let mut found = vec![];
        let mut ai = 0;
        while let Ok(adapter) = factory.EnumAdapters1(ai) {
            let mut oi = 0;
            while let Ok(output) = adapter.EnumOutputs(oi) {
                let desc = output.GetDesc().map_err(failed)?;
                if desc.AttachedToDesktop.as_bool() {
                    found.push((adapter.clone(), output, desc));
                }
                oi += 1;
            }
            ai += 1;
        }
        if found.is_empty() {
            return Err(failed("当前登录会话没有可采集的桌面输出"));
        }
        Ok(found)
    }
}
/// 使用设备名作为当前拓扑 ID；缩放和工作区通过对应 HMONITOR 获取。
pub fn enumerate() -> Result<Vec<Monitor>, Failure> {
    outputs()?
        .into_iter()
        .map(|(_, _, d)| {
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            unsafe {
                if !GetMonitorInfoW(d.Monitor, &mut info).as_bool() {
                    return Err(failed(windows::core::Error::from_thread()));
                }
            }
            let mut x = 96;
            let mut y = 96;
            unsafe {
                let _ = GetDpiForMonitor(d.Monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y);
            }
            let len = d
                .DeviceName
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(d.DeviceName.len());
            Ok(Monitor {
                id: String::from_utf16_lossy(&d.DeviceName[..len]),
                bounds: rect(d.DesktopCoordinates),
                work_area: rect(info.rcWork),
                scale: x as f64 / 96.,
                primary: info.dwFlags & MONITORINFOF_PRIMARY != 0,
            })
        })
        .collect()
}
/// RECT 直接使用物理屏幕坐标，不用逻辑 DPI 二次换算。
fn rect(r: windows::Win32::Foundation::RECT) -> Rect {
    Rect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}
/// RAII 确保任何复制或 Map 失败都会归还帧。
struct Acquired(IDXGIOutputDuplication);
impl Drop for Acquired {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.ReleaseFrame();
        }
    }
}
/// 每次截图独立创建并释放 D3D 设备、复制对象和暂存纹理，不持续录屏。
pub fn grab(monitor: &Monitor, timeout: u32) -> Result<Frame, Failure> {
    desktop_access()?;
    unsafe {
        let (adapter, output, desc) = outputs()?
            .into_iter()
            .find(|(_, _, d)| rect(d.DesktopCoordinates) == monitor.bounds)
            .ok_or_else(|| failed("显示器拓扑已变化"))?;
        let adapter: IDXGIAdapter = adapter.cast().map_err(failed)?;
        let mut device = None;
        let mut context = None;
        D3D11CreateDevice(
            &adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
        .map_err(failed)?;
        let device = device.ok_or_else(|| failed("未创建设备"))?;
        let context = context.ok_or_else(|| failed("未创建设备上下文"))?;
        let duplication = output
            .cast::<IDXGIOutput1>()
            .map_err(failed)?
            .DuplicateOutput(&device)
            .map_err(failed)?;
        let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource = None;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout as u64);
        loop {
            let remaining = deadline
                .saturating_duration_since(std::time::Instant::now())
                .as_millis()
                .min(timeout as u128) as u32;
            if remaining == 0 {
                return Err(failed("未取得有效桌面帧"));
            }
            duplication
                .AcquireNextFrame(remaining, &mut frame_info, &mut resource)
                .map_err(failed)?;
            if frame_info.LastPresentTime != 0 {
                break;
            }
            duplication.ReleaseFrame().map_err(failed)?;
            resource = None;
        }
        let _frame = Acquired(duplication);
        let texture: ID3D11Texture2D = resource
            .ok_or_else(|| failed("未取得桌面帧"))?
            .cast()
            .map_err(failed)?;
        let mut td = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut td);
        if td.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
            return Err(failed("当前桌面格式未通过 SDR 采集支持"));
        }
        td.Usage = D3D11_USAGE_STAGING;
        td.BindFlags = 0;
        td.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        td.MiscFlags = 0;
        let mut staging = None;
        device
            .CreateTexture2D(&td, None, Some(&mut staging))
            .map_err(failed)?;
        let staging = staging.ok_or_else(|| failed("未创建暂存纹理"))?;
        context.CopyResource(&staging, &texture);
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .map_err(failed)?;
        let (width, height) = (
            (monitor.bounds.right - monitor.bounds.left) as u32,
            (monitor.bounds.bottom - monitor.bounds.top) as u32,
        );
        if width as u64 * height as u64 > 64 * 1024 * 1024 {
            context.Unmap(&staging, 0);
            return Err(failed("显示器超过采集像素预算"));
        }
        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        for y in 0..height {
            for x in 0..width {
                let (sx, sy) = if desc.Rotation == DXGI_MODE_ROTATION_ROTATE90 {
                    (y, td.Height - 1 - x)
                } else if desc.Rotation == DXGI_MODE_ROTATION_ROTATE180 {
                    (td.Width - 1 - x, td.Height - 1 - y)
                } else if desc.Rotation == DXGI_MODE_ROTATION_ROTATE270 {
                    (td.Width - 1 - y, x)
                } else {
                    (x, y)
                };
                if sx >= td.Width || sy >= td.Height {
                    context.Unmap(&staging, 0);
                    return Err(failed("旋转后的纹理尺寸不匹配"));
                }
                let p = mapped
                    .pData
                    .cast::<u8>()
                    .add(sy as usize * mapped.RowPitch as usize + sx as usize * 4);
                let i = (y as usize * width as usize + x as usize) * 4;
                rgba[i] = *p.add(2);
                rgba[i + 1] = *p.add(1);
                rgba[i + 2] = *p;
                rgba[i + 3] = 255;
            }
        }
        context.Unmap(&staging, 0);
        Ok(Frame {
            rgba,
            width,
            height,
            at: pinpage_protocol::now(),
        })
    }
}

/// 系统明确拒绝访问输入桌面时直接报错，避免把不可见会话的黑帧当作成功证据。
fn desktop_access() -> Result<(), Failure> {
    use windows_sys::Win32::System::StationsAndDesktops::*;
    unsafe {
        let mut cursor = windows::Win32::Foundation::POINT::default();
        windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut cursor)
            .map_err(|e| failed(format!("当前线程无法读取交互桌面：{e}")))?;
        let desktop = OpenInputDesktop(0, 0, DESKTOP_READOBJECTS | DESKTOP_SWITCHDESKTOP);
        if desktop.is_null() {
            return Err(failed(format!(
                "无法访问交互桌面：{}",
                std::io::Error::last_os_error()
            )));
        }
        CloseDesktop(desktop);
    }
    Ok(())
}
/// 锁屏、安全桌面、设备丢失等原始 HRESULT 保留在说明中，不以黑色像素猜测失败。
fn failed(error: impl ToString) -> Failure {
    Failure::new("captureUnavailable", error)
}
