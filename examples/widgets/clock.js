// 功能：基于系统当前时间显示时钟，后台暂停后可直接校正，记忆显示秒数的偏好。
const time = document.querySelector('#clock-time');
const date = document.querySelector('#clock-date');
const weekday = document.querySelector('#clock-weekday');
const seconds = document.querySelector('#show-seconds');
const storageKey = 'pinpage-clock-seconds-v1';
const dateFormat = new Intl.DateTimeFormat('zh-CN', {
  year: 'numeric',
  month: 'long',
  day: 'numeric',
});
const weekdayFormat = new Intl.DateTimeFormat('zh-CN', { weekday: 'long' });
const minuteFormat = new Intl.DateTimeFormat('zh-CN', {
  hour: '2-digit',
  minute: '2-digit',
  hourCycle: 'h23',
});
const secondFormat = new Intl.DateTimeFormat('zh-CN', {
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hourCycle: 'h23',
});

/// 内容不变时不重复更新 DOM，减少持续可见小工具的无效绘制。
function update(target, value) {
  if (target.textContent !== value) target.textContent = value;
}

/// 每轮读取系统时间，避免用累加计数导致休眠恢复后时间漂移。
function render() {
  const now = new Date();
  update(time, (seconds.checked ? secondFormat : minuteFormat).format(now));
  update(date, dateFormat.format(now));
  update(weekday, weekdayFormat.format(now));
}

try {
  seconds.checked = localStorage.getItem(storageKey) === 'true';
} catch {
  // 存储被禁用时仍可查看时间和临时切换显示。
}
seconds.addEventListener('change', () => {
  try {
    localStorage.setItem(storageKey, String(seconds.checked));
  } catch {
    // 偏好保存失败不影响时钟功能。
  }
  render();
});
document.addEventListener('visibilitychange', render);
render();
setInterval(render, 1000);
