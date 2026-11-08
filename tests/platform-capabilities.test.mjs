// 功能：验证平台能力对预设过滤、旧配置回退和拖动提示的统一规则。
import assert from 'node:assert/strict';
import test from 'node:test';
import {
  disabledCapabilities,
  dragHintKey,
  fallbackWindowFields,
  hasLimitedWindowFeatures,
  hasNativeDrag,
  supportedPresets,
  supportsWindow,
} from '../src/features/pins/platform.ts';

/// 模拟 Windows 的完整能力，预设判断与当前测试机平台无关。
const full = {
  desktopMode: true,
  rounded: true,
  positionLock: true,
  sizeLock: true,
  windowGestures: true,
  dragModifier: 'alt',
};

/// 只支持标准浮动窗口的平台，用于模拟 macOS 当前的回退边界。
const limited = {
  ...disabledCapabilities,
  dragModifier: null,
};

const presets = [
  { id: 'fullscreen', mode: 'floating', rounded: true, positionLocked: false, sizeLocked: false },
  { id: 'standard', mode: 'floating', rounded: false, positionLocked: false, sizeLocked: false },
  { id: 'topRight', mode: 'floating', rounded: true, positionLocked: true, sizeLocked: false },
  { id: 'bottomRight', mode: 'desktop', rounded: true, positionLocked: true, sizeLocked: true },
];

test('完整能力保留全部预设，受限平台只保留完全可应用的预设', () => {
  assert.deepEqual(
    supportedPresets(presets, full).map((preset) => preset.id),
    ['fullscreen', 'standard', 'topRight', 'bottomRight'],
  );
  assert.deepEqual(
    supportedPresets(presets, limited).map((preset) => preset.id),
    ['standard'],
  );
});

test('每项能力都参与判断，不能只按桌面模式裁剪', () => {
  assert.equal(supportsWindow(presets[1], limited), true);
  assert.equal(supportsWindow(presets[0], limited), false);
  assert.equal(supportsWindow(presets[2], limited), false);
  assert.equal(supportsWindow(presets[3], limited), false);
});

test('旧桌面配置回退为带系统边框的浮动窗口并保留几何内容', () => {
  const fallback = fallbackWindowFields(
    {
      mode: 'desktop',
      decorations: false,
      rounded: true,
      positionLocked: true,
      sizeLocked: true,
      width: 420,
      height: 360,
      x: 30,
      y: 40,
    },
    limited,
  );
  assert.deepEqual(fallback, {
    mode: 'floating',
    decorations: true,
    rounded: false,
    positionLocked: false,
    sizeLocked: false,
    width: 420,
    height: 360,
    x: 30,
    y: 40,
  });
});

test('已支持的字段不被回退改写，完整能力平台保持原组合', () => {
  const standard = presets[1];
  assert.deepEqual(fallbackWindowFields(standard, limited), standard);
  const desktop = presets[3];
  assert.deepEqual(fallbackWindowFields(desktop, full), desktop);
});

test('拖动提示按能力选择，受限平台说明可从托盘或管理器找回', () => {
  assert.equal(dragHintKey(full), 'ui.pinSettings.dragHint');
  assert.equal(
    dragHintKey({ ...disabledCapabilities, dragModifier: 'option' }),
    'ui.pinSettings.dragHintOption',
  );
  assert.equal(dragHintKey(limited), 'ui.pinSettings.dragHintFallback');
  assert.equal(hasLimitedWindowFeatures(full), false);
  assert.equal(hasLimitedWindowFeatures(limited), true);
  assert.equal(hasNativeDrag(full), true);
  assert.equal(hasNativeDrag({ ...limited, dragModifier: 'option' }), true);
  assert.equal(hasNativeDrag(limited), false);
});
