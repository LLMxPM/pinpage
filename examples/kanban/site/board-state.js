// 功能：校验单人本地看板卡片，限制容量、文本及状态枚举，拒绝异常存储。
export const COLUMNS = ['todo', 'doing', 'done'];
export const MAX_CARDS = 100;

/** 恢复合法卡片并修复重复标识，损坏字段不阻断整个看板。 */
export function normalizeCards(raw) {
  if (!Array.isArray(raw)) return [];
  const ids = new Set();
  return raw
    .filter((card) => typeof card?.title === 'string' && card.title.trim())
    .slice(0, MAX_CARDS)
    .map((card, index) => {
      let id =
        typeof card.id === 'string' && /^[\w-]{1,80}$/.test(card.id) ? card.id : `card-${index}`;
      while (ids.has(id)) id = `${id.slice(0, 60)}-${index}`;
      ids.add(id);
      return {
        id,
        title: card.title.trim().slice(0, 160),
        detail: typeof card.detail === 'string' ? card.detail.slice(0, 1000) : '',
        status: COLUMNS.includes(card.status) ? card.status : 'todo',
      };
    });
}

/** 状态流转仅允许已登记的列，不接受越界偏移。 */
export function moveCard(card, offset) {
  if (!COLUMNS.includes(card.status)) return false;
  const target = COLUMNS.indexOf(card.status) + offset;
  if (!Number.isInteger(offset) || Math.abs(offset) !== 1 || target < 0 || target >= COLUMNS.length)
    return false;
  card.status = COLUMNS[target];
  return true;
}
