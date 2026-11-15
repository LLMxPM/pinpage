// 功能：提供待办编辑、排序、完成过滤、批量清理和可撤销删除，复用旧版存储键。
import { MAX_TASKS, normalizeTasks, newTaskId } from './task-state.js';

/** 查找页面固定控件，不接受用户输入选择器。 */
const $ = (selector) => document.querySelector(selector);
const KEY = 'pinpage-checklist-v1';
let tasks = [];
let deleted = null;
let editing = null;
try {
  tasks = normalizeTasks(JSON.parse(localStorage.getItem(KEY) || '[]'));
} catch {
  $('#task-save-status').textContent = '无法读取原有记录，可继续添加';
}

/** 保存结构化清单；配额失败时保留编辑能力并明确告知。 */
function save() {
  try {
    localStorage.setItem(KEY, JSON.stringify(tasks));
    $('#task-save-status').textContent = tasks.length >= MAX_TASKS ? '已达 100 项上限' : '已保存';
  } catch {
    $('#task-save-status').textContent = '保存失败，当前修改仅在本页保留';
  }
}

/** 创建有可访问名称的轻量按钮。 */
function button(text, name, action) {
  const control = document.createElement('button');
  control.type = 'button';
  control.textContent = text;
  control.title = name;
  control.setAttribute('aria-label', name);
  control.addEventListener('click', action);
  return control;
}

/** 保存合法文本并限制长度；空白输入保留原任务，取消时不修改。 */
function finishEdit(task, input, cancelled = false) {
  if (editing !== task.id) return;
  if (!cancelled) {
    const text = input.value.trim();
    if (!text) {
      input.setCustomValidity('请输入待办内容');
      input.reportValidity();
      return;
    }
    task.text = text.slice(0, 160);
    save();
  }
  editing = null;
  render();
  [...$('#tasks').querySelectorAll('[data-edit]')]
    .find((control) => control.dataset.edit === task.id)
    ?.focus();
}

/** 构造可编辑任务行，用户文字仅写入文本节点和输入值。 */
function renderRow(task, index) {
  const row = document.createElement('li');
  row.className = `task-row${task.done ? ' done' : ''}`;
  const label = document.createElement('label');
  const check = document.createElement('input');
  check.type = 'checkbox';
  check.checked = task.done;
  check.dataset.taskId = task.id;
  check.setAttribute('aria-label', `完成：${task.text}`);
  check.addEventListener('change', () => {
    task.done = check.checked;
    save();
    render();
    const next = [...$('#tasks').querySelectorAll('input[type="checkbox"]')].find(
      (control) => control.dataset.taskId === task.id,
    );
    (next ?? $('#task-input')).focus();
  });
  const text = document.createElement('span');
  text.textContent = task.text;
  label.append(check, text);
  row.append(label);
  const actions = document.createElement('div');
  actions.className = 'row-actions';
  const edit = button('编辑', `编辑：${task.text}`, () => {
    editing = task.id;
    render();
  });
  edit.dataset.edit = task.id;
  actions.append(edit);
  for (const [offset, arrow, name] of [
    [-1, '↑', '上移'],
    [1, '↓', '下移'],
  ]) {
    const move = button(arrow, `${name}：${task.text}`, () => {
      const target = index + offset;
      [tasks[index], tasks[target]] = [tasks[target], tasks[index]];
      save();
      render();
      [...$('#tasks').querySelectorAll('[data-edit]')]
        .find((control) => control.dataset.edit === task.id)
        ?.focus();
    });
    move.disabled = index + offset < 0 || index + offset >= tasks.length;
    actions.append(move);
  }
  actions.append(
    button('×', `删除：${task.text}`, () => {
      deleted = [{ task: { ...task }, index }];
      tasks.splice(index, 1);
      save();
      render();
      $('#task-input').focus();
    }),
  );
  row.append(actions);
  if (editing === task.id) {
    const input = document.createElement('input');
    input.value = task.text;
    input.maxLength = 160;
    input.setAttribute('aria-label', '编辑待办内容');
    input.className = 'task-edit';
    input.addEventListener('input', () => input.setCustomValidity(''));
    input.addEventListener('keydown', (event) => {
      if (event.isComposing) return;
      if (event.key === 'Enter' || event.key === 'Escape') {
        event.preventDefault();
        finishEdit(task, input, event.key === 'Escape');
      }
    });
    row.replaceChildren(
      input,
      button('保存', '保存修改', () => finishEdit(task, input)),
      button('取消', '取消修改', () => finishEdit(task, input, true)),
    );
    requestAnimationFrame(() => {
      if (input.isConnected) {
        input.focus();
        input.select();
      }
    });
  }
  return row;
}

/** 渲染受高度约束的列表，保持滚动位置，容量提示仅在达到上限时突出。 */
function render() {
  const list = $('#tasks');
  const scroll = list.scrollTop;
  list.replaceChildren();
  let visible = 0;
  tasks.forEach((task, index) => {
    if ($('#hide-completed').checked && task.done) return;
    list.append(renderRow(task, index));
    visible += 1;
  });
  list.scrollTop = scroll;
  const done = tasks.filter((task) => task.done).length;
  $('#task-progress').textContent = `${done} / ${tasks.length} 已完成`;
  $('#empty-hint').hidden = visible > 0;
  $('#empty-hint').textContent = tasks.length
    ? '当前待办已全部完成'
    : '暂无待办，开始添加第一件事吧';
  $('#clear-completed').disabled = !done;
  $('#add-task').disabled = tasks.length >= MAX_TASKS;
  $('#task-input').disabled = tasks.length >= MAX_TASKS;
  $('#undo-task').hidden = !deleted;
  $('#undo-task').disabled = Boolean(deleted && tasks.length + deleted.length > MAX_TASKS);
}

$('#task-form').addEventListener('submit', (event) => {
  event.preventDefault();
  const text = $('#task-input').value.trim();
  if (!text || tasks.length >= MAX_TASKS) return;
  tasks.push({ id: newTaskId(), text: text.slice(0, 160), done: false });
  $('#task-input').value = '';
  save();
  render();
  $('#task-input').focus();
});
$('#hide-completed').addEventListener('change', render);
$('#clear-completed').addEventListener('click', () => {
  deleted = tasks.flatMap((task, index) => (task.done ? [{ task: { ...task }, index }] : []));
  tasks = tasks.filter((task) => !task.done);
  save();
  render();
});
$('#undo-task').addEventListener('click', () => {
  if (!deleted || tasks.length + deleted.length > MAX_TASKS) return;
  for (const { task, index } of deleted) tasks.splice(Math.min(index, tasks.length), 0, task);
  deleted = null;
  save();
  render();
});
render();
