// 功能：管理最多一百条本地待办，使用文本节点渲染内容，避免把用户输入解释为 HTML。
const storageKey = 'pinpage-checklist-v1';
const list = document.querySelector('#tasks');
const input = document.querySelector('#task-input');
const status = document.querySelector('#save-status');
const progress = document.querySelector('#task-progress');
const empty = document.querySelector('#empty-hint');
const add = document.querySelector('#add-task');
let tasks = [];

/// 损坏或超范围的本地数据被过滤，文本长度和项目数量与表单约束一致。
function restore() {
  try {
    const saved = JSON.parse(localStorage.getItem(storageKey) ?? '[]');
    if (Array.isArray(saved)) {
      tasks = saved
        .filter((item) => item && typeof item.text === 'string' && item.text.trim())
        .slice(0, 100)
        .map((item) => ({ text: item.text.slice(0, 160), done: item.done === true }));
    }
  } catch {
    status.textContent = '无法读取本地记录，可继续添加';
  }
}

/// 持久化结构化清单；失败时提醒用户当前修改尚未落盘。
function save() {
  try {
    localStorage.setItem(storageKey, JSON.stringify(tasks));
    status.textContent = '已自动保存';
  } catch {
    status.textContent = '保存失败，当前修改仅在本页保留';
  }
}

/// 更新进度和容量提示，完成勾选时不重建节点，保留当前键盘焦点。
function updateProgress() {
  progress.textContent =
    tasks.filter((item) => item.done).length + ' / ' + tasks.length + ' 已完成';
  empty.hidden = tasks.length > 0;
  add.disabled = tasks.length >= 100;
  input.disabled = tasks.length >= 100;
}

/// 用户输入始终写入 textContent；仅增删时重绘，删除后回到添加入口。
function render() {
  list.replaceChildren();
  tasks.forEach((task, index) => {
    const row = document.createElement('li');
    row.className = 'task-row' + (task.done ? ' done' : '');
    const label = document.createElement('label');
    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.checked = task.done;
    checkbox.addEventListener('change', () => {
      task.done = checkbox.checked;
      row.classList.toggle('done', task.done);
      save();
      updateProgress();
    });
    const text = document.createElement('span');
    text.textContent = task.text;
    label.append(checkbox, text);
    const remove = document.createElement('button');
    remove.type = 'button';
    remove.textContent = '删除';
    remove.setAttribute('aria-label', '删除待办：' + task.text);
    remove.addEventListener('click', () => {
      tasks.splice(index, 1);
      save();
      render();
      input.focus();
    });
    row.append(label, remove);
    list.append(row);
  });
  updateProgress();
}

/// 忽略纯空格输入，新增后保留输入焦点，支持连续录入。
function submit(event) {
  event.preventDefault();
  const text = input.value.trim();
  if (!text || tasks.length >= 100) return;
  tasks.push({ text: text.slice(0, 160), done: false });
  input.value = '';
  save();
  render();
  if (!input.disabled) input.focus();
}

restore();
render();
document.querySelector('#task-form').addEventListener('submit', submit);
