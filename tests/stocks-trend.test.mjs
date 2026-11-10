// 功能：验证真实分时的时间、缺失与缓存边界，以及选中标的请求节流和异步切换竞态。
import assert from 'node:assert/strict';
import test from 'node:test';
import {
  parseTrend,
  sanitizeTrend,
  sessionMinute,
} from '../examples/stocks/site/trend-provider.js';
import { chartGeometry, sessionLabel } from '../examples/stocks/site/chart.js';
import { createTrendController } from '../examples/stocks/site/trend-controller.js';
import { defaults, loadState } from '../examples/stocks/site/store.js';
const NOW = Date.parse('2026-11-11T15:10:00+08:00');

/** 构造供应商形状的响应；分钟数据独立于即时价格，不依赖外网。 */
function response(symbol = 'sh000001', rows = ['0930 10.00 1 10', '0931 10.10 2 20']) {
  return {
    code: 0,
    data: {
      [symbol]: {
        data: { date: '20261111', data: rows },
        qt: { [symbol]: ['', '', '', '', '10.00'] },
      },
    },
  };
}

test('交易分钟保留缺口，午休与无效时刻不混入分钟走势', () => {
  assert.equal(sessionMinute('0930'), 0);
  assert.equal(sessionMinute('1130'), 120);
  assert.equal(sessionMinute('1300'), 120);
  assert.equal(sessionMinute('1500'), 240);
  assert.equal(sessionLabel(120), '11:30 / 13:00');
  for (const value of ['0929', '1131', '1230', '1501', '0960', '2400', '9:30', null])
    assert.equal(sessionMinute(value), null);
});

test('分时按代码和日期匹配，重复分钟更新为最后价格，非法和未来点丢弃', () => {
  const rows = [
    '0932 10.2 3 30',
    '0930 10.0 1 10',
    '0931 10.1 2 20',
    '0931 10.15 2 20',
    '0933 -1 0 0',
    '1200 20 0 0',
    '1300 10.3 0 0',
  ];
  const trend = parseTrend(
    response('sh000001', rows),
    'sh000001',
    Date.parse('2026-11-11T10:00:00+08:00'),
  );
  assert.deepEqual(
    trend.points.map((point) => point.time),
    ['0930', '0931', '0932'],
  );
  assert.equal(trend.points[1].price, 10.15);
  assert.equal(trend.previousClose, 10);
  assert.throws(() => parseTrend(response(), 'sz000001', NOW), /暂无分时/);
  assert.throws(() => parseTrend(response(), null, NOW), /分时响应无效/);
  const invalid = response();
  invalid.data.sh000001.data.date = '20260230';
  assert.throws(() => parseTrend(invalid, 'sh000001', NOW), /日期/);
});

test('空数据、单点和无昨收都保持真实，不补造直线或引用其他日期基线', () => {
  const empty = parseTrend(response('sh000001', []), 'sh000001', NOW);
  assert.equal(chartGeometry(empty), null);
  const single = parseTrend(response('sh000001', ['0930 10 0 0']), 'sh000001', NOW);
  const geometry = chartGeometry(single);
  assert.equal(geometry.points.length, 1);
  assert(geometry.top > geometry.bottom);
  assert(Number.isFinite(geometry.points[0].y));
  const noClose = { ...single, previousClose: null };
  assert.equal(chartGeometry(noClose).baseline, null);
});

test('横轴按已取得分钟范围缩放，中间缺失分钟不压缩成等间隔样本', () => {
  const trend = parseTrend(
    response('sh000001', ['0930 10 0 0', '0931 10.1 0 0', '0940 10.2 0 0']),
    'sh000001',
    NOW,
  );
  const geometry = chartGeometry(trend);
  assert.equal(geometry.endMinute, 10);
  const [a, b, c] = geometry.points;
  assert(Math.abs((c.x - b.x) / (b.x - a.x) - 9) < 1e-9);
  assert.equal(c.x, 597);
});

test('缓存过滤越界日期、价格与不属于当前自选的趋势，兼容旧版偏好', (t) => {
  t.mock.method(Date, 'now', () => NOW);
  assert.equal(
    sanitizeTrend({ symbol: 'sh000001', date: '20260230', points: [] }, 'sh000001', NOW),
    null,
  );
  const clean = sanitizeTrend(
    {
      symbol: 'sh000001',
      date: '20261111',
      previousClose: Infinity,
      points: [
        { time: '0930', price: 1e100 },
        { time: '0931', price: 10 },
      ],
      fetchedAt: NOW + 1,
    },
    'sh000001',
    NOW,
  );
  assert.equal(clean.points.length, 1);
  assert.equal(clean.previousClose, null);
  assert.equal(clean.fetchedAt, 0);
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  let raw = {
    symbols: ['sz000001'],
    theme: 'light',
    surface: 'bare',
    surfaceOpacity: 0,
    selectedSymbol: 'sh000001',
    trends: [parseTrend(response(), 'sh000001', NOW)],
  };
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: { getItem: () => JSON.stringify(raw) },
  });
  try {
    let state = loadState().state;
    assert.equal(state.selectedSymbol, 'sz000001');
    assert.equal(state.trends.length, 0);
    assert.equal(state.surfaceOpacity, 82);
    assert.equal(state.showTrend, true);
    raw = { ...raw, showTrend: false, surfaceOpacity: 35 };
    state = loadState().state;
    assert.equal(state.showTrend, false);
    assert.equal(state.surfaceOpacity, 35);
  } finally {
    if (descriptor) Object.defineProperty(globalThis, 'localStorage', descriptor);
    else delete globalThis.localStorage;
  }
});

test('普通趋势刷新按分钟节流，手动刷新也不突破三秒间隔', async (t) => {
  let now = NOW;
  t.mock.method(Date, 'now', () => now);
  const fetch = t.mock.method(
    globalThis,
    'fetch',
    async () => new Response(JSON.stringify(response())),
  );
  const state = defaults();
  const controller = createTrendController(
    state,
    () => {},
    () => {},
  );
  await controller.refresh();
  await controller.refresh();
  await controller.refresh(true);
  assert.equal(fetch.mock.callCount(), 1);
  now += 3001;
  await controller.refresh(true);
  assert.equal(fetch.mock.callCount(), 2);
  now += 30000;
  await controller.refresh();
  assert.equal(fetch.mock.callCount(), 2);
  now += 30001;
  await controller.refresh();
  assert.equal(fetch.mock.callCount(), 3);
  state.showTrend = false;
  now += 61000;
  await controller.refresh();
  assert.equal(fetch.mock.callCount(), 3);
  state.selectedSymbol = null;
  assert.equal(controller.status().loading, false);
});

test('切换和删除期间的迟到响应不会写回已删除标的或覆盖当前选择', async (t) => {
  t.mock.method(Date, 'now', () => NOW);
  let release;
  const first = new Promise((resolve) => {
    release = resolve;
  });
  let calls = 0;
  t.mock.method(globalThis, 'fetch', async () =>
    ++calls === 1 ? first : new Response(JSON.stringify(response('sz000001'))),
  );
  const state = defaults();
  const controller = createTrendController(
    state,
    () => {},
    () => {},
  );
  const pending = controller.refresh();
  state.symbols = ['sz000001'];
  state.selectedSymbol = 'sz000001';
  await controller.refresh();
  release(new Response(JSON.stringify(response())));
  await pending;
  for (let attempt = 0; attempt < 20 && !state.trends.length; attempt++)
    await new Promise((resolve) => setTimeout(resolve, 1));
  assert.equal(calls, 2);
  assert.equal(state.trends.length, 1);
  assert.equal(state.trends[0].symbol, 'sz000001');
  assert.equal(state.selectedSymbol, 'sz000001');
});
