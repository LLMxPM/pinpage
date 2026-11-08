// 功能：验证卡片主操作的运行、可见与错误优先级，以及置前入口只在浮动可见窗口出现。
import assert from 'node:assert/strict';
import test from 'node:test';
import { canBringToFront, primaryAction } from '../src/features/pins/pinActions.ts';

/// 构造最小页面配置；测试只关心操作语义，不涉及真实窗口。
function createPin(mode = 'floating') {
  return { mode };
}

/// 构造运行态，缺省为已开启且可见。
function createRuntime(overrides = {}) {
  return {
    open: true,
    visible: true,
    loading: false,
    currentUrl: 'https://example.com',
    error: null,
    ...overrides,
  };
}

test('未开启时主操作是开启页面', () => {
  assert.deepEqual(primaryAction(createPin(), undefined), {
    operation: 'open',
    icon: 'power',
    labelKey: 'ui.card.open',
  });
});

test('已开启但隐藏时优先恢复显示，即使加载失败', () => {
  const hidden = createRuntime({ visible: false });
  assert.deepEqual(primaryAction(createPin(), hidden), {
    operation: 'focus',
    icon: 'eye',
    labelKey: 'ui.card.show',
  });
  assert.equal(
    primaryAction(createPin(), { ...hidden, error: 'errors.loadFailed' })?.operation,
    'focus',
  );
});

test('可见且正常时没有主操作，避免出现无反馈的按钮', () => {
  assert.equal(primaryAction(createPin(), createRuntime()), null);
  assert.equal(primaryAction(createPin('desktop'), createRuntime()), null);
});

test('可见但加载失败时主操作是重新加载', () => {
  assert.deepEqual(primaryAction(createPin(), createRuntime({ error: 'errors.loadFailed' })), {
    operation: 'reload',
    icon: 'refresh',
    labelKey: 'ui.card.retry',
  });
});

test('只有打开、可见、无错误且为浮动模式时才提供置前入口', () => {
  assert.equal(canBringToFront(createPin(), createRuntime()), true);
  assert.equal(canBringToFront(createPin('desktop'), createRuntime()), false);
  assert.equal(canBringToFront(createPin(), createRuntime({ visible: false })), false);
  assert.equal(canBringToFront(createPin(), createRuntime({ error: 'errors.loadFailed' })), false);
  assert.equal(canBringToFront(createPin(), undefined), false);
});
