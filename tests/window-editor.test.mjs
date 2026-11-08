// 功能：验证连续窗口输入、失败保留、原生回传和预设保存前等待的业务顺序。
import assert from 'node:assert/strict';
import test from 'node:test';
import { registerHooks } from 'node:module';
import { effectScope, nextTick, ref } from 'vue';
import { createOperationQueue } from '../src/features/pins/operationQueue.ts';

// Node 的类型擦除不处理前端无后缀导入；只为本组测试解析项目内的 TypeScript。
const hooks = registerHooks({
  resolve(specifier, context, nextResolve) {
    try {
      return nextResolve(specifier, context);
    } catch (error) {
      if (specifier.startsWith('.') && context.parentURL?.includes('/src/features/pins/'))
        return nextResolve(`${specifier}.ts`, context);
      throw error;
    }
  },
});
const { useLiveWindow } = await import('../src/features/pins/useLiveWindow.ts');
hooks.deregister();

/// 创建不连接 IPC 的真实字段模型，原生状态通过响应式快照模拟。
function setup(t, apply) {
  const pin = ref({
    id: 'p-live',
    name: '计时器',
    source: { kind: 'url', url: 'https://example.com/' },
    width: 420,
    height: 360,
    x: 60,
    y: 80,
    scale: 1.25,
    alwaysOnTop: false,
    decorations: false,
    enabled: true,
    mode: 'floating',
    rounded: true,
    positionLocked: false,
    sizeLocked: false,
    preset: null,
  });
  const scope = effectScope();
  const live = scope.run(() =>
    useLiveWindow(
      () => pin.value,
      (patch) => apply(patch, pin),
    ),
  );
  t.after(() => scope.stop());
  return { pin, live };
}

test('连续输入期间的旧回传不覆盖新草稿，另存前等待并提交最后一次输入', async (t) => {
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  const calls = [];
  const { pin, live } = setup(t, async (patch, actual) => {
    calls.push(patch);
    if (calls.length === 1) await gate;
    actual.value = { ...actual.value, ...patch };
    await nextTick();
    return actual.value;
  });
  live.input.width = 500;
  const first = live.commit('width');
  live.input.width = 620;
  pin.value = { ...pin.value, x: -900 };
  await nextTick();
  const savingPreset = live.flush();
  assert.equal(calls.length, 1);
  release();
  await first;
  assert.equal(await savingPreset, true);
  assert.deepEqual(calls, [{ width: 500 }, { width: 620 }]);
  assert.equal(pin.value.x, -900);
  assert.equal(live.input.width, 620);
  assert.equal(live.dirty, false);
});

test('失败保留输入和实际读数，非法或不完整数字不会进入保存预设的链路', async (t) => {
  const calls = [];
  const { pin, live } = setup(t, async (patch) => {
    calls.push(patch);
    return null;
  });
  live.input.width = 500;
  await live.commit('width');
  assert.equal(live.failed, true);
  assert.equal(live.input.width, 500);
  assert.equal(pin.value.width, 420);
  assert.equal(await live.flush(), false);
  live.reset();
  live.input.x = '';
  assert.equal(await live.flush(), false);
  live.input.x = 0.5;
  assert.equal(await live.flush(), false);
  assert.equal(calls.length, 1);
});

test('快速将开关切回原值时，第二次操作仍然排队且不被首次回传覆盖', async (t) => {
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  const enqueue = createOperationQueue();
  const calls = [];
  const { pin, live } = setup(t, (patch, actual) =>
    enqueue(async () => {
      calls.push(patch);
      if (calls.length === 1) await gate;
      actual.value = { ...actual.value, ...patch };
      await nextTick();
      return actual.value;
    }),
  );
  live.input.alwaysOnTop = true;
  const first = live.commit('alwaysOnTop');
  live.input.alwaysOnTop = false;
  const second = live.commit('alwaysOnTop');
  release();
  await Promise.all([first, second]);
  assert.deepEqual(calls, [{ alwaysOnTop: true }, { alwaysOnTop: false }]);
  assert.equal(pin.value.alwaysOnTop, false);
  assert.equal(live.input.alwaysOnTop, false);
  assert.equal(live.dirty, false);
});

test('窗口系统校正后的实际数值回填，输入相同值不会重复提交', async (t) => {
  let count = 0;
  const { live } = setup(t, async (patch, pin) => {
    count++;
    pin.value = { ...pin.value, ...patch, width: 504 };
    await nextTick();
    return pin.value;
  });
  live.input.width = 500;
  await live.commit('width');
  assert.equal(live.input.width, 504);
  await live.commit('width');
  assert.equal(count, 1);
});

test('即时调整、另存和删除按调用顺序执行，中途失败不丢弃后续任务', async () => {
  const enqueue = createOperationQueue();
  const calls = [];
  const first = enqueue(async () => {
    calls.push('调整');
    throw new Error('模拟应用失败');
  });
  const second = enqueue(async () => {
    calls.push('另存');
    return 'preset-id';
  });
  const third = enqueue(async () => {
    calls.push('删除');
  });
  await assert.rejects(first, /模拟应用失败/);
  assert.equal(await second, 'preset-id');
  await third;
  assert.deepEqual(calls, ['调整', '另存', '删除']);
});
