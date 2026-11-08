// 功能：验证平台能力声明、页面安全回退和预设能力校验。
use super::*;

/// 构造包含全部 Windows 专属能力的页面，便于测试被裁剪的字段。
fn full_pin() -> PinConfig {
    serde_json::from_value(serde_json::json!({
        "id": "p-platform",
        "name": "计时器",
        "source": {"kind": "url", "url": "https://example.com/"},
        "width": 420,
        "height": 360,
        "x": 30,
        "y": 40,
        "scale": 1.0,
        "alwaysOnTop": false,
        "decorations": false,
        "enabled": true,
        "mode": "desktop",
        "rounded": true,
        "positionLocked": true,
        "sizeLocked": true,
        "preset": "bottomRight"
    }))
    .unwrap()
}

/// 模拟只支持标准浮动窗口的平台，不依赖当前测试机操作系统。
fn limited() -> PlatformCapabilities {
    PlatformCapabilities {
        desktop_mode: false,
        rounded: false,
        position_lock: false,
        size_lock: false,
        window_gestures: false,
        drag_modifier: None,
    }
}

#[test]
/// 当前构建的能力声明必须与实际实现一致：只有 Windows 开放完整能力。
fn current_capabilities_match_platform() {
    let capabilities = capabilities();
    if cfg!(windows) {
        assert!(capabilities.desktop_mode);
        assert!(capabilities.rounded);
        assert!(capabilities.position_lock);
        assert!(capabilities.size_lock);
        assert!(capabilities.window_gestures);
        assert_eq!(
            capabilities.drag_modifier,
            Some(crate::models::PlatformDragModifier::Alt)
        );
    } else {
        assert!(!capabilities.desktop_mode);
        assert!(!capabilities.rounded);
        assert!(!capabilities.position_lock);
        assert!(!capabilities.size_lock);
        assert!(!capabilities.window_gestures);
        assert_eq!(capabilities.drag_modifier, None);
    }
}

#[test]
/// 从 Windows 配置加载到能力不足的平台时，页面降级为可找回到标准浮动窗口。
fn unsupported_pin_falls_back_without_losing_content() {
    let mut pin = full_pin();
    assert!(fallback_pin(limited(), &mut pin));
    assert_eq!(pin.mode, WindowMode::Floating);
    assert!(pin.decorations);
    assert!(!pin.rounded);
    assert!(!pin.position_locked);
    assert!(!pin.size_locked);
    assert!(pin.preset.is_none());
    assert_eq!((pin.width, pin.height), (420., 360.));
    assert_eq!((pin.x, pin.y), (Some(30), Some(40)));
    assert_eq!(pin.name, "计时器");
    assert!(pin.enabled);
}

#[test]
/// 已符合能力的页面不会被重复改写；完整能力平台保留用户原组合。
fn supported_pin_is_not_rewritten() {
    let mut pin = full_pin();
    assert!(!fallback_pin(capabilities(), &mut pin));
    assert_eq!(pin.mode, WindowMode::Desktop);
    assert!(pin.rounded);
    assert_eq!(pin.preset.as_deref(), Some("bottomRight"));

    let mut limited_pin = pin.clone();
    limited_pin.mode = WindowMode::Floating;
    limited_pin.decorations = true;
    limited_pin.rounded = false;
    limited_pin.position_locked = false;
    limited_pin.size_locked = false;
    limited_pin.preset = None;
    assert!(!fallback_pin(limited(), &mut limited_pin));
}

#[test]
/// 显式提交不支持的能力必须报错，回退只能发生在配置加载阶段。
fn explicit_unsupported_requests_are_rejected() {
    let pin = full_pin();
    assert!(validate_pin(capabilities(), &pin).is_ok());
    let error = validate_pin(limited(), &pin).unwrap_err();
    assert_eq!(error.code, "errors.platformUnsupported");

    let floating: PinConfig = serde_json::from_value(serde_json::json!({
        "id": "p-platform",
        "name": "计时器",
        "source": {"kind": "url", "url": "https://example.com/"},
        "width": 420,
        "height": 360,
        "x": 30,
        "y": 40,
        "scale": 1.0,
        "alwaysOnTop": false,
        "decorations": true,
        "enabled": true,
        "mode": "floating",
        "rounded": false,
        "positionLocked": false,
        "sizeLocked": false
    }))
    .unwrap();
    assert!(validate_pin(limited(), &floating).is_ok());
}

#[test]
/// 预设应用与写入使用同一能力边界，不能在桌面模式下绕过锁或圆角限制。
fn preset_validation_checks_each_capability() {
    let desktop = PresetFields {
        mode: WindowMode::Desktop,
        always_on_top: false,
        decorations: false,
        rounded: true,
        position_locked: true,
        size_locked: true,
        width: 320.,
        height: 480.,
        fx: 1.,
        fy: 1.,
        fullscreen: false,
    };
    assert!(validate_preset(capabilities(), &desktop).is_ok());
    assert_eq!(
        validate_preset(limited(), &desktop).unwrap_err().code,
        "errors.platformUnsupported"
    );

    let standard = PresetFields {
        mode: WindowMode::Floating,
        decorations: true,
        rounded: false,
        position_locked: false,
        size_locked: false,
        ..desktop
    };
    assert!(validate_preset(limited(), &standard).is_ok());
}
