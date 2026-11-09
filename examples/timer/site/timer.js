// 功能：以截止时间计算倒计时，支持暂停、休眠恢复和页面重载后的状态恢复。
const time = document.querySelector('#time');
const status = document.querySelector('#status');
const toggle = document.querySelector('#toggle');
const storageKey = 'pinpage-timer-v1';
let duration = 25 * 60_000;
let remaining = duration;
let deadline = null;

/// 存储截止时间与剩余时长；存储不可用时计时仍继续运行。
function persist() {
  try {
    localStorage.setItem(storageKey, JSON.stringify({ duration, remaining, deadline }));
  } catch {
    /* 私密模式或存储配额受限时保留内存状态。 */
  }
}

/// 打开记录库；用 IndexedDB 保存结构化的完成记录，演示随页面一起导出的数据。
const openRecords = (() => {
  let opening = null;
  return () => {
    if (!opening) {
      opening = new Promise((resolve, reject) => {
        const request = indexedDB.open('pinpage-timer', 1);
        request.onupgradeneeded = () => {
          const db = request.result;
          if (db.objectStoreNames.contains('sessions')) return;
          const store = db.createObjectStore('sessions', { keyPath: 'id', autoIncrement: true });
          store.createIndex('finishedAt', 'finishedAt');
        };
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
      });
    }
    return opening;
  };
})();

/// 读取记录总数并显示在页脚，失败时保持静默。
async function renderRecords(db) {
  try {
    const transaction = db.transaction('sessions', 'readonly');
    const count = await new Promise((resolve, reject) => {
      const request = transaction.objectStore('sessions').count();
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const target = document.querySelector('#record-count');
    if (target) target.textContent = count ? `已完成 ${count} 次` : '';
  } catch {
    /* 计数失败不影响计时。 */
  }
}

/// 完成一轮时写入一条记录，主键自增，附带完成时间。
async function recordSession(minutes) {
  try {
    const db = await openRecords();
    const transaction = db.transaction('sessions', 'readwrite');
    transaction.objectStore('sessions').add({ minutes, finishedAt: Date.now() });
    await new Promise((resolve, reject) => {
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error);
    });
    await renderRecords(db);
  } catch {
    /* 记录不可用时只丢失统计，不影响计时本身。 */
  }
}

/// 根据真实截止时间刷新，回调只决定显示频率，不累计流逝秒数。
function render() {
  const value = deadline === null ? remaining : Math.max(0, deadline - Date.now());
  const seconds = Math.ceil(value / 1000);
  const display = `${Math.floor(seconds / 60)
    .toString()
    .padStart(2, '0')}:${(seconds % 60).toString().padStart(2, '0')}`;
  if (time.textContent !== display) time.textContent = display;
  const title = `${display} · 专注计时器`;
  if (document.title !== title) document.title = title;
  if (deadline !== null && value === 0) {
    deadline = null;
    remaining = 0;
    persist();
    void recordSession(Math.max(1, Math.round(duration / 60_000)));
  }
  const buttonText = deadline !== null ? '暂停计时' : remaining === 0 ? '再来一次' : '开始专注';
  const statusText =
    deadline !== null
      ? '计时中'
      : remaining === 0
        ? '本轮已完成'
        : remaining < duration
          ? '已暂停'
          : '准备就绪';
  if (toggle.textContent !== buttonText) toggle.textContent = buttonText;
  if (status.textContent !== statusText) status.textContent = statusText;
}

/// 切换暂停和继续，恢复时重新计算截止时间。
function toggleTimer() {
  if (deadline !== null) {
    remaining = Math.max(0, deadline - Date.now());
    deadline = null;
  } else {
    if (remaining <= 0) remaining = duration;
    deadline = Date.now() + remaining;
  }
  persist();
  render();
}

/// 重置当前预设，明确结束已运行的倒计时。
function resetTimer() {
  deadline = null;
  remaining = duration;
  persist();
  render();
}

/// 加载合法的持久化状态，损坏或超出范围的数据使用默认预设。
function restore() {
  try {
    const saved = JSON.parse(localStorage.getItem(storageKey) || 'null');
    if (
      saved &&
      Number.isFinite(saved.duration) &&
      saved.duration > 0 &&
      saved.duration <= 86_400_000 &&
      Number.isFinite(saved.remaining) &&
      saved.remaining >= 0 &&
      saved.remaining <= saved.duration &&
      (saved.deadline === null || Number.isFinite(saved.deadline))
    ) {
      ({ duration, remaining, deadline } = saved);
    }
  } catch {
    /* 状态损坏时使用默认时长。 */
  }
}

/// 用相对路径 fetch 读取预设，作为本地协议资源兼容性样例。
async function loadPresets() {
  try {
    const response = await fetch('./presets.json');
    if (!response.ok) throw new Error('预设读取失败');
    const presets = await response.json();
    for (const preset of presets) {
      const button = document.createElement('button');
      button.textContent = preset.label;
      button.className = duration === preset.minutes * 60_000 ? 'active' : '';
      button.addEventListener('click', () => {
        duration = preset.minutes * 60_000;
        document
          .querySelectorAll('#presets button')
          .forEach((item) => item.classList.remove('active'));
        button.classList.add('active');
        resetTimer();
      });
      document.querySelector('#presets').append(button);
    }
  } catch {
    document.querySelector('#resource-status').textContent = '预设加载失败';
  }
}

restore();
toggle.addEventListener('click', toggleTimer);
document.querySelector('#reset').addEventListener('click', resetTimer);
document.addEventListener('visibilitychange', render);
window.addEventListener('pageshow', render);
setInterval(render, 250);
render();
void loadPresets();
void openRecords()
  .then(renderRecords)
  .catch(() => {});
