// 功能：管理看板编辑、状态流转、同列排序、批量清理与撤销，所有修改即时保存。
import { COLUMNS, MAX_CARDS, normalizeCards, moveCard } from './board-state.js';

/** 查找固定页面控件。 */
const $ = (id) => document.getElementById(id);
const KEY = 'pinpage-kanban-v1';
let cards = [],
  title = '我的项目',
  theme = 'light',
  column = 'todo',
  editing = null,
  deleted = null;
try {
  const saved = JSON.parse(localStorage.getItem(KEY) || '{}');
  cards = normalizeCards(saved?.cards);
  if (typeof saved?.title === 'string' && saved.title.trim()) title = saved.title.slice(0, 40);
  if (saved?.theme === 'dark') theme = 'dark';
  if (COLUMNS.includes(saved?.column)) column = saved.column;
} catch {
  $('save-status').textContent = '无法读取旧看板，可继续使用';
}
$('board-title').value = title;

/** 即时持久化，不依赖关闭事件；容量达到上限时展示约束。 */
function save() {
  try {
    localStorage.setItem(KEY, JSON.stringify({ cards, title, theme, column }));
    $('save-status').textContent =
      cards.length >= MAX_CARDS ? '已达到 100 张卡片上限' : '已保存在本地';
  } catch {
    $('save-status').textContent = '保存失败，当前修改关闭后可能丢失';
  }
}

/** 用户文本仅写入文本节点，避免把输入解释为标签。 */
function node(tag, className, text) {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}
/** 创建含操作对象名称的按钮，图标也可被读屏识别。 */
function button(text, name, action) {
  const control = node('button', '', text);
  control.type = 'button';
  control.title = name;
  control.setAttribute('aria-label', name);
  control.addEventListener('click', action);
  return control;
}

/** 新建默认进入当前列；编辑保留原有标识。 */
function edit(card = null) {
  if (!card && cards.length >= MAX_CARDS) return;
  editing = card?.id ?? null;
  $('card-title').value = card?.title ?? '';
  $('card-detail').value = card?.detail ?? '';
  $('card-status').value = card?.status ?? column;
  $('card-title').setCustomValidity('');
  $('card-dialog').showModal();
  $('card-title').focus();
}

/** 同列上移或下移，不改变其他列卡片的相对次序。 */
function reorder(card, offset) {
  const peers = cards.filter((item) => item.status === card.status);
  const target = peers[peers.indexOf(card) + offset];
  if (!target) return;
  const left = cards.indexOf(card),
    right = cards.indexOf(target);
  [cards[left], cards[right]] = [cards[right], cards[left]];
  save();
  render(card.id);
}

/** 构建有键盘替代操作的卡片，不占用宿主的修饰键拖窗手势。 */
function renderCard(card) {
  const row = node('li', 'card', '');
  const open = button(card.title, `编辑：${card.title}`, () => edit(card));
  open.className = 'card-title';
  open.dataset.cardId = card.id;
  row.append(open);
  if (card.detail) row.append(node('p', 'card-detail', card.detail));
  const actions = node('div', 'card-actions', '');
  for (const [offset, arrow, name] of [
    [-1, '←', '移至前一状态'],
    [1, '→', '移至后一状态'],
  ]) {
    const move = button(arrow, `${name}：${card.title}`, () => {
      if (moveCard(card, offset)) {
        column = card.status;
        save();
        render(card.id);
      }
    });
    move.disabled =
      COLUMNS.indexOf(card.status) + offset < 0 ||
      COLUMNS.indexOf(card.status) + offset >= COLUMNS.length;
    actions.append(move);
  }
  const peers = cards.filter((item) => item.status === card.status),
    index = peers.indexOf(card);
  for (const [offset, text, name] of [
    [-1, '↑', '上移'],
    [1, '↓', '下移'],
  ]) {
    const move = button(text, `${name}：${card.title}`, () => reorder(card, offset));
    move.disabled = index + offset < 0 || index + offset >= peers.length;
    actions.append(move);
  }
  actions.append(
    button('×', `删除：${card.title}`, () => {
      deleted = [{ card: { ...card }, index: cards.indexOf(card) }];
      cards = cards.filter((item) => item.id !== card.id);
      save();
      render();
    }),
  );
  row.append(actions);
  return row;
}

/** 宽窗口展示三列，窄窗口只显示当前状态；重绘后恢复目标卡片焦点。 */
function render(focusId = null) {
  document.documentElement.dataset.theme = theme;
  document.documentElement.dataset.column = column;
  for (const tab of document.querySelectorAll('[data-column-button]'))
    tab.setAttribute('aria-pressed', String(tab.dataset.columnButton === column));
  for (const status of COLUMNS) {
    const container = $(`${status}-cards`),
      scroll = container.scrollTop;
    const group = cards.filter((card) => card.status === status);
    container.replaceChildren(...group.map(renderCard));
    container.scrollTop = scroll;
    $(`${status}-count`).textContent = group.length;
    $(`${status}-empty`).hidden = group.length > 0;
  }
  $('add-card').disabled = cards.length >= MAX_CARDS;
  $('clear-done').disabled = !cards.some((card) => card.status === 'done');
  $('undo').hidden = !deleted;
  $('undo').disabled = Boolean(deleted && cards.length + deleted.length > MAX_CARDS);
  if (focusId)
    [...document.querySelectorAll('[data-card-id]')]
      .find((item) => item.dataset.cardId === focusId)
      ?.focus({ preventScroll: true });
}
$('card-form').addEventListener('submit', (event) => {
  event.preventDefault();
  const cardTitle = $('card-title').value.trim();
  if (!cardTitle) {
    $('card-title').setCustomValidity('请输入事项');
    $('card-title').reportValidity();
    return;
  }
  const values = {
    title: cardTitle,
    detail: $('card-detail').value,
    status: $('card-status').value,
  };
  const existing = cards.find((card) => card.id === editing);
  if (existing) Object.assign(existing, values);
  else if (cards.length < MAX_CARDS)
    cards.push({ id: globalThis.crypto?.randomUUID?.() ?? `card-${Date.now()}`, ...values });
  column = values.status;
  save();
  render();
  $('card-dialog').close();
});
$('card-title').addEventListener('input', () => $('card-title').setCustomValidity(''));
$('add-card').addEventListener('click', () => edit());
$('close-dialog').addEventListener('click', () => $('card-dialog').close());
$('board-title').addEventListener('input', () => {
  title = $('board-title').value;
  save();
});
$('theme').addEventListener('click', () => {
  theme = theme === 'light' ? 'dark' : 'light';
  save();
  render();
});
for (const tab of document.querySelectorAll('[data-column-button]'))
  tab.addEventListener('click', () => {
    column = tab.dataset.columnButton;
    save();
    render();
  });
$('clear-done').addEventListener('click', () => {
  deleted = cards.flatMap((card, index) =>
    card.status === 'done' ? [{ card: { ...card }, index }] : [],
  );
  cards = cards.filter((card) => card.status !== 'done');
  save();
  render();
});
$('undo').addEventListener('click', () => {
  if (!deleted || cards.length + deleted.length > MAX_CARDS) return;
  for (const { card, index } of deleted) cards.splice(Math.min(index, cards.length), 0, card);
  deleted = null;
  save();
  render();
});
render();
