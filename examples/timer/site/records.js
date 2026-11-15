// 功能：复用旧计时器 IndexedDB，分开记录专注和休息，失败时保留计时功能。
let opening;

/** 继续使用原表与主键，不删除或覆盖旧记录。 */
function openRecords() {
  if (!opening)
    opening = new Promise((resolve, reject) => {
      const request = indexedDB.open('pinpage-timer', 1);
      request.onupgradeneeded = () => {
        const store = request.result.createObjectStore('sessions', {
          keyPath: 'id',
          autoIncrement: true,
        });
        store.createIndex('finishedAt', 'finishedAt');
      };
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  return opening;
}

/** 展示今日专注轮数；旧版无类型记录保留，但不混入新统计。 */
export async function renderRecords() {
  const target = document.querySelector('#record-count');
  try {
    const db = await openRecords();
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const records = await new Promise((resolve, reject) => {
      const request = db
        .transaction('sessions')
        .objectStore('sessions')
        .index('finishedAt')
        .getAll(IDBKeyRange.lowerBound(start.getTime()));
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const focus = records.filter((record) => record.kind === 'focus');
    target.textContent = focus.length
      ? `今日专注 ${focus.length} 轮 · ${Math.round(focus.reduce((sum, item) => sum + item.minutes, 0))} 分钟`
      : '从一轮专注开始';
  } catch {
    target.textContent = '完成记录暂不可用，计时仍可使用';
  }
}

/** 保存带类型的一轮完成记录，休息不会增加专注次数。 */
export async function recordSession(state, finishedAt) {
  try {
    const db = await openRecords();
    await new Promise((resolve, reject) => {
      const transaction = db.transaction('sessions', 'readwrite');
      transaction
        .objectStore('sessions')
        .add({ kind: state.kind, minutes: state.duration / 60000, finishedAt });
      transaction.oncomplete = resolve;
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error);
    });
    await renderRecords();
  } catch {
    document.querySelector('#record-count').textContent = '完成记录保存失败，计时仍可使用';
  }
}
