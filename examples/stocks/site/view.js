// 功能：渲染自选列表、选中行情与管理操作，使用文本节点并保留滚动和键盘焦点。
import { renderChart } from './chart.js';
import { visibleSymbols } from './list-model.js';
const options = {
  timeZone: 'Asia/Shanghai',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
};
const dateFormat = new Intl.DateTimeFormat('zh-CN', {
  ...options,
  month: '2-digit',
  day: '2-digit',
});
const timeFormat = new Intl.DateTimeFormat('zh-CN', options);

/** 按北京时间格式化行情日期；缺失值明确显示尚未取得。 */
export function formatTime(value, date = false) {
  return Number.isFinite(value)
    ? (date ? dateFormat : timeFormat).format(new Date(value))
    : '尚未取得';
}

/** 创建文本节点，上游名称和用户输入不参与 HTML 解析。 */
function node(tag, className, text) {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

/** 格式化有正负号的价格或涨跌幅，缺失数据保留占位符。 */
function signed(value, decimals = 2, suffix = '') {
  return Number.isFinite(value)
    ? `${value > 0 ? '+' : ''}${value.toFixed(decimals)}${suffix}`
    : '—';
}

/** 将方向作为外观标识，未知或无有效报价时保持中性色。 */
function direction(quote) {
  return quote?.price != null && Number.isFinite(quote.percent)
    ? quote.percent > 0
      ? 'up'
      : quote.percent < 0
        ? 'down'
        : 'flat'
    : 'flat';
}

/** 渲染可选择的自选行，刷新保持滚动位置及原有行焦点。 */
export function renderQuotes(state, runtime, select) {
  const container = document.querySelector('#quotes');
  const focused = document.activeElement?.dataset.symbol;
  const scroll = container.scrollTop;
  const fragment = document.createDocumentFragment();
  const quotes = new Map(state.quotes.map((quote) => [quote.symbol, quote]));
  const visible = visibleSymbols(state);
  for (const symbol of visible) {
    const quote = quotes.get(symbol);
    const row = node('button', 'quote', '');
    row.type = 'button';
    row.dataset.symbol = symbol;
    row.setAttribute('aria-pressed', String(state.selectedSymbol === symbol));
    const identity = node('span', 'identity', '');
    const name = node('strong', 'name', quote?.name ?? symbol);
    name.title = quote?.name ?? symbol;
    const stamp = quote ? formatTime(quote.timestamp, true) : '等待报价';
    identity.append(
      name,
      node(
        'span',
        'quote-meta',
        `${symbol.toUpperCase()} · ${stamp}${runtime.missing.has(symbol) ? ' · 未更新' : ''}`,
      ),
    );
    row.append(
      identity,
      node('span', 'price', quote?.price != null ? quote.price.toFixed(quote.decimals) : '—'),
      node(
        'span',
        `percent ${direction(quote)}`,
        quote?.price != null ? signed(quote.percent, 2, '%') : '—',
      ),
    );
    row.addEventListener('click', () => select(symbol));
    fragment.append(row);
  }
  container.replaceChildren(fragment);
  container.scrollTop = scroll;
  if (focused)
    [...container.children]
      .find((row) => row.dataset.symbol === focused)
      ?.focus({ preventScroll: true });
  document.querySelector('#empty').hidden = visible.length > 0;
  document.querySelector('#empty').textContent = state.symbols.length
    ? '没有匹配的自选，可尝试名称或市场代码。'
    : '还没有自选，打开设置添加市场代码。';
  document.querySelector('#watch-count').textContent = state.symbols.length;
}

/** 更新选中行情及真实分钟趋势；图表日期与报价时间分别显示。 */
export function renderSelected(state, trendStatus) {
  const quote = state.quotes.find((item) => item.symbol === state.selectedSymbol);
  document.querySelector('#market-panel').hidden = !state.showTrend || !state.selectedSymbol;
  document.querySelector('#selected-name').textContent =
    quote?.name ?? state.selectedSymbol?.toUpperCase() ?? '暂无自选';
  document.querySelector('#selected-market').textContent = state.selectedSymbol
    ? `A 股 / ${state.selectedSymbol.toUpperCase()}`
    : '添加标的开始观察';
  document.querySelector('#selected-price').textContent =
    quote?.price != null ? quote.price.toFixed(quote.decimals) : '—';
  const changes = document.querySelector('.selected-changes');
  changes.dataset.direction = direction(quote);
  document.querySelector('#selected-change').textContent =
    quote?.price != null ? signed(quote.change, quote.decimals) : '—';
  document.querySelector('#selected-percent').textContent =
    quote?.price != null ? signed(quote.percent, 2, '%') : '—';
  document.querySelector('#selected-time').textContent = quote
    ? `报价 ${formatTime(quote.timestamp, true)}`
    : '等待报价';
  const extra = document.querySelector('#selected-details');
  extra.replaceChildren();
  for (const [label, field] of [
    ['昨收', 'previousClose'],
    ['今开', 'open'],
    ['最高', 'high'],
    ['最低', 'low'],
  ]) {
    const cell = node('div', '', '');
    cell.append(
      node('dt', '', label),
      node('dd', '', Number.isFinite(quote?.[field]) ? quote[field].toFixed(quote.decimals) : '—'),
    );
    extra.append(cell);
  }
  if (state.showTrend)
    renderChart(
      state.trends.find((trend) => trend.symbol === state.selectedSymbol),
      quote,
      trendStatus,
    );
}

/** 按代码构建移动和移除按钮，窄窗口中同样可由键盘操作。 */
export function renderWatchlist(state, onChange) {
  const list = document.querySelector('#watchlist');
  list.replaceChildren();
  for (const [index, symbol] of state.symbols.entries()) {
    const item = node('li', '', '');
    const name = state.quotes.find((quote) => quote.symbol === symbol)?.name ?? symbol;
    item.append(node('span', 'watch-name', `${name} · ${symbol.toUpperCase()}`));
    for (const [label, text, change, disabled] of [
      ['上移', '↑', -1, index === 0],
      ['下移', '↓', 1, index === state.symbols.length - 1],
      ['移除', '×', 0, false],
    ]) {
      const button = node('button', 'mini-button', text);
      button.type = 'button';
      button.setAttribute('aria-label', `${label}${name}`);
      button.disabled = disabled;
      button.addEventListener('click', () => onChange(index, change));
      item.append(button);
    }
    list.append(item);
  }
}
