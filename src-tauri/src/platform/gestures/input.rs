// 功能：校验精确修饰键、鼠标真实命中窗口及现有输入归属，避免抢占其他应用输入。
use super::{sequence::Button, Target};
use crate::models::GestureModifier;
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ScreenToClient,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

/// 只读取当前物理按键状态，不记录或拦截网页键盘事件。
fn down(key: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(key as i32) < 0 }
}

/// 精确匹配一类修饰键；右 Alt/AltGr 及额外修饰键始终不触发。
pub fn modifier_matches(modifier: GestureModifier) -> bool {
    matches_keys(
        modifier,
        down(VK_LMENU),
        down(VK_RMENU),
        down(VK_CONTROL),
        down(VK_SHIFT),
        down(VK_LWIN) || down(VK_RWIN),
    )
}

/// 将系统读数与策略分开，便于穷举额外修饰键及 AltGr 的回归用例。
pub fn matches_keys(
    modifier: GestureModifier,
    left_alt: bool,
    right_alt: bool,
    ctrl: bool,
    shift: bool,
    win: bool,
) -> bool {
    if right_alt || win {
        return false;
    }
    match modifier {
        GestureModifier::Alt => left_alt && !ctrl && !shift,
        GestureModifier::Ctrl => ctrl && !left_alt && !shift,
        GestureModifier::Shift => shift && !left_alt && !ctrl,
        GestureModifier::Disabled => false,
    }
}

/// 已有其他鼠标键按下时不抢走网页或其他程序正在进行的操作。
pub fn another_button_down(button: Button) -> bool {
    [
        (Button::Left, VK_LBUTTON),
        (Button::Right, VK_RBUTTON),
        (Button::Middle, VK_MBUTTON),
        (Button::X1, VK_XBUTTON1),
        (Button::X2, VK_XBUTTON2),
    ]
    .iter()
    .any(|(other, key)| *other != button && down(*key))
}

/// 系统捕获、菜单或移动循环已有输入所有者时，不发起新的窗口手势。
pub fn input_busy() -> bool {
    let mut info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        GetGUIThreadInfo(0, &mut info) == 0
            || !info.hwndCapture.is_null()
            || info.flags
                & (GUI_INMENUMODE | GUI_INMOVESIZE | GUI_POPUPMENUMODE | GUI_SYSTEMMENUMODE)
                != 0
    }
}

/// 既校验实际命中的子窗口，也限制在客户区；不按外接矩形抢占被遮挡或标题栏输入。
pub fn hit(target: &Target, point: POINT) -> bool {
    unsafe {
        let hwnd = target.hwnd as HWND;
        let under = WindowFromPoint(point);
        if IsWindowVisible(hwnd) == 0
            || IsWindowEnabled(hwnd) == 0
            || IsIconic(hwnd) != 0
            || (under != hwnd && IsChild(hwnd, under) == 0)
        {
            return false;
        }
        let mut rect = RECT::default();
        let mut local = point;
        GetClientRect(hwnd, &mut rect) != 0
            && ScreenToClient(hwnd, &mut local) != 0
            && local.x >= rect.left
            && local.x < rect.right
            && local.y >= rect.top
            && local.y < rect.bottom
    }
}

/// 菜单显示前检查两侧 Alt 是否仍按住；按住期间的自动重复会关闭原生菜单。
pub fn alt_held() -> bool {
    down(VK_LMENU) || down(VK_RMENU)
}

/// 异步菜单显示前不允许抢占已经开始的任何鼠标按压。
pub fn any_button_down() -> bool {
    [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_XBUTTON1, VK_XBUTTON2]
        .into_iter()
        .any(down)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 所有修饰键组合中只接受单独左 Alt、Ctrl 或 Shift，禁用始终放行。
    fn exact_modifiers_exclude_altgr_and_extra_keys() {
        for mask in 0u8..32 {
            let keys = (
                mask & 1 != 0,
                mask & 2 != 0,
                mask & 4 != 0,
                mask & 8 != 0,
                mask & 16 != 0,
            );
            for (modifier, expected) in [
                (GestureModifier::Alt, 1),
                (GestureModifier::Ctrl, 4),
                (GestureModifier::Shift, 8),
                (GestureModifier::Disabled, 255),
            ] {
                assert_eq!(
                    matches_keys(modifier, keys.0, keys.1, keys.2, keys.3, keys.4),
                    mask == expected
                );
            }
        }
    }
}
