// 功能：验证示例合并的数据迁移、休眠计时、日期边界、看板约束与行情本地排序。
import assert from 'node:assert/strict';
import test from 'node:test';
import {
  restoreTimer,
  remainingAt,
  settleTimer,
  elapsedAt,
  restoreStopwatch,
  formatDuration,
} from '../examples/timer/site/time-state.js';
import { normalizeTasks } from '../examples/notes/site/task-state.js';
import { sampleIsFresh } from '../examples/system-monitor/site/sample-state.js';
import { parseDateOnly, daysUntil, normalizeEvents } from '../examples/calendar/site/date-state.js';
import { normalizeCards, moveCard } from '../examples/kanban/site/board-state.js';
import { visibleSymbols } from '../examples/stocks/site/list-model.js';

test('旧计时状态保留截止时间，休眠到期只结算一次且保留休息类型', () => {
  const old = { duration: 300000, remaining: 300000, deadline: 123456789 };
  assert.deepEqual(restoreTimer(old), { ...old, kind: 'focus' });
  const rest = restoreTimer({ ...old, kind: 'rest' });
  assert.equal(remainingAt(rest, old.deadline - 1000), 1000);
  assert.equal(settleTimer(rest, old.deadline - 1), false);
  assert.equal(settleTimer(rest, old.deadline + 3600000), true);
  assert.equal(rest.kind, 'rest');
  assert.equal(rest.remaining, 0);
  assert.equal(settleTimer(rest, old.deadline + 7200000), false);
});

test('损坏计时输入回退，秒表跨重载补算且时间回拨不会变成负数', () => {
  for (const raw of [
    null,
    {},
    { duration: -1 },
    { duration: 86400001 },
    { duration: 1000, remaining: 1001, deadline: null },
  ])
    assert.equal(restoreTimer(raw).duration, 1500000);
  assert.equal(elapsedAt({ elapsed: 2000, startedAt: 10000 }, 15000), 7000);
  assert.equal(elapsedAt({ elapsed: 2000, startedAt: 10000 }, 9000), 2000);
  assert.deepEqual(restoreStopwatch({ elapsed: -1, startedAt: Infinity }), {
    elapsed: 0,
    startedAt: null,
  });
  assert.equal(formatDuration(3600000), '1:00:00');
  assert.equal(formatDuration(1), '00:01');
});

test('旧待办迁移稳定标识并过滤非法输入，完成状态和文本边界保持一致', () => {
  const raw = [
    { text: '  第一件  ', done: true },
    { id: 'same', text: '第二件' },
    { id: 'same', text: '第三件' },
    { text: ' ' },
    { text: 12 },
  ];
  const restored = normalizeTasks(raw);
  assert.equal(restored.length, 3);
  assert.equal(restored[0].text, '第一件');
  assert.equal(restored[0].done, true);
  assert.equal(new Set(restored.map((item) => item.id)).size, 3);
  assert.deepEqual(normalizeTasks(restored), restored);
  assert.equal(
    normalizeTasks(Array.from({ length: 150 }, () => ({ text: 'x'.repeat(200) }))).length,
    100,
  );
  assert.equal(normalizeTasks([{ text: 'x'.repeat(200) }])[0].text.length, 160);
});

test('成功读到旧缓存、异常未来采样及缺失时间戳都不能显示实时', () => {
  assert.equal(sampleIsFresh(10000, 13000), true);
  assert.equal(sampleIsFresh(10000, 16000), false);
  assert.equal(sampleIsFresh(20000, 10000), false);
  assert.equal(sampleIsFresh(null, 10000), false);
});

test('日期计算拒绝回绕，跨闰日、年末及本地夏令时仍按日历日计数', () => {
  for (const value of ['2026-02-29', '2026-02-30', '2026-13-01', '2026-1-01', null])
    assert.equal(parseDateOnly(value), null);
  assert.equal(daysUntil('2028-03-01', new Date(2028, 1, 28, 23, 59)), 2);
  assert.equal(daysUntil('2027-01-01', new Date(2026, 11, 31, 23, 59)), 1);
  assert.equal(daysUntil('2026-11-15', new Date(2026, 10, 15)), 0);
  const previous = process.env.TZ;
  try {
    process.env.TZ = 'America/New_York';
    assert.equal(daysUntil('2026-03-09', new Date(2026, 2, 8)), 1);
    assert.equal(daysUntil('2026-11-02', new Date(2026, 9, 31)), 2);
  } finally {
    if (previous === undefined) delete process.env.TZ;
    else process.env.TZ = previous;
  }
});

test('日期事件和看板恢复限制容量、修复重复标识并拒绝非法状态流转', () => {
  const events = normalizeEvents([
    { id: 'x', title: '日期', date: '2026-11-15' },
    { id: 'x', title: '重复标识', date: '2026-12-01' },
    { title: '坏日期', date: '2026-02-30' },
  ]);
  assert.equal(events.length, 2);
  assert.notEqual(events[0].id, events[1].id);
  assert.equal(
    normalizeEvents(Array.from({ length: 10 }, () => ({ title: '日子', date: '2026-11-15' })))
      .length,
    3,
  );
  const cards = normalizeCards([
    { id: 'x', title: '任务', status: 'unknown' },
    { id: 'x', title: '任务二', status: 'doing' },
  ]);
  assert.equal(cards[0].status, 'todo');
  assert.notEqual(cards[0].id, cards[1].id);
  assert.equal(moveCard(cards[0], -1), false);
  assert.equal(moveCard(cards[0], 1), true);
  assert.equal(cards[0].status, 'doing');
  assert.equal(moveCard(cards[0], 2), false);
  assert.equal(moveCard({ status: 'unknown' }, 1), false);
});

test('行情过滤按缓存名称和代码，涨幅排序将缺失价放后且不改变自选原序', () => {
  const state = {
    symbols: ['sh000001', 'sz000001', 'sh600519'],
    quotes: [
      { symbol: 'sh000001', name: '上证指数', price: 3000, percent: 1 },
      { symbol: 'sz000001', name: '平安银行', price: 10, percent: 2 },
      { symbol: 'sh600519', name: '贵州茅台', price: null, percent: 99 },
    ],
    sort: 'percent',
  };
  assert.deepEqual(visibleSymbols(state), ['sz000001', 'sh000001', 'sh600519']);
  assert.deepEqual(state.symbols, ['sh000001', 'sz000001', 'sh600519']);
  assert.deepEqual(visibleSymbols({ ...state, filter: '平安' }), ['sz000001']);
  assert.deepEqual(visibleSymbols({ ...state, filter: 'SH000001' }), ['sh000001']);
});
