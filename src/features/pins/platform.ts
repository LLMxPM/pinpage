// 功能：集中判断窗口能力、过滤不可应用预设，并为旧配置生成前端回退草稿。

import type { PlatformCapabilities, PresetFields, WindowPreset } from '../../types';

/// 不具备任何原生窗口能力时使用的最保守组合，也供浏览器预览复用。
export const disabledCapabilities: PlatformCapabilities = {
  desktopMode: false,
  rounded: false,
  positionLock: false,
  sizeLock: false,
  windowGestures: false,
  dragModifier: null,
};

/// 能力判断只需要这些窗口字段，页面与预设可共用同一实现。
type CapabilityFields = Pick<
  PresetFields,
  'mode' | 'decorations' | 'rounded' | 'positionLocked' | 'sizeLocked'
>;

/// 页面或预设是否只使用当前平台实际支持的能力。
export function supportsWindow(
  fields: CapabilityFields,
  capabilities: PlatformCapabilities,
): boolean {
  return (
    (fields.mode === 'floating' || capabilities.desktopMode) &&
    (!fields.rounded || capabilities.rounded) &&
    (!fields.positionLocked || capabilities.positionLock) &&
    (!fields.sizeLocked || capabilities.sizeLock)
  );
}

/// 保留可应用的预设及原顺序；桌面、圆角和独立锁定都参与判断。
export function supportedPresets(
  presets: WindowPreset[],
  capabilities: PlatformCapabilities,
): WindowPreset[] {
  return presets.filter((preset) => supportsWindow(preset, capabilities));
}

/// 将旧配置或跨平台预设转为当前平台可保存的组合，不修改尺寸、位置和内容。
export function fallbackWindowFields<T extends CapabilityFields>(
  fields: T,
  capabilities: PlatformCapabilities,
): T {
  const next = { ...fields };
  if (next.mode === 'desktop' && !capabilities.desktopMode) {
    // 桌面模式回到浮动层时恢复系统边框，避免无边框窗口缺少可靠操作入口。
    next.mode = 'floating';
    next.decorations = true;
    next.rounded = false;
    next.positionLocked = false;
    next.sizeLocked = false;
  }
  if (!capabilities.rounded) next.rounded = false;
  if (!capabilities.positionLock) next.positionLocked = false;
  if (!capabilities.sizeLock) next.sizeLocked = false;
  return next;
}

/// 当前平台是否存在可用的原生拖动方式；无手势时新页面默认保留系统边框。
export function hasNativeDrag(capabilities: PlatformCapabilities): boolean {
  return capabilities.windowGestures || capabilities.dragModifier !== null;
}

/// 无边框窗口的拖动提示；固定选项和完全没有手势时使用各自的降级文案。
export function dragHintKey(capabilities: PlatformCapabilities): string {
  if (capabilities.windowGestures) return 'ui.pinSettings.dragHint';
  if (capabilities.dragModifier) return 'ui.pinSettings.dragHintOption';
  return 'ui.pinSettings.dragHintFallback';
}

/// 是否存在需要在界面统一说明的能力限制。
export function hasLimitedWindowFeatures(capabilities: PlatformCapabilities): boolean {
  return (
    !capabilities.desktopMode ||
    !capabilities.rounded ||
    !capabilities.positionLock ||
    !capabilities.sizeLock ||
    !capabilities.windowGestures
  );
}
