// 功能：渲染月历和倒数日，管理日期增改、撤销删除及跨日恢复。
import {
  MAX_EVENTS,
  normalizeEvents,
  localDateKey,
  daysUntil,
  parseDateOnly,
} from './date-state.js';

/** 获取固定控件。 */
const $ = (id) => document.getElementById(id);
const KEY = 'pinpage-calendar-v1';
let events = [],
  theme = 'light',
  editing = null,
  deleted = null;
let displayed = new Date(),
  todayKey = '',
  timer;
try {
  const saved = JSON.parse(localStorage.getItem(KEY) || '{}');
  events = normalizeEvents(saved?.events);
  if (saved?.theme === 'dark') theme = 'dark';
} catch {
  $('save-status').textContent = '无法读取原有日期，可继续使用';
}

/** 独立保存事件与配色，落盘失败保留当前会话并提示备份。 */
function save() {
  try {
    localStorage.setItem(KEY, JSON.stringify({ events, theme }));
    $('save-status').textContent = '已保存在本地';
  } catch {
    $('save-status').textContent = '保存失败，关闭后可能丢失当前日期';
  }
}

/** 用文本节点构造控件，日期标题不作为 HTML。 */
function node(tag, className, text) {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

/** 在容量内新建或编辑日期，点击月历只预填日期，不自动覆盖事件。 */
function edit(event = null, date = localDateKey()) {
  if (!event && events.length >= MAX_EVENTS) return;
  editing = event?.id ?? null;
  $('event-title').value = event?.title ?? '';
  $('event-date').value = event?.date ?? date;
  $('event-date').setCustomValidity('');
  $('event-dialog').showModal();
  $('event-title').focus();
}

/** 月历以周一开始，边界月份禁用越界导航，选择日期可快速录入。 */
function renderMonth() {
  const year = displayed.getFullYear(),
    month = displayed.getMonth();
  $('month-label').textContent = `${year} 年 ${month + 1} 月`;
  $('previous-month').disabled = year === 1900 && month === 0;
  $('next-month').disabled = year === 2199 && month === 11;
  const first = new Date(year, month, 1);
  const start = new Date(year, month, 1 - ((first.getDay() + 6) % 7));
  const total =
    Math.ceil((((first.getDay() + 6) % 7) + new Date(year, month + 1, 0).getDate()) / 7) * 7;
  const fragment = document.createDocumentFragment();
  for (let index = 0; index < total; index += 1) {
    const date = new Date(start.getFullYear(), start.getMonth(), start.getDate() + index);
    const key = localDateKey(date),
      matches = events.filter((event) => event.date === key);
    const button = node(
      'button',
      `day${date.getMonth() !== month ? ' outside' : ''}${key === localDateKey() ? ' is-today' : ''}${matches.length ? ' has-event' : ''}`,
      date.getDate(),
    );
    button.type = 'button';
    button.disabled = parseDateOnly(key) === null || events.length >= MAX_EVENTS;
    button.setAttribute(
      'aria-label',
      `${key}${matches.length ? `，${matches.map((event) => event.title).join('、')}` : ''}`,
    );
    if (key === localDateKey()) button.setAttribute('aria-current', 'date');
    button.addEventListener('click', () => edit(null, key));
    fragment.append(button);
  }
  $('days').replaceChildren(fragment);
}

/** 未来日期优先，过往日期显示已经过天数，今日不显示负零。 */
function renderEvents() {
  $('events').replaceChildren();
  const ordered = [...events].sort((a, b) => {
    const left = daysUntil(a.date),
      right = daysUntil(b.date);
    return (left < 0) - (right < 0) || (left < 0 ? right - left : left - right);
  });
  for (const event of ordered) {
    const days = daysUntil(event.date);
    const row = node('li', 'event', '');
    const identity = node('div', 'event-identity', '');
    identity.append(node('strong', '', event.title), node('span', 'hint', event.date));
    const count = node(
      'strong',
      `count${days < 0 ? ' past' : ''}`,
      days === 0 ? '就是今天' : `${days > 0 ? '还有' : '已过'} ${Math.abs(days)} 天`,
    );
    const actions = node('div', 'event-actions', '');
    for (const [label, text, action] of [
      ['编辑', '编辑', () => edit(event)],
      [
        '删除',
        '×',
        () => {
          deleted = { ...event };
          events = events.filter((item) => item.id !== event.id);
          save();
          render();
        },
      ],
    ]) {
      const button = node('button', '', text);
      button.setAttribute('aria-label', `${label}：${event.title}`);
      button.addEventListener('click', action);
      actions.append(button);
    }
    row.append(identity, count, actions);
    $('events').append(row);
  }
  $('empty').hidden = events.length > 0;
  $('add-event').disabled = events.length >= MAX_EVENTS;
  $('undo').hidden = !deleted;
  $('undo').disabled = events.length >= MAX_EVENTS;
}

/** 跨午夜自动重绘；仅每分钟校正一次，恢复可见时立即更新。 */
function render() {
  const now = new Date();
  $('today-label').textContent = new Intl.DateTimeFormat('zh-CN', {
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  }).format(now);
  document.documentElement.dataset.theme = theme;
  renderMonth();
  renderEvents();
  todayKey = localDateKey(now);
}
/** 跨日校正并对齐下一分钟，页面恢复时也复用同一调度器。 */
function tick() {
  clearTimeout(timer);
  if (todayKey !== localDateKey()) {
    displayed = new Date();
    render();
  }
  timer = setTimeout(tick, 60000 - (Date.now() % 60000) + 30);
}
$('event-form').addEventListener('submit', (event) => {
  event.preventDefault();
  const title = $('event-title').value.trim(),
    date = $('event-date').value;
  if (!title) {
    $('event-title').setCustomValidity('请输入日期名称');
    $('event-title').reportValidity();
    return;
  }
  if (parseDateOnly(date) === null) return;
  const existing = events.find((item) => item.id === editing);
  if (existing) Object.assign(existing, { title, date });
  else if (events.length < MAX_EVENTS)
    events.push({ id: globalThis.crypto?.randomUUID?.() ?? `date-${Date.now()}`, title, date });
  save();
  render();
  $('event-dialog').close();
});
$('event-title').addEventListener('input', () => $('event-title').setCustomValidity(''));
$('add-event').addEventListener('click', () => edit());
$('close-dialog').addEventListener('click', () => $('event-dialog').close());
$('theme').addEventListener('click', () => {
  theme = theme === 'light' ? 'dark' : 'light';
  save();
  render();
});
$('today').addEventListener('click', () => {
  displayed = new Date();
  render();
});
for (const [id, offset] of [
  ['previous-month', -1],
  ['next-month', 1],
])
  $(id).addEventListener('click', () => {
    displayed = new Date(displayed.getFullYear(), displayed.getMonth() + offset, 1);
    render();
  });
$('undo').addEventListener('click', () => {
  if (deleted && events.length < MAX_EVENTS) {
    events.push(deleted);
    deleted = null;
    save();
    render();
  }
});
document.addEventListener('visibilitychange', tick);
window.addEventListener('pageshow', tick);
render();
tick();
