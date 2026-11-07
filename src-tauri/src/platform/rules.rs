// 功能：拦截本应用窗口的原生消息，实施位置锁、尺寸锁和动态圆角。
use super::{corners, desktop};
use std::cell::Cell;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    UI::{Shell::*, WindowsAndMessaging::*},
};

pub struct Rules {
    pub configuring: Cell<bool>,
    pub position_locked: Cell<bool>,
    pub size_locked: Cell<bool>,
    pub rounded: Cell<bool>,
    pub decorated: Cell<bool>,
    pub scale: Cell<f64>,
    pub anchor: Cell<RECT>,
    rounding: Cell<bool>,
}
const SUBCLASS_ID: usize = 0x50494e;

/// 仅在窗口所属 UI 线程安装消息处理器，窗口销毁时释放自身数据。
pub unsafe fn install(window: HWND) -> Result<*const Rules, String> {
    let mut data = 0usize;
    if GetWindowSubclass(window, Some(procedure), SUBCLASS_ID, &mut data) != 0 {
        return Ok(data as *const Rules);
    }
    let rules = Box::new(Rules {
        configuring: Cell::new(true),
        position_locked: Cell::new(false),
        size_locked: Cell::new(false),
        rounded: Cell::new(false),
        decorated: Cell::new(true),
        scale: Cell::new(1.),
        anchor: Cell::new(desktop::parent_rect(window)),
        rounding: Cell::new(false),
    });
    let data = Box::into_raw(rules);
    if SetWindowSubclass(window, Some(procedure), SUBCLASS_ID, data as usize) == 0 {
        drop(Box::from_raw(data));
        return Err("无法设置窗口锁定规则".into());
    }
    Ok(data)
}

/// 原生移动、拖动缩放、系统菜单和吸附均在同一处约束，管理设置期间临时放行。
unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    let rules = &*(data as *const Rules);
    if message == WM_DPICHANGED {
        let dpi = wparam as u32 & 0xffff;
        if dpi > 0 {
            rules.scale.set(dpi as f64 / 96.);
        }
    }
    if message == WM_NCDESTROY {
        RemoveWindowSubclass(window, Some(procedure), id);
        let result = DefSubclassProc(window, message, wparam, lparam);
        drop(Box::from_raw(data as *mut Rules));
        return result;
    }
    if !rules.configuring.get() {
        if message == WM_SYSCOMMAND {
            let command = wparam as u32 & 0xfff0;
            if (command == SC_MOVE && rules.position_locked.get())
                || (command == SC_SIZE && rules.size_locked.get())
                || (command == SC_MAXIMIZE
                    && (rules.position_locked.get() || rules.size_locked.get()))
            {
                return 0;
            }
        }
        if message == WM_WINDOWPOSCHANGING && lparam != 0 && IsIconic(window) == 0 {
            let position = &mut *(lparam as *mut WINDOWPOS);
            let rect = rules.anchor.get();
            if rules.position_locked.get() && position.flags & SWP_NOMOVE == 0 {
                position.x = rect.left;
                position.y = rect.top;
            }
            if rules.size_locked.get() && position.flags & SWP_NOSIZE == 0 {
                position.cx = rect.right - rect.left;
                position.cy = rect.bottom - rect.top;
            }
        }
    }
    let result = DefSubclassProc(window, message, wparam, lparam);
    if message == WM_NCHITTEST && !rules.configuring.get() {
        if rules.position_locked.get() && result == HTCAPTION as isize {
            return HTCLIENT as isize;
        }
        if rules.size_locked.get() && (HTLEFT as isize..=HTBOTTOMRIGHT as isize).contains(&result) {
            return HTCLIENT as isize;
        }
    }
    if message == WM_WINDOWPOSCHANGED && IsIconic(window) == 0 {
        rules.anchor.set(desktop::parent_rect(window));
    }
    if message == WM_SIZE && !rules.configuring.get() && !rules.rounding.replace(true) {
        let _ = corners::apply(
            window,
            rules.rounded.get(),
            rules.decorated.get(),
            rules.scale.get(),
        );
        rules.rounding.set(false);
    }
    result
}
