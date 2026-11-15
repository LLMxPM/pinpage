// 功能：校验日期事件，按本地日历日计算倒数，避免夏令时造成一天偏差。
export const MAX_EVENTS = 3;

/** 只接受真实的 YYYY-MM-DD 日期，拒绝自动回绕及范围外年份。 */
export function parseDateOnly(value) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const [year, month, day] = value.split('-').map(Number);
  if (year < 1900 || year > 2199) return null;
  const stamp = Date.UTC(year, month - 1, day);
  const date = new Date(stamp);
  return date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
    ? stamp / 86400000
    : null;
}

/** 将本地日期序列化，避免 toISOString 将午夜换成前一天。 */
export function localDateKey(date = new Date()) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

/** 倒数按当地日期边界计算，与一天实际经过 23 或 25 小时无关。 */
export function daysUntil(value, now = new Date()) {
  const target = parseDateOnly(value),
    today = parseDateOnly(localDateKey(now));
  return target === null || today === null ? null : target - today;
}

/** 恢复最多三个事件，限制文本、日期和重复标识。 */
export function normalizeEvents(raw) {
  if (!Array.isArray(raw)) return [];
  const ids = new Set();
  return raw
    .filter(
      (event) =>
        typeof event?.title === 'string' &&
        event.title.trim() &&
        parseDateOnly(event.date) !== null,
    )
    .slice(0, MAX_EVENTS)
    .map((event, index) => {
      let id =
        typeof event.id === 'string' && /^[\w-]{1,80}$/.test(event.id)
          ? event.id
          : `event-${index}`;
      if (ids.has(id)) id = `restored-${index}`;
      while (ids.has(id)) id += '-x';
      ids.add(id);
      return { id, title: event.title.trim().slice(0, 60), date: event.date };
    });
}
