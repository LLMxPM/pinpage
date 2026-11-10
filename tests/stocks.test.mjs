// 功能：验证行情解析的市场歧义、精度、缺失字段、时间与恶意响应边界，以及缓存恢复约束。
import assert from 'node:assert/strict';
import test from 'node:test';
import {
  fetchQuotes,
  normalizeSymbol,
  parseQuotes,
  quoteTimestamp,
} from '../examples/stocks/site/provider.js';
import { defaults, loadState, saveState } from '../examples/stocks/site/store.js';

/** 生成最小报价报文；故意不提供成交量等无关字段，以验证必要边界。 */
function line(symbol, overrides = {}) {
  const fields = Array(35).fill('');
  Object.assign(
    fields,
    {
      1: '测试标的',
      2: symbol.slice(2),
      3: '12.34',
      4: '12.00',
      5: '12.20',
      30: '20261111101530',
      31: '0.34',
      32: '2.83',
      33: '12.50',
      34: '12.10',
    },
    overrides,
  );
  return `v_${symbol}="${fields.join('~')}";`;
}

test('市场代码必须明确，指数与同代码股票不会混淆', () => {
  assert.equal(normalizeSymbol(' 600519.SH '), 'sh600519');
  assert.equal(normalizeSymbol('SZ000001'), 'sz000001');
  assert.equal(normalizeSymbol('bj920001'), 'bj920001');
  for (const input of [
    '000001',
    'sh000001,sz000001',
    '../sh600519',
    'https://example.com',
    null,
    {},
    'sh12345',
  ])
    assert.equal(normalizeSymbol(input), null);
  const quotes = parseQuotes(
    `${line('sh000001', { 1: '上证指数' })}\n${line('sz000001', { 1: '平安银行' })}`,
    ['sh000001', 'sz000001'],
  );
  assert.equal(quotes.get('sh000001').name, '上证指数');
  assert.equal(quotes.get('sz000001').name, '平安银行');
});

test('保留 ETF 三位精度和涨跌零值，缺失报价不伪造为零', () => {
  const quotes = parseQuotes(
    `${line('sh510300', { 3: '4.135', 31: '0.000', 32: '0.00' })}\n${line('sz000001', { 3: '-', 31: '', 32: '-' })}`,
    ['sh510300', 'sz000001'],
  );
  assert.equal(quotes.get('sh510300').price, 4.135);
  assert.equal(quotes.get('sh510300').decimals, 3);
  assert.equal(quotes.get('sh510300').percent, 0);
  assert.equal(quotes.get('sz000001').price, null);
  assert.equal(quotes.get('sz000001').change, null);
  assert.equal(quotes.get('sz000001').percent, null);
});

test('北京时间无隐含本机时区，非法日期及回绕时间被拒绝', () => {
  assert.equal(quoteTimestamp('20261111101530'), Date.parse('2026-11-11T02:15:30Z'));
  for (const value of ['20260230101530', '20261111240000', '20261111106000', '20261111', '', null])
    assert.equal(quoteTimestamp(value), null);
});

test('盘前未形成的开高低价格保持缺失，零涨跌仍有效', () => {
  const quote = parseQuotes(
    line('sh600519', { 5: '0.00', 33: '0.00', 34: '0.00', 31: '0.00', 32: '0.00' }),
    ['sh600519'],
  ).get('sh600519');
  assert.equal(quote.open, null);
  assert.equal(quote.high, null);
  assert.equal(quote.low, null);
  assert.equal(quote.percent, 0);
});

test('缺失、串号和无效时间独立丢弃，其他标的仍可返回', () => {
  const text = `${line('sh600519')}\n${line('sz000001', { 2: '600519' })}\n${line('sh510300', { 30: '' })}\nv_bj920001="";`;
  assert.deepEqual(
    [...parseQuotes(text, ['sh600519', 'sz000001', 'sh510300', 'bj920001']).keys()],
    ['sh600519'],
  );
  assert.equal(parseQuotes(line('sh600519'), ['sz000001']).size, 0);
});

test('接口只做文本解析，响应中的脚本不会执行', () => {
  delete globalThis.stockInjected;
  const text = `globalThis.stockInjected = true;\n${line('sh600519', { 1: '<img src=x onerror=alert(1)>' })}`;
  const quote = parseQuotes(text, ['sh600519']).get('sh600519');
  assert.equal(globalThis.stockInjected, undefined);
  assert.equal(quote.name, '<img src=x onerror=alert(1)>');
});

test('非法请求在发送前拒绝，HTTP 失败与非行情响应不会标记成功', async (t) => {
  const mock = t.mock.method(globalThis, 'fetch', async () => new Response('', { status: 429 }));
  await assert.rejects(fetchQuotes(['sh600519,sz000001']), /市场代码无效/);
  await assert.rejects(fetchQuotes([null]), /市场代码无效/);
  assert.equal(mock.mock.callCount(), 0);
  await assert.rejects(fetchQuotes(['sh600519']), /HTTP 429/);
  assert.equal(mock.mock.calls[0].arguments[1].credentials, 'omit');
  mock.mock.mockImplementation(async () => new Response('<html>限流</html>'));
  await assert.rejects(fetchQuotes(['sh600519']), /未返回有效报价/);
});

test('缓存恢复限制数量、类型与标的范围，损坏或不可写存储可降级', () => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  let stored = JSON.stringify({
    symbols: ['sh000001', 'sz000001', 'sh000001', '../bad'],
    interval: 1,
    theme: 'unknown',
    quotes: [{ name: '污染数据', symbol: 'sh000001', price: '1' }],
  });
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: () => stored,
      setItem: (_key, value) => {
        stored = value;
      },
    },
  });
  try {
    const restored = loadState().state;
    assert.deepEqual(restored.symbols, ['sh000001', 'sz000001']);
    assert.equal(restored.interval, 30);
    assert.equal(restored.theme, 'dark');
    assert.deepEqual(restored.quotes, []);
    const state = defaults();
    state.quotes = [...parseQuotes(line('sh000001'), state.symbols).values()];
    assert.equal(saveState(state), true);
    assert.equal(loadState().state.quotes.length, 1);
    stored = JSON.stringify({
      ...state,
      requestedAt: 1e100,
      quotes: [{ ...state.quotes[0], timestamp: 1e100 }],
    });
    assert.equal(loadState().state.requestedAt, null);
    assert.equal(loadState().state.quotes.length, 0);
    stored = '{bad';
    assert.equal(loadState().available, false);
    globalThis.localStorage.setItem = () => {
      throw new Error('存储不可写');
    };
    assert.equal(saveState(state), false);
  } finally {
    if (descriptor) Object.defineProperty(globalThis, 'localStorage', descriptor);
    else delete globalThis.localStorage;
  }
});
