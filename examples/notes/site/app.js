// 功能：管理便签与待办的视图、配色和字号，兼容旧页面偏好及独立内容存储。
import './notes.js';
import './tasks.js';

const KEY = 'pinpage-notebook-v1';
const preferences = { view: 'notes', theme: 'paper', fontSize: 16 };
try {
  const saved = JSON.parse(localStorage.getItem(KEY) || 'null');
  if (['notes', 'tasks', 'both'].includes(saved?.view)) preferences.view = saved.view;
  else if (
    localStorage.getItem('pinpage-checklist-v1') &&
    !localStorage.getItem('pinpage-notes-v1')
  )
    preferences.view = 'tasks';
  if (['paper', 'sage', 'dark'].includes(saved?.theme)) preferences.theme = saved.theme;
  if ([14, 16, 18, 20].includes(saved?.fontSize)) preferences.fontSize = saved.fontSize;
} catch {
  /* 默认视图仍可用，内容模块各自报告读取失败。 */
}

/** 应用视图与主题，原生透明变量只读；设置变化不重建编辑区域。 */
function apply() {
  document.documentElement.dataset.view = preferences.view;
  document.documentElement.dataset.theme = preferences.theme;
  document.documentElement.style.setProperty('--editor-size', `${preferences.fontSize}px`);
  document.querySelector('#note-pane').hidden = preferences.view === 'tasks';
  document.querySelector('#task-pane').hidden = preferences.view === 'notes';
  for (const button of document.querySelectorAll('[data-view-button]'))
    button.setAttribute('aria-pressed', String(button.dataset.viewButton === preferences.view));
  document.querySelector('#theme').value = preferences.theme;
  document.querySelector('#font-size').value = preferences.fontSize;
}

/** 偏好即时保存，内容的存储失败提示不会被外观设置覆盖。 */
function save() {
  apply();
  try {
    localStorage.setItem(KEY, JSON.stringify(preferences));
    document.querySelector('#preferences-status').textContent = '设置已保存';
  } catch {
    document.querySelector('#preferences-status').textContent = '设置保存失败，当前外观仅临时生效';
  }
}
for (const button of document.querySelectorAll('[data-view-button]'))
  button.addEventListener('click', () => {
    preferences.view = button.dataset.viewButton;
    save();
  });
for (const [id, key] of [
  ['theme', 'theme'],
  ['font-size', 'fontSize'],
])
  document.querySelector(`#${id}`).addEventListener('change', (event) => {
    preferences[key] = key === 'fontSize' ? Number(event.target.value) : event.target.value;
    save();
  });
const dialog = document.querySelector('#settings-dialog');
document.querySelector('#open-settings').addEventListener('click', () => dialog.showModal());
document.querySelector('#close-settings').addEventListener('click', () => dialog.close());
apply();
