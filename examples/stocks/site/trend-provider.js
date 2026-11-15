// 功能：获取并校验腾讯当日分钟价格，保留交易时间位置，不补造缺失或未来的分时点。
import { normalizeSymbol, quoteTimestamp } from './provider.js';

/** 映射 A 股日间交易分钟，压缩午休但保持缺失分钟在横轴上的实际位置。 */
export function sessionMinute(time) {
  if (typeof time !== 'string' || !/^\d{4}$/.test(time)) return null;
  const hour = Number(time.slice(0, 2));
  const minute = Number(time.slice(2));
  if (minute > 59) return null;
  const total = hour * 60 + minute;
  if (total >= 570 && total <= 690) return total - 570;
  if (total >= 780 && total <= 900) return 120 + total - 780;
  return null;
}

/** 清理网络或本地缓存的趋势数据，限制日期、点数、价格与可用时间。 */
export function sanitizeTrend(input, symbol, now = Date.now()) {
  if (
    typeof symbol !== 'string' ||
    normalizeSymbol(symbol) !== symbol ||
    input?.symbol !== symbol ||
    typeof input.date !== 'string' ||
    !/^20\d{6}$/.test(input.date) ||
    quoteTimestamp(`${input.date}000000`) === null ||
    !Array.isArray(input.points) ||
    input.points.length > 500
  )
    return null;
  const points = new Map();
  for (const point of input.points) {
    const minute = sessionMinute(point?.time);
    const timestamp = minute === null ? null : quoteTimestamp(`${input.date}${point.time}00`);
    if (
      timestamp === null ||
      timestamp > now ||
      !Number.isFinite(point.price) ||
      point.price <= 0 ||
      point.price > 1e9
    )
      continue;
    points.set(point.time, { time: point.time, minute, price: point.price });
  }
  return {
    symbol,
    date: input.date,
    previousClose:
      Number.isFinite(input.previousClose) && input.previousClose > 0 && input.previousClose <= 1e9
        ? input.previousClose
        : null,
    points: [...points.values()].sort((a, b) => a.time.localeCompare(b.time)).slice(0, 242),
    fetchedAt:
      Number.isFinite(input.fetchedAt) && input.fetchedAt > 0 && input.fetchedAt <= now
        ? input.fetchedAt
        : 0,
  };
}

/** 解析供应商返回的分钟序列；只接受请求对应的代码，空分时保留为空而非伪造直线。 */
export function parseTrend(payload, symbol, now = Date.now()) {
  if (payload?.code !== 0 || typeof symbol !== 'string' || normalizeSymbol(symbol) !== symbol)
    throw new Error('分时响应无效');
  const data = payload.data?.[symbol];
  if (!data?.data || !Array.isArray(data.data.data)) throw new Error('暂无分时数据');
  const rawClose = data.qt?.[symbol]?.[4];
  const previousClose =
    typeof rawClose === 'string' && /^\d+(?:\.\d+)?$/.test(rawClose) ? Number(rawClose) : null;
  const trend = sanitizeTrend(
    {
      symbol,
      date: data.data.date,
      previousClose,
      fetchedAt: now,
      points: data.data.data.map((row) => {
        const fields = typeof row === 'string' ? row.trim().split(/\s+/) : [];
        return {
          time: fields[0],
          price: /^\d+(?:\.\d+)?$/.test(fields[1] ?? '') ? Number(fields[1]) : null,
        };
      }),
    },
    symbol,
    now,
  );
  if (!trend) throw new Error('分时日期或数据无效');
  return trend;
}

/** 只拉取当前关注的一只标的，不携带凭据或执行远程脚本，十秒超时。 */
export async function fetchTrend(symbol) {
  if (typeof symbol !== 'string' || normalizeSymbol(symbol) !== symbol)
    throw new Error('市场代码无效');
  const response = await fetch(
    `https://web.ifzq.gtimg.cn/appstock/app/minute/query?code=${symbol}`,
    {
      credentials: 'omit',
      cache: 'no-store',
      referrerPolicy: 'no-referrer',
      signal: AbortSignal.timeout(10000),
    },
  );
  if (!response.ok) throw new Error(`分时请求失败（HTTP ${response.status}）`);
  const text = await response.text();
  if (text.length > 256 * 1024) throw new Error('分时响应过大');
  return parseTrend(JSON.parse(text), symbol);
}
