// 功能：只读取腾讯财经 HTTPS 报价文本，校验代码、编码、数字与时间，不执行远程 JavaScript。
export const SOURCE = '腾讯财经';
export const MAX_SYMBOLS = 12;

/** 规范化显式市场代码；拒绝含义不明确的纯数字及 URL、脚本等输入。 */
export function normalizeSymbol(input) {
  if (typeof input !== 'string') return null;
  const value = input.trim().toLowerCase();
  const prefix = /^(sh|sz|bj)(\d{6})$/.exec(value);
  const suffix = /^(\d{6})\.(sh|sz|bj)$/.exec(value);
  return prefix ? `${prefix[1]}${prefix[2]}` : suffix ? `${suffix[2]}${suffix[1]}` : null;
}

/** 将缺失字段保持为空，不把空串、横线或无效数值变成零。 */
function number(value) {
  if (typeof value !== 'string' || !/^-?\d+(?:\.\d+)?$/.test(value)) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

/** 价格字段中的零通常表示尚未开盘或缺失；涨跌额和涨跌幅仍保留合法零值。 */
function positivePrice(value) {
  const parsed = number(value);
  return parsed !== null && parsed > 0 ? parsed : null;
}

/** 将上游北京时间转换为毫秒；日期回绕、缺失和无效时间均拒绝。 */
export function quoteTimestamp(value) {
  if (!/^\d{14}$/.test(value ?? '')) return null;
  const iso = `${value.slice(0, 4)}-${value.slice(4, 6)}-${value.slice(6, 8)}T${value.slice(8, 10)}:${value.slice(10, 12)}:${value.slice(12, 14)}+08:00`;
  const timestamp = Date.parse(iso);
  if (!Number.isFinite(timestamp)) return null;
  const roundtrip = new Date(timestamp + 8 * 3600000).toISOString().slice(0, 19).replace(/\D/g, '');
  return roundtrip === value ? timestamp : null;
}

/** 解析白名单标的的赋值文本，只接受匹配的六位代码及完整字段。 */
export function parseQuotes(text, symbols) {
  const allowed = new Set(symbols);
  const result = new Map();
  for (const match of text.matchAll(/(?:^|\n)\s*v_((?:sh|sz|bj)\d{6})="([^"\r\n]*)"\s*;/g)) {
    const [, symbol, payload] = match;
    if (!allowed.has(symbol)) continue;
    const fields = payload.split('~');
    if (fields.length < 35 || fields[2] !== symbol.slice(2) || !fields[1]?.trim()) continue;
    const price = number(fields[3]);
    const timestamp = quoteTimestamp(fields[30]);
    if (timestamp === null) continue;
    result.set(symbol, {
      symbol,
      name: fields[1].trim().slice(0, 80),
      price: price !== null && price > 0 ? price : null,
      decimals: Math.min(4, Math.max(2, fields[3]?.split('.')[1]?.length ?? 2)),
      previousClose: positivePrice(fields[4]),
      open: positivePrice(fields[5]),
      change: number(fields[31]),
      percent: number(fields[32]),
      high: positivePrice(fields[33]),
      low: positivePrice(fields[34]),
      timestamp,
    });
  }
  return result;
}

/** 批量请求最多十二个标的；不携带凭据、Referer 伪装或远程脚本，十秒超时。 */
export async function fetchQuotes(symbols) {
  if (
    !Array.isArray(symbols) ||
    !symbols.length ||
    symbols.length > MAX_SYMBOLS ||
    symbols.some((value) => typeof value !== 'string' || normalizeSymbol(value) !== value)
  ) {
    throw new Error('市场代码无效');
  }
  const response = await fetch(`https://qt.gtimg.cn/q=${symbols.join(',')}`, {
    credentials: 'omit',
    cache: 'no-store',
    referrerPolicy: 'no-referrer',
    signal: AbortSignal.timeout(10000),
  });
  if (!response.ok) throw new Error(`行情请求失败（HTTP ${response.status}）`);
  const bytes = await response.arrayBuffer();
  if (bytes.byteLength > 256 * 1024) throw new Error('行情响应异常');
  const result = parseQuotes(new TextDecoder('gb18030', { fatal: true }).decode(bytes), symbols);
  if (!result.size) throw new Error('未返回有效报价，请检查代码或稍后重试');
  return result;
}
