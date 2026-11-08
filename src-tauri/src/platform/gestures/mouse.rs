// 功能：独立消息线程只拦截已匹配的鼠标按键对，移动永远放行，不记录或拦截键盘。
use super::{
    input,
    sequence::{Button, Sequence},
    *,
};
use std::{cell::RefCell, ptr::null_mut, sync::atomic::AtomicU32};
use windows_sys::Win32::{
    Foundation::{LRESULT, WPARAM},
    System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
};

static THREAD: AtomicU32 = AtomicU32::new(0);
thread_local! { static OWNED: RefCell<Option<Owned>> = const { RefCell::new(None) }; }
struct Owned {
    sequence: Sequence,
    token: usize,
}

/// 确认钩子安装结果后返回；消息线程不使用按键轮询，也不等待 UI 窗口操作。
pub fn start() -> Result<(), String> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("pinpage-gestures".into())
        .spawn(move || unsafe {
            let mut message = MSG::default();
            PeekMessageW(&mut message, null_mut(), 0, 0, PM_NOREMOVE);
            let hook = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(callback),
                GetModuleHandleW(std::ptr::null()),
                0,
            );
            if hook.is_null() {
                let _ = sender.send(Err(std::io::Error::last_os_error().to_string()));
                return;
            }
            THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
            let _ = sender.send(Ok(()));
            loop {
                let result = GetMessageW(&mut message, null_mut(), 0, 0);
                if result <= 0 {
                    break;
                }
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            UnhookWindowsHookEx(hook);
            THREAD.store(0, Ordering::SeqCst);
        })
        .map_err(|error| error.to_string())?;
    receiver.recv().map_err(|error| error.to_string())?
}

/// 退出仅通知本模块的线程，不等待可能正在回调的系统消息。
pub fn stop() {
    let thread = THREAD.load(Ordering::SeqCst);
    if thread != 0 {
        unsafe {
            PostThreadMessageW(thread, WM_QUIT, 0, 0);
        }
    }
}

/// 归一化按钮边沿，额外按钮仅用于已接管手势的成对排空。
fn button(message: u32, data: u32) -> Option<(Button, bool)> {
    Some(match message {
        WM_LBUTTONDOWN => (Button::Left, true),
        WM_LBUTTONUP => (Button::Left, false),
        WM_RBUTTONDOWN => (Button::Right, true),
        WM_RBUTTONUP => (Button::Right, false),
        WM_MBUTTONDOWN => (Button::Middle, true),
        WM_MBUTTONUP => (Button::Middle, false),
        WM_XBUTTONDOWN | WM_XBUTTONUP => (
            if data >> 16 == XBUTTON1 as u32 {
                Button::X1
            } else {
                Button::X2
            },
            message == WM_XBUTTONDOWN,
        ),
        _ => return None,
    })
}

/// 移动与模拟输入直接走系统链；回调只判定目标和投递实例票据，不访问应用状态锁。
unsafe extern "system" fn callback(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 || lparam == 0 || wparam as u32 == WM_MOUSEMOVE {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    let event = &*(lparam as *const MSLLHOOKSTRUCT);
    if event.flags & LLMHF_INJECTED != 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    let consumed = OWNED
        .try_with(|slot| {
            let Ok(mut owned) = slot.try_borrow_mut() else {
                return false;
            };
            process(&mut owned, wparam as u32, event)
        })
        .unwrap_or(false);
    if consumed {
        1
    } else {
        CallNextHookEx(null_mut(), code, wparam, lparam)
    }
}

/// 每次新的物理点击使旧菜单票据过期；只在按下沿识别组合，不中途抢走网页拖动。
fn process(slot: &mut Option<Owned>, message: u32, event: &MSLLHOOKSTRUCT) -> bool {
    let edge = button(message, event.mouseData);
    if let Some(owned) = slot.as_mut() {
        let consumed = continue_sequence(owned, message, edge, event.pt);
        if owned.sequence.finished() {
            *slot = None;
        }
        return consumed;
    }
    let Some((button, true)) = edge else {
        return false;
    };
    let token = CURRENT.fetch_add(1, Ordering::SeqCst) + 1;
    if !matches!(button, Button::Left | Button::Right)
        || !input::modifier_matches(modifier())
        || input::another_button_down(button)
        || input::input_busy()
    {
        return false;
    }
    let Some(target) = target_at(event.pt, button == Button::Left) else {
        return false;
    };
    INSTANCE.store(target.instance, Ordering::SeqCst);
    PRIMARY.store(button as u8, Ordering::SeqCst);
    HELD.store(true, Ordering::SeqCst);
    if !post(BEGIN, token, event.pt) {
        HELD.store(false, Ordering::SeqCst);
        return false;
    }
    *slot = Some(Owned {
        sequence: Sequence::new(button),
        token,
    });
    true
}

/// 已接管的按键必须完整排空，取消或更换修饰键不把残缺的松开漏给页面。
fn continue_sequence(
    owned: &mut Owned,
    message: u32,
    edge: Option<(Button, bool)>,
    point: POINT,
) -> bool {
    if let Some((button, pressed)) = edge {
        if pressed {
            owned.sequence.press(button);
            CANCELLED.store(owned.token, Ordering::SeqCst);
            post(CANCEL, owned.token, point);
        } else {
            if !owned.sequence.owns(button) {
                return false;
            }
            let menu = owned.sequence.release(button);
            if button == owned.sequence.primary {
                HELD.store(false, Ordering::SeqCst);
                if button == Button::Right && !menu {
                    CANCELLED.store(owned.token, Ordering::SeqCst);
                }
                post(FINISH, owned.token, point);
            }
        }
        return true;
    }
    if message == WM_MOUSEWHEEL || message == WM_MOUSEHWHEEL {
        owned.sequence.cancel();
        CANCELLED.store(owned.token, Ordering::SeqCst);
        post(CANCEL, owned.token, point);
        return true;
    }
    false
}
