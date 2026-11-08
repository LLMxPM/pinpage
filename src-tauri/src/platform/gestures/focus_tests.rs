// 功能：用本线程隐藏窗口验证手势焦点消息隔离，不改变真实键盘焦点或注入系统输入。
use super::*;

/// 模拟下游 WebView 焦点转交，仅计数消息，不执行真实 SetFocus。
unsafe extern "system" fn downstream(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    if message == WM_SETFOCUS {
        let count = &*(data as *const Cell<u32>);
        count.set(count.get() + 1);
    }
    DefSubclassProc(hwnd, message, wparam, lparam)
}

#[test]
/// 手势期间焦点消息止于宿主，收尾后重新交给正常消息链。
fn host_focus_is_retained_only_during_owned_gesture() {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            w!("STATIC"),
            w!("PinPage focus routing test"),
            WS_POPUP,
            0,
            0,
            100,
            100,
            null_mut(),
            null_mut(),
            null_mut(),
            std::ptr::null(),
        );
        assert!(!hwnd.is_null());
        let count = Cell::new(0u32);
        assert_ne!(
            SetWindowSubclass(hwnd, Some(downstream), 1, &count as *const _ as usize),
            0
        );
        install(hwnd, usize::MAX).unwrap();
        CAPTURE.with(|slot| {
            slot.set(Some(Capture {
                hwnd,
                instance: usize::MAX,
                token: usize::MAX,
                left: true,
                origin: RECT::default(),
                motion: motion::Motion::new((0, 0), (0, 0), (4, 4)),
            }))
        });
        SendMessageW(hwnd, WM_SETFOCUS, 0, 0);
        let during = count.get();
        CAPTURE.with(|slot| slot.set(None));
        SendMessageW(hwnd, WM_SETFOCUS, 0, 0);
        let after = count.get();
        RemoveWindowSubclass(hwnd, Some(downstream), 1);
        DestroyWindow(hwnd);
        assert_eq!(during, 0);
        assert_eq!(after, 1);
    }
}
