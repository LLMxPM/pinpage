// 功能：按选中标的调度分钟行情，限制请求频率、隔离切换竞态并保留失败缓存。
import { fetchTrend } from './trend-provider.js';

/** 创建当前页面专属的分时调度器；仅选中且可见的趋势面板参与刷新。 */
export function createTrendController(state, changed, persist) {
  const attempts = new Map();
  const errors = new Map();
  let loadingSymbol = null;
  let queued = false;

  /** 返回选中标的的加载状态，旧图仍可展示且带缓存说明。 */
  function status() {
    return {
      loading: Boolean(loadingSymbol) && state.selectedSymbol === loadingSymbol,
      error: errors.get(state.selectedSymbol) ?? '',
    };
  }

  /** 普通刷新每只最多每分钟一次，手动刷新至少间隔三秒；同一页面不并发请求。 */
  async function refresh(force = false) {
    const symbol = state.selectedSymbol;
    if (!state.showTrend || !symbol || !state.symbols.includes(symbol)) return;
    if (loadingSymbol) {
      queued = true;
      return;
    }
    const previous = attempts.get(symbol);
    const delay = force
      ? 3000
      : Math.min(300000, 60000 * 2 ** Math.min(previous?.failures ?? 0, 3));
    if (previous && Date.now() - previous.at < delay) return;
    attempts.set(symbol, { at: Date.now(), failures: previous?.failures ?? 0 });
    loadingSymbol = symbol;
    changed();
    try {
      const trend = await fetchTrend(symbol);
      if (!state.symbols.includes(symbol)) return;
      state.trends = state.trends.filter(
        (item) => item.symbol !== symbol && state.symbols.includes(item.symbol),
      );
      state.trends.push(trend);
      attempts.set(symbol, { at: Date.now(), failures: 0 });
      errors.delete(symbol);
      persist();
    } catch {
      attempts.set(symbol, { at: Date.now(), failures: (previous?.failures ?? 0) + 1 });
      errors.set(symbol, navigator.onLine ? '分时更新失败' : '分时网络已断开');
    } finally {
      loadingSymbol = null;
      changed();
      if (queued) {
        queued = false;
        void refresh();
      }
    }
  }
  return { refresh, status };
}
