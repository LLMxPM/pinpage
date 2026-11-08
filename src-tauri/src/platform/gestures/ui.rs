// 功能：仅在 UI 线程捕获鼠标、移动原生窗口并释放捕获，使用实例票据隔离迟到消息。
use super::*;
use std::{cell::Cell, ptr::null_mut};

#[path = "focus.rs"]
mod focus;
#[cfg(test)]
#[path = "focus_tests.rs"]
mod tests;
use windows_sys::{
    core::w,
    Win32::{
        Foundation::{LRESULT, RECT, WPARAM},
        Graphics::Gdi::MapWindowPoints,
        UI::{Input::KeyboardAndMouse::*, Shell::*},
    },
};

const SUBCLASS_ID: usize = 0x50494e47;
#[derive(Clone, Copy)]
struct Capture {
    hwnd: HWND,
    instance: usize,
    token: usize,
    left: bool,
    origin: RECT,
    motion: motion::Motion,
}
thread_local! { static CAPTURE: Cell<Option<Capture>> = const { Cell::new(None) }; }

/// 私有消息窗口只收本应用手势请求，不显示、不授予网页访问入口，进程退出时由系统释放。
pub fn create_broker() -> Result<(), String> {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            w!("STATIC"),
            w!("PinPage gesture broker"),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            null_mut(),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if SetWindowSubclass(hwnd, Some(broker), SUBCLASS_ID, 0) == 0 {
            DestroyWindow(hwnd);
            return Err(std::io::Error::last_os_error().to_string());
        }
        BROKER.store(hwnd as usize, Ordering::SeqCst);
    }
    Ok(())
}

/// 内容窗口只安装一次独立子类，refData 随实例注册更新，不依赖 WebView 子窗口实现。
pub fn install(hwnd: HWND, instance: usize) -> Result<(), String> {
    if unsafe { SetWindowSubclass(hwnd, Some(window_proc), SUBCLASS_ID, instance) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

/// 取消指定实例的动作；先清空状态再释放捕获，避免 WM_CAPTURECHANGED 重入。
pub fn cancel(instance: Option<usize>, restore: bool) {
    let active = CAPTURE.with(Cell::get);
    if let Some(active) = active.filter(|active| instance.is_none_or(|id| active.instance == id)) {
        CAPTURE.with(|slot| slot.set(None));
        CANCELLED.store(active.token, Ordering::SeqCst);
        if restore && active.left {
            let _ = place(active.hwnd, active.origin.left, active.origin.top);
        }
        unsafe {
            if GetCapture() == active.hwnd {
                ReleaseCapture();
            }
        }
        focus::restore(active);
    }
}

/// 开始前再次确认实例、按键归属和窗口可见性；快速点击松开后不会迟到地开始拖动。
fn begin(target: &Target, token: usize, point: POINT) {
    let left = PRIMARY.load(Ordering::SeqCst) == sequence::Button::Left as u8;
    if !HELD.load(Ordering::SeqCst) || (left && !target.draggable) {
        return;
    }
    let hwnd = target.hwnd as HWND;
    let mut origin = RECT::default();
    unsafe {
        if IsWindowVisible(hwnd) == 0
            || IsWindowEnabled(hwnd) == 0
            || GetWindowRect(hwnd, &mut origin) == 0
        {
            return;
        }
        cancel(None, false);
        if !current(token)
            || !HELD.load(Ordering::SeqCst)
            || INSTANCE.load(Ordering::SeqCst) != target.instance
        {
            return;
        }
        let motion = motion::Motion::new(
            (origin.left, origin.top),
            (point.x, point.y),
            (
                GetSystemMetrics(SM_CXDRAG).max(1) as u32,
                GetSystemMetrics(SM_CYDRAG).max(1) as u32,
            ),
        );
        CAPTURE.with(|slot| {
            slot.set(Some(Capture {
                hwnd,
                instance: target.instance,
                token,
                left,
                origin,
                motion,
            }))
        });
        focus::acquire(hwnd);
        if !current(token) {
            cancel(Some(target.instance), false);
            return;
        }
        SetCapture(hwnd);
        if GetCapture() != hwnd {
            cancel(Some(target.instance), false);
        }
    }
}

/// 按屏幕物理位移移动窗口，不调用 Tauri 设置事务，因此位置锁仍由现有规则约束。
fn move_to(mut active: Capture, point: POINT) {
    if !active.left || !current(active.token) {
        return;
    }
    let Some((x, y)) = active.motion.position((point.x, point.y)) else {
        return;
    };
    CAPTURE.with(|slot| slot.set(Some(active)));
    if let Err(error) = place(active.hwnd, x, y) {
        cancel(Some(active.instance), false);
        if let Some(app) = APP.get() {
            state::report(app, error);
        }
    }
}

/// 更新位置时不改尺寸和层级；桌面子窗口先从屏幕坐标映射到当前父窗口。
fn place(hwnd: HWND, x: i32, y: i32) -> Result<(), AppMessage> {
    unsafe {
        let mut point = POINT { x, y };
        let parent = GetParent(hwnd);
        if GetWindowLongPtrW(hwnd, GWL_STYLE) & WS_CHILD as isize != 0 && !parent.is_null() {
            MapWindowPoints(null_mut(), parent, &mut point, 1);
        }
        if SetWindowPos(
            hwnd,
            null_mut(),
            point.x,
            point.y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        ) == 0
        {
            return Err(AppMessage::new("errors.setWindowPosFailed")
                .detail(std::io::Error::last_os_error()));
        }
    }
    Ok(())
}

/// 松开只结束同票据的捕获，右键菜单须在释放捕获后重新校验命中和有效性。
fn finish(token: usize, point: POINT) {
    if let Some(active) = CAPTURE
        .with(Cell::get)
        .filter(|active| active.token == token)
    {
        move_to(active, point);
        CAPTURE.with(|slot| slot.set(None));
        unsafe {
            if GetCapture() == active.hwnd {
                ReleaseCapture();
            }
        }
        focus::restore(active);
    }
    if !current(token) || PRIMARY.load(Ordering::SeqCst) != sequence::Button::Right as u8 {
        return;
    }
    let Some(target) = target_for(INSTANCE.load(Ordering::SeqCst)) else {
        return;
    };
    if !input::hit(&target, point) {
        return;
    }
    let request = MenuRequest {
        id: target.id,
        hwnd: target.hwnd,
        x: point.x,
        y: point.y,
        instance: target.instance,
        token,
    };
    let Some(app) = APP.get().filter(|_| request.valid()) else {
        return;
    };
    let app = app.clone();
    // Alt 按住时的自动重复会触发原生菜单的 VK_MENU 退出路径，松开后再展示。
    std::thread::spawn(move || {
        let mut waited = false;
        if modifier() == GestureModifier::Alt && input::alt_held() {
            waited = true;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while input::alt_held() {
                if std::time::Instant::now() >= deadline || !request.valid() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        // 先让松开前已排队的自动重复键消息在主线程处理，再开始菜单跟踪。
        if waited {
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
        if !request.valid() {
            return;
        }
        if let Err(error) = crate::window_menu::show(&app, request) {
            state::report(&app, error);
        }
    });
}

/// 所有请求只发到私有 broker；过期 BEGIN 被丢弃，FINISH 仍可释放属于旧票据的捕获。
unsafe extern "system" fn broker(
    hwnd: HWND,
    message: u32,
    token: WPARAM,
    data: LPARAM,
    _id: usize,
    _reference: usize,
) -> LRESULT {
    match message {
        BEGIN if current(token) => {
            if let Some(target) = target_for(INSTANCE.load(Ordering::SeqCst)) {
                begin(&target, token, message_point(data));
            }
            0
        }
        FINISH => {
            finish(token, message_point(data));
            0
        }
        CANCEL => {
            if CAPTURE
                .with(Cell::get)
                .is_some_and(|active| active.token == token)
            {
                cancel(None, false);
            }
            0
        }
        _ => DefSubclassProc(hwnd, message, token, data),
    }
}

/// 网页之外的宿主接收捕获期间的移动；物理移动沿系统正常路径传播，不冻结光标。
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    instance: usize,
) -> LRESULT {
    let active = CAPTURE
        .with(Cell::get)
        .filter(|active| active.instance == instance);
    match message {
        // Wry 默认把宿主焦点转给 WebView；只在手势期间延后，否则宿主收不到 Esc。
        WM_SETFOCUS if active.is_some() => return 0,
        WM_MOUSEMOVE if active.is_some() => {
            let mut point = POINT::default();
            if GetCursorPos(&mut point) != 0 {
                move_to(active.unwrap(), point);
            }
            return 0;
        }
        WM_LBUTTONUP | WM_RBUTTONUP
            if active.is_some_and(|capture| (message == WM_LBUTTONUP) == capture.left) =>
        {
            // 正常松开由钩子消费；若钩子失效，宿主收到真实松开时仍必须释放捕获。
            cancel(Some(instance), false);
            return 0;
        }
        WM_KEYDOWN | WM_SYSKEYDOWN if active.is_some() && wparam == VK_ESCAPE as usize => {
            cancel(Some(instance), true);
            return 0;
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => cancel(Some(instance), false),
        WM_NCDESTROY => {
            // 先移除实例，禁止取消流程向正在销毁的宿主恢复焦点。
            if let Ok(mut targets) = TARGETS.write() {
                targets.retain(|target| target.instance != instance);
            }
            cancel(Some(instance), false);
            RemoveWindowSubclass(hwnd, Some(window_proc), id);
        }
        _ => {}
    }
    DefSubclassProc(hwnd, message, wparam, lparam)
}
