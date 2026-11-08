// 功能：以纯状态计算拖动位置，阈值只在起步时生效，支持负坐标与拖回起点。
#[derive(Clone, Copy)]
pub struct Motion {
    origin: (i32, i32),
    pointer: (i32, i32),
    threshold: (u32, u32),
    started: bool,
}

impl Motion {
    /// 保存物理坐标与系统拖动阈值，不依赖当前窗口位置累计位移。
    pub fn new(origin: (i32, i32), pointer: (i32, i32), threshold: (u32, u32)) -> Self {
        Self {
            origin,
            pointer,
            threshold,
            started: false,
        }
    }

    /// 超过起步阈值后持续返回绝对位置；使用宽整数防止坐标差溢出。
    pub fn position(&mut self, point: (i32, i32)) -> Option<(i32, i32)> {
        let dx = i64::from(point.0) - i64::from(self.pointer.0);
        let dy = i64::from(point.1) - i64::from(self.pointer.1);
        if !self.started {
            if dx.unsigned_abs() < u64::from(self.threshold.0)
                && dy.unsigned_abs() < u64::from(self.threshold.1)
            {
                return None;
            }
            self.started = true;
        }
        let clamp = |value: i64| value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        Some((
            clamp(i64::from(self.origin.0) + dx),
            clamp(i64::from(self.origin.1) + dy),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::Motion;

    #[test]
    /// 未跨阈值不移动，跨过后返回起点附近也必须更新。
    fn threshold_only_applies_before_drag_starts() {
        let mut motion = Motion::new((10, 20), (100, 200), (4, 4));
        assert_eq!(motion.position((103, 203)), None);
        assert_eq!(motion.position((104, 200)), Some((14, 20)));
        assert_eq!(motion.position((101, 199)), Some((11, 19)));
        assert_eq!(motion.position((100, 200)), Some((10, 20)));
    }

    #[test]
    /// 多屏负坐标与跨屏移动按屏幕坐标差计算。
    fn negative_coordinates_and_vertical_threshold() {
        let mut motion = Motion::new((-1200, -600), (-1000, -500), (4, 4));
        assert_eq!(motion.position((-1000, -504)), Some((-1200, -604)));
        assert_eq!(motion.position((100, 100)), Some((-100, 0)));
    }

    #[test]
    /// 极端坐标在最终位置饱和，不因中间减法溢出而反向移动。
    fn position_saturates_without_intermediate_overflow() {
        let mut motion = Motion::new((10, -10), (i32::MIN, i32::MAX), (4, 4));
        assert_eq!(
            motion.position((i32::MAX, i32::MIN)),
            Some((i32::MAX, i32::MIN))
        );
    }
}
