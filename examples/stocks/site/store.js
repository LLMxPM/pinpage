// 功能：保存自选、外观与最近行情，恢复时限制数量并验证外部存储字段。
import { MAX_SYMBOLS, normalizeSymbol } from './provider.js';
import { sanitizeTrend } from './trend-provider.js';

const KEY = 'pinpage-stocks-v1';
export const DEFAULT_SYMBOLS = [
  'sh000001',
  'sz399001',
  'sz399006',
  'sh600519',
  'sz000001',
  'sh510300',
];

/** 生成独立默认状态，避免多个页面或重置操作共享可变数组。 */
export function defaults() {
  return {
    symbols: [...DEFAULT_SYMBOLS],
    interval: 30,
    theme: 'dark',
    surface: 'card',
    surfaceOpacity: 82,
    showTrend: true,
    selectedSymbol: DEFAULT_SYMBOLS[0],
    trends: [],
    quotes: [],
    requestedAt: null,
  };
}

/** 恢复非敏感偏好和缓存；损坏存储退回默认值，缓存绝不当成新请求结果。 */
export function loadState() {
  try {
    const text = localStorage.getItem(KEY);
    if (!text) return { state: defaults(), available: true };
    if (text.length > 512 * 1024) throw new Error('存储过大');
    const raw = JSON.parse(text);
    const state = defaults();
    if (Array.isArray(raw.symbols))
      state.symbols = [...new Set(raw.symbols.map(normalizeSymbol).filter(Boolean))].slice(
        0,
        MAX_SYMBOLS,
      );
    if ([0, 15, 30, 60].includes(raw.interval)) state.interval = raw.interval;
    if (['light', 'dark'].includes(raw.theme)) state.theme = raw.theme;
    if (['card', 'bare'].includes(raw.surface)) state.surface = raw.surface;
    if (
      Number.isInteger(raw.surfaceOpacity) &&
      raw.surfaceOpacity >= 25 &&
      raw.surfaceOpacity <= 100
    )
      state.surfaceOpacity = raw.surfaceOpacity;
    if (typeof raw.showTrend === 'boolean') state.showTrend = raw.showTrend;
    state.selectedSymbol = state.symbols.includes(raw.selectedSymbol)
      ? raw.selectedSymbol
      : (state.symbols[0] ?? null);
    state.trends = Array.isArray(raw.trends)
      ? raw.trends
          .slice(0, MAX_SYMBOLS)
          .map((trend) =>
            state.symbols.includes(trend?.symbol) ? sanitizeTrend(trend, trend.symbol) : null,
          )
          .filter(Boolean)
      : [];
    state.quotes = Array.isArray(raw.quotes)
      ? raw.quotes.filter((quote) => validQuote(quote, state.symbols)).slice(0, MAX_SYMBOLS)
      : [];
    state.requestedAt = validTimestamp(raw.requestedAt) ? raw.requestedAt : null;
    return { state, available: true };
  } catch {
    return { state: defaults(), available: false };
  }
}

/** 验证缓存字段类型与边界，拒绝伪造代码、无限数值和异常文本。 */
function validQuote(quote, symbols) {
  return (
    quote &&
    symbols.includes(quote.symbol) &&
    typeof quote.name === 'string' &&
    quote.name.length > 0 &&
    quote.name.length <= 80 &&
    validTimestamp(quote.timestamp) &&
    Number.isInteger(quote.decimals) &&
    quote.decimals >= 2 &&
    quote.decimals <= 4 &&
    ['price', 'previousClose', 'open', 'change', 'percent', 'high', 'low'].every(
      (field) => quote[field] === null || Number.isFinite(quote[field]),
    ) &&
    (quote.price === null || quote.price > 0)
  );
}

/** 限制可显示日期范围，防止损坏缓存中的巨大数值使日期格式化抛错。 */
function validTimestamp(value) {
  return Number.isFinite(value) && value >= Date.UTC(2000, 0, 1) && value < Date.UTC(2100, 0, 1);
}

/** 同步保存小体积状态；调用方负责展示存储失败，当前会话仍可继续使用。 */
export function saveState(state) {
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
    return true;
  } catch {
    return false;
  }
}
