// 功能：验证窗口几何回传能更新设置草稿，同时不覆盖用户正在编辑的字段。
import assert from 'node:assert/strict';
import test from 'node:test';
import { effectScope, nextTick, ref } from 'vue';
import { usePinSettingsForm } from '../src/features/pins/usePinSettingsForm.ts';

/// 构造独立页面快照；测试只关心表单行为，不连接原生窗口或真实配置。
function createPin(id = 'p-test') {
  return {
    id,
    name: '便签',
    source: { kind: 'bundled', entry: 'index.html' },
    width: 420,
    height: 360,
    x: 60,
    y: 80,
    scale: 1.25,
    alwaysOnTop: true,
    decorations: false,
    enabled: true,
    mode: 'floating',
    rounded: false,
    positionLocked: false,
    sizeLocked: false,
  };
}

/// 在可释放的 Vue 作用域中创建表单，确保每条测试不遗留侦听器。
function setup(t) {
  const pin = ref(createPin());
  const scope = effectScope();
  const input = scope.run(() => usePinSettingsForm(() => pin.value));
  t.after(() => scope.stop());
  return { pin, input };
}

test('窗口移动和缩放后，已打开表单同步逻辑尺寸与物理坐标', async (t) => {
  const { pin, input } = setup(t);
  pin.value = { ...pin.value, width: 612.4, height: 398.6, x: -300, y: 240 };
  await nextTick();
  assert.deepEqual([input.width, input.height, input.x, input.y], [612, 399, -300, 240]);
});

test('用户正在编辑的字段保留，未编辑的字段跟随连续回传', async (t) => {
  const { pin, input } = setup(t);
  input.name = '我的便签';
  input.width = 500;
  pin.value = { ...pin.value, name: '原生名称', width: 700, height: 450, x: 220, y: 180 };
  await nextTick();
  pin.value.height = 480;
  pin.value.x = 260;
  await nextTick();
  assert.deepEqual(
    { name: input.name, width: input.width, height: input.height, x: input.x, y: input.y },
    { name: '我的便签', width: 500, height: 480, x: 260, y: 180 },
  );
});

test('只修改外观后提交时使用最新窗口几何，不把窗口移回旧位置', async (t) => {
  const { pin, input } = setup(t);
  input.rounded = true;
  pin.value = { ...pin.value, width: 640, height: 420, x: 900, y: 500 };
  await nextTick();
  const submitted = { ...input };
  assert.deepEqual(
    [submitted.width, submitted.height, submitted.x, submitted.y, submitted.rounded],
    [640, 420, 900, 500, true],
  );
});

test('切换页面时重置草稿，原生锁定变化和负坐标可以回传', async (t) => {
  const { pin, input } = setup(t);
  input.name = '尚未提交';
  input.x = 999;
  pin.value = { ...createPin('p-another'), name: '时钟', x: -800, positionLocked: true };
  await nextTick();
  assert.equal(input.name, '时钟');
  assert.equal(input.x, -800);
  assert.equal(input.positionLocked, true);
});
