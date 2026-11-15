// 功能：校验待办输入并迁移旧版无标识任务，保持跨日清单和稳定任务标识。
export const MAX_TASKS = 100;

/** 恢复合法任务，不解释 HTML，不自动清空跨日数据，并修复重复标识。 */
export function normalizeTasks(raw) {
  if (!Array.isArray(raw)) return [];
  const ids = new Set();
  return raw
    .filter((item) => item && typeof item.text === 'string' && item.text.trim())
    .slice(0, MAX_TASKS)
    .map((item, index) => {
      let id =
        typeof item.id === 'string' && /^[\w-]{1,80}$/.test(item.id) ? item.id : `legacy-${index}`;
      while (ids.has(id)) id = `${id.slice(0, 60)}-${index}`;
      ids.add(id);
      return { id, text: item.text.trim().slice(0, 160), done: item.done === true };
    });
}

/** 新任务标识不依赖安全上下文，兼容本地 WebView 来源。 */
export function newTaskId() {
  return (
    globalThis.crypto?.randomUUID?.() ??
    `task-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
  );
}
