// 功能：统一鼠标手势的注册、实例票据与生命周期；不向内容页注入脚本或开放权限。
mod input;
mod motion;
mod mouse;
mod sequence;
mod ui;

use crate::{
    errors::AppMessage,
    models::{GestureModifier, PinConfig},
    state,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
    OnceLock, RwLock,
};
use tauri::{AppHandle, Manager, WebviewWindow};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, POINT},
    UI::WindowsAndMessaging::*,
};

const BEGIN: u32 = WM_APP + 0x510;
const FINISH: u32 = WM_APP + 0x511;
const CANCEL: u32 = WM_APP + 0x512;
static APP: OnceLock<AppHandle> = OnceLock::new();
static TARGETS: RwLock<Vec<Target>> = RwLock::new(Vec::new());
static MODIFIER: AtomicU8 = AtomicU8::new(0);
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static INSTANCE: AtomicUsize = AtomicUsize::new(0);
static NEXT_INSTANCE: AtomicUsize = AtomicUsize::new(1);
static HELD: AtomicBool = AtomicBool::new(false);
static PRIMARY: AtomicU8 = AtomicU8::new(0);
static CANCELLED: AtomicUsize = AtomicUsize::new(0);
static BROKER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct Target {
    hwnd: usize,
    id: String,
    instance: usize,
    draggable: bool,
}

/// 菜单票据始终绑定一次手势和一次窗口实例，而不是可重用的裸 HWND。
pub(crate) struct MenuRequest {
    pub id: String,
    pub hwnd: usize,
    pub x: i32,
    pub y: i32,
    instance: usize,
    token: usize,
}

impl MenuRequest {
    /// 菜单异步构造完成后必须再次校验，禁止旧菜单抢占新手势或重建后的窗口。
    pub fn valid(&self) -> bool {
        current(self.token)
            && INSTANCE.load(Ordering::SeqCst) == self.instance
            && !HELD.load(Ordering::SeqCst)
            && modifier() != GestureModifier::Disabled
            && target_for(self.instance)
                .is_some_and(|target| target.id == self.id && target.hwnd == self.hwnd)
            && !input::input_busy()
            && !input::any_button_down()
    }

    /// 最终弹出前额外要求 Alt 已松开，避免重新按下后的自动重复再次关闭菜单。
    pub fn ready(&self) -> bool {
        self.valid() && !(modifier() == GestureModifier::Alt && input::alt_held())
    }
}

/// 应用 setup 在 UI 线程建立私有消息窗口，再启动唯一的低级鼠标钩子。
pub fn start(app: &AppHandle) {
    if APP.set(app.clone()).is_err() {
        return;
    }
    set_modifier(
        app.state::<crate::state::AppState>()
            .settings
            .lock()
            .unwrap()
            .gesture_modifier,
    );
    if let Err(error) = ui::create_broker().and_then(|_| mouse::start()) {
        state::report(
            app,
            AppMessage::new("errors.gestureUnavailable").detail(error),
        );
    }
}

/// 退出时先释放 UI 捕获，再清理钩子线程；进程退出前保留消息窗口避免句柄重用。
pub fn stop() {
    ui::cancel(None, false);
    mouse::stop();
}

/// 保存成功才发布新选择，正在进行的手势仍由原始状态机收尾。
pub fn set_modifier(modifier: GestureModifier) {
    MODIFIER.store(
        match modifier {
            GestureModifier::Alt => 0,
            GestureModifier::Ctrl => 1,
            GestureModifier::Shift => 2,
            GestureModifier::Disabled => 3,
        },
        Ordering::SeqCst,
    );
}

/// 钩子只读原子值，绝不争抢应用配置或磁盘写入锁。
fn modifier() -> GestureModifier {
    match MODIFIER.load(Ordering::SeqCst) {
        0 => GestureModifier::Alt,
        1 => GestureModifier::Ctrl,
        2 => GestureModifier::Shift,
        _ => GestureModifier::Disabled,
    }
}

/// 当前票据未被取消才可以开始或完成动作；按键收尾不受取消影响。
fn current(token: usize) -> bool {
    CURRENT.load(Ordering::SeqCst) == token && CANCELLED.load(Ordering::SeqCst) != token
}

/// 在窗口所属 UI 线程安装捕获处理器并更新小型命中缓存。
pub fn register(window: &WebviewWindow, pin: &PinConfig) -> Result<(), AppMessage> {
    let hwnd = window.hwnd().map_err(crate::errors::unknown)?.0 as usize;
    let instance = TARGETS
        .read()
        .unwrap()
        .iter()
        .find(|target| target.hwnd == hwnd)
        .map(|target| target.instance)
        .unwrap_or_else(|| NEXT_INSTANCE.fetch_add(1, Ordering::SeqCst));
    ui::install(hwnd as HWND, instance)
        .map_err(|error| AppMessage::new("errors.gestureUnavailable").detail(error))?;
    // 显式设置可能改动边框、模式或位置锁，先结束该窗口正在进行的鼠标捕获。
    ui::cancel(Some(instance), false);
    let mut targets = TARGETS.write().unwrap();
    targets.retain(|target| target.hwnd != hwnd);
    targets.push(Target {
        hwnd,
        id: pin.id.clone(),
        instance,
        draggable: !pin.effective_decorations() && !pin.position_locked,
    });
    Ok(())
}

/// 只在候选鼠标按下时查询缓存，锁繁忙就放行，不让系统输入等待。
fn target_at(point: POINT, left: bool) -> Option<Target> {
    TARGETS
        .try_read()
        .ok()?
        .iter()
        .find(|target| (!left || target.draggable) && input::hit(target, point))
        .cloned()
}

/// UI 执行前按实例查询，已关闭或重建的窗口不会接收旧请求。
fn target_for(instance: usize) -> Option<Target> {
    TARGETS
        .read()
        .ok()?
        .iter()
        .find(|target| target.instance == instance)
        .cloned()
}

/// 钩子只向本应用长期持有的私有消息窗口投递，绝不向缓存的内容 HWND 补发鼠标消息。
fn post(message: u32, token: usize, point: POINT) -> bool {
    let broker = BROKER.load(Ordering::SeqCst) as HWND;
    !broker.is_null() && unsafe { PostMessageW(broker, message, token, point_param(point)) != 0 }
}

/// 打包有符号屏幕坐标，符合 Win32 鼠标消息坐标约定。
fn point_param(point: POINT) -> LPARAM {
    ((point.x as u16 as u32) | ((point.y as u16 as u32) << 16)) as LPARAM
}

/// 恢复负坐标，支持主屏左侧和上方的显示器。
fn message_point(value: LPARAM) -> POINT {
    POINT {
        x: value as i16 as i32,
        y: (value >> 16) as i16 as i32,
    }
}
