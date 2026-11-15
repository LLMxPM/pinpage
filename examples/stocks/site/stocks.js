// 功能：协调行情轮询、失败退避、自选编辑和页面恢复，不调用任何 PinPage 宿主权限。
import { fetchQuotes, MAX_SYMBOLS, normalizeSymbol } from './provider.js';
import { loadState, saveState } from './store.js';
import { formatTime, renderQuotes, renderSelected, renderWatchlist } from './view.js';
import { createTrendController } from './trend-controller.js';
import { initializeListControls } from './list-controls.js';

const restored = loadState();
const state = restored.state;
const runtime = {
  missing: new Set(),
  loading: false,
  failures: 0,
  timer: null,
  lastAttempt: 0,
  revision: 0,
};
/** 获取本页固定控件；不接受上游或用户提供的选择器。 */
const $ = (selector) => document.querySelector(selector);
const trends = createTrendController(state, () => renderSelected(state, trends.status()), persist);

/** 同步当前自选和趋势，不因布局变化重新请求网络。 */
function render() {
  document.documentElement.dataset.trend = String(state.showTrend && Boolean(state.selectedSymbol));
  renderQuotes(state, runtime, selectSymbol);
  renderSelected(state, trends.status());
}

/** 切换当前观察标的，图表只请求用户正在关注的一个代码。 */
function selectSymbol(symbol) {
  state.selectedSymbol = symbol;
  persist();
  render();
  void trends.refresh();
}

/** 保存非敏感页面状态，失败时保留内存中的可用状态并明确提醒。 */
function persist() {
  $('#storage-state').textContent = saveState(state)
    ? '自选与外观自动保存在本页。'
    : '本地保存失败，当前设置在关闭后可能丢失。';
}

/** 同步外观和设置控件；透明仅受宿主变量影响，不修改窗口或整体文字透明度。 */
function applyPreferences() {
  document.documentElement.dataset.theme = state.theme;
  document.documentElement.dataset.surface = state.surface;
  document.documentElement.dataset.trend = String(state.showTrend);
  document.documentElement.style.setProperty(
    '--surface-opacity',
    String(state.surfaceOpacity / 100),
  );
  for (const key of ['interval', 'theme', 'surface']) $(`#${key}`).value = String(state[key]);
  $('#surface-opacity').value = state.surfaceOpacity;
  $('#opacity-value').textContent = `${state.surfaceOpacity}%`;
  $('#opacity-row').hidden = state.surface === 'bare';
  $('#show-trend').checked = state.showTrend;
}

/** 展示缓存、错误或部分成功信息，始终保留每只标的的原始行情时间。 */
function notice(text = '') {
  $('#notice').textContent = text;
  $('#notice').hidden = !text;
}

/** 请求时间独立于上游行情时间，刷新偏好变化后立即同步说明。 */
function renderRequestState() {
  $('#request-state').textContent =
    `最近请求成功 ${formatTime(state.requestedAt, true)} · ${state.interval ? `${state.interval} 秒刷新` : '仅手动刷新'}${runtime.failures && state.interval ? ' · 失败后降低频率' : ''}`;
}

/** 定时器仅驱动刷新，失败后最多退避五分钟；不并发请求，不依据星期推断交易日。 */
function schedule() {
  clearTimeout(runtime.timer);
  if (!state.interval || !state.symbols.length) return;
  if (!navigator.onLine) return;
  const delay = Math.max(
    document.visibilityState === 'visible' ? 0 : 60,
    Math.min(300, state.interval * 2 ** Math.min(runtime.failures, 5)),
  );
  runtime.timer = setTimeout(refresh, delay * 1000);
}

/** 获取当前自选快照；编辑期间返回的旧请求不会覆盖已删除或新加入的标的。 */
async function refresh(manual = false) {
  if (runtime.loading) return;
  clearTimeout(runtime.timer);
  if (!state.symbols.length) {
    $('#connection-state').textContent = '暂无自选';
    return;
  }
  if (Date.now() - runtime.lastAttempt < 3000) {
    schedule();
    return;
  }
  runtime.lastAttempt = Date.now();
  runtime.loading = true;
  $('#refresh').disabled = true;
  $('#connection-state').textContent = '正在更新';
  const symbols = [...state.symbols];
  const revision = runtime.revision;
  try {
    const fetched = await fetchQuotes(symbols);
    if (revision !== runtime.revision) return;
    runtime.failures = 0;
    runtime.missing = new Set(symbols.filter((symbol) => !fetched.has(symbol)));
    const previous = new Map(state.quotes.map((quote) => [quote.symbol, quote]));
    state.quotes = symbols
      .map((symbol) => fetched.get(symbol) ?? previous.get(symbol))
      .filter(Boolean);
    state.requestedAt = Date.now();
    $('#connection-state').textContent = runtime.missing.size ? '部分报价已更新' : '请求成功';
    const olderDay = state.quotes.some(
      (quote) =>
        new Date(quote.timestamp + 8 * 3600000).toISOString().slice(0, 10) !==
        new Date(Date.now() + 8 * 3600000).toISOString().slice(0, 10),
    );
    notice(
      runtime.missing.size
        ? `${runtime.missing.size} 个标的本轮未返回，旧报价保留；可在设置中检查代码。`
        : olderDay
          ? '含非当日报价，请以各行行情时间为准。'
          : '',
    );
    persist();
  } catch (error) {
    if (revision !== runtime.revision) return;
    runtime.failures += 1;
    $('#connection-state').textContent = '更新失败';
    notice(
      `${navigator.onLine ? (error.name === 'TimeoutError' ? '行情请求超时' : '行情请求失败，接口可能不可用或被限流') : '网络已断开'}。${state.quotes.length ? '保留上次报价，请留意行情时间。' : '稍后重试。'}`,
    );
  } finally {
    runtime.loading = false;
    $('#refresh').disabled = false;
    render();
    void trends.refresh(manual);
    renderRequestState();
    if ($('#settings-dialog').open) renderWatchlist(state, changeWatchlist);
    if (revision !== runtime.revision) {
      clearTimeout(runtime.timer);
      runtime.timer = setTimeout(refresh, 3000);
    } else schedule();
  }
}

/** 调整或删除一条自选；及时落盘，不依赖关闭事件。 */
function changeWatchlist(index, movement) {
  if (movement)
    [state.symbols[index], state.symbols[index + movement]] = [
      state.symbols[index + movement],
      state.symbols[index],
    ];
  else state.symbols.splice(index, 1);
  state.quotes = state.quotes.filter((quote) => state.symbols.includes(quote.symbol));
  state.trends = state.trends.filter((trend) => state.symbols.includes(trend.symbol));
  if (!state.symbols.includes(state.selectedSymbol))
    state.selectedSymbol = state.symbols[0] ?? null;
  runtime.revision += 1;
  persist();
  render();
  void trends.refresh();
  renderWatchlist(state, changeWatchlist);
  if (!state.symbols.length) {
    $('#connection-state').textContent = '暂无自选';
    notice();
  }
  schedule();
}

/** 添加显式市场代码，真实报价由下一轮验证；重复、越界或不明确的输入直接拒绝。 */
function addSymbol(event) {
  event.preventDefault();
  const symbol = normalizeSymbol($('#symbol').value);
  if (!symbol) {
    $('#settings-message').textContent = '请填写明确的市场代码，如 sh600519 或 600519.SH。';
    return;
  }
  if (state.symbols.includes(symbol)) {
    $('#settings-message').textContent = '这个标的已经在自选中。';
    return;
  }
  if (state.symbols.length >= MAX_SYMBOLS) {
    $('#settings-message').textContent = `最多添加 ${MAX_SYMBOLS} 个标的。`;
    return;
  }
  state.symbols.push(symbol);
  if (!state.selectedSymbol) state.selectedSymbol = symbol;
  runtime.revision += 1;
  $('#symbol').value = '';
  $('#settings-message').textContent = '已添加，等待接口返回有效报价。';
  persist();
  render();
  renderWatchlist(state, changeWatchlist);
  clearTimeout(runtime.timer);
  runtime.timer = setTimeout(refresh, Math.max(0, 3000 - (Date.now() - runtime.lastAttempt)));
}

$('#refresh').addEventListener('click', () => refresh(true));
$('#settings').addEventListener('click', () => {
  renderWatchlist(state, changeWatchlist);
  $('#settings-dialog').showModal();
});
$('#close-settings').addEventListener('click', () => $('#settings-dialog').close());
$('#add-symbol').addEventListener('submit', addSymbol);
for (const key of ['interval', 'theme', 'surface']) {
  $(`#${key}`).addEventListener('change', (event) => {
    state[key] = key === 'interval' ? Number(event.target.value) : event.target.value;
    applyPreferences();
    render();
    persist();
    renderRequestState();
    schedule();
  });
}
$('#surface-opacity').addEventListener('input', (event) => {
  state.surfaceOpacity = Number(event.target.value);
  applyPreferences();
  persist();
});
$('#show-trend').addEventListener('change', (event) => {
  state.showTrend = event.target.checked;
  applyPreferences();
  persist();
  render();
  void trends.refresh();
});
/** 提示当前自适应布局；窗口实际尺寸由宿主控制，页面不调用窗口 IPC。 */
function updateLayout() {
  const layout =
    innerWidth >= 640 ? '宽屏' : innerWidth <= 340 || innerHeight <= 500 ? '紧凑' : '标准';
  $('#layout-label').textContent = `${layout}布局`;
}
window.addEventListener('resize', updateLayout);
// 恢复可见、联网或休眠返回时校正，不累积补发已错过的请求。
window.addEventListener('offline', () => {
  clearTimeout(runtime.timer);
  $('#connection-state').textContent = '离线缓存';
  notice('网络已断开，保留上次行情与趋势。');
});
window.addEventListener('online', () => {
  if (state.interval) void refresh();
});
window.addEventListener('pageshow', () => refresh());
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible' && state.interval) refresh();
  else schedule();
});
initializeListControls(state, render, persist);
applyPreferences();
render();
updateLayout();
renderRequestState();
if (!restored.available) $('#storage-state').textContent = '原有存储不可用或损坏，已使用默认配置。';
if (state.quotes.length) notice('正在显示本地缓存，连接后更新。');
void refresh();
