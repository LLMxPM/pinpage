// 功能：纯状态机管理一次鼠标手势的归属与成对收尾，不依赖窗口或系统输入。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left = 1,
    Right = 2,
    Middle = 4,
    X1 = 8,
    X2 = 16,
}

#[derive(Debug)]
pub struct Sequence {
    pub primary: Button,
    held: u8,
    cancelled: bool,
}

impl Sequence {
    /// 仅在修饰键和目标窗口已校验的鼠标按下沿建立归属。
    pub fn new(primary: Button) -> Self {
        Self {
            primary,
            held: primary as u8,
            cancelled: false,
        }
    }

    /// 手势内追加的按键也要成对收尾，同时取消菜单或拖动，避免混合操作。
    pub fn press(&mut self, button: Button) {
        self.held |= button as u8;
        self.cancelled = true;
    }

    /// 消费属于本次手势的松开；返回是否应在原窗口打开菜单。
    pub fn release(&mut self, button: Button) -> bool {
        let owned = self.held & button as u8 != 0;
        self.held &= !(button as u8);
        owned && button == self.primary && button == Button::Right && !self.cancelled
    }

    /// 只允许本次已接管的按钮松开被消费。
    pub fn owns(&self, button: Button) -> bool {
        self.held & button as u8 != 0
    }

    /// 所有接管按钮都松开后才能接受下一次手势。
    pub fn finished(&self) -> bool {
        self.held == 0
    }

    /// 取消只禁止动作，后续松开仍由原手势消费。
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

#[cfg(test)]
#[path = "sequence_tests.rs"]
mod tests;
