// 功能：验证手势按键归属、取消与成对收尾，确保不会把残缺序列漏给网页。
use super::*;

#[test]
/// 右键松开只触发一次菜单，状态机不再查询已经松开的修饰键。
fn right_release_opens_once_without_rechecking_modifier() {
    let mut sequence = Sequence::new(Button::Right);
    assert!(sequence.owns(Button::Right));
    assert!(sequence.release(Button::Right));
    assert!(sequence.finished());
    assert!(!sequence.release(Button::Right));
}

#[test]
/// 取消动作后依然消费原按钮松开，防止留下孤立的 mouseup。
fn cancellation_still_drains_owned_release() {
    let mut sequence = Sequence::new(Button::Right);
    sequence.cancel();
    assert!(sequence.owns(Button::Right));
    assert!(!sequence.release(Button::Right));
    assert!(sequence.finished());
}

#[test]
/// 无关按钮的松开不结束当前手势，也不弹出菜单。
fn unrelated_release_preserves_owner() {
    let mut sequence = Sequence::new(Button::Right);
    assert!(!sequence.owns(Button::Left));
    assert!(!sequence.release(Button::Left));
    assert!(!sequence.finished());
    assert!(sequence.release(Button::Right));
}

#[test]
/// 左键结束不触发菜单，取消后仍保留按钮归属直至松开。
fn drag_release_and_cancel_never_open_menu() {
    let mut sequence = Sequence::new(Button::Left);
    assert!(sequence.owns(Button::Left));
    sequence.cancel();
    assert!(sequence.owns(Button::Left));
    assert!(!sequence.release(Button::Left));
    assert!(sequence.finished());
}

#[test]
/// 混合按钮按下取消动作，任意松开顺序均需排空两个按钮。
fn chord_cancels_and_drains_in_both_orders() {
    for primary in [Button::Left, Button::Right] {
        for extra in [
            Button::Left,
            Button::Right,
            Button::Middle,
            Button::X1,
            Button::X2,
        ] {
            if primary == extra {
                continue;
            }
            for order in [[primary, extra], [extra, primary]] {
                let mut sequence = Sequence::new(primary);
                sequence.press(extra);
                assert!(sequence.owns(primary));
                assert!(sequence.owns(extra));
                assert!(!sequence.release(order[0]));
                assert!(!sequence.finished());
                assert!(!sequence.release(order[1]));
                assert!(sequence.finished());
            }
        }
    }
}
