// 功能：恢复便签内容并在输入时同步保存；存储不可用时明确提示，保持编辑可用。
const title = document.querySelector('#note-title');
const content = document.querySelector('#note-content');
const status = document.querySelector('#save-status');
const count = document.querySelector('#note-count');
const storageKey = 'pinpage-notes-v1';

/// 只接受本示例认识的文本字段，损坏的数据不会阻止页面打开。
function restore() {
  try {
    const saved = JSON.parse(localStorage.getItem(storageKey) ?? 'null');
    if (saved && typeof saved.title === 'string' && typeof saved.content === 'string') {
      title.value = saved.title.slice(0, 60);
      content.value = saved.content.slice(0, 10000);
    }
  } catch {
    status.textContent = '无法读取本地记录，可继续编辑';
  }
  count.textContent = content.value.length + ' / 10000';
}

/// 每次输入立即持久化，关闭 WebView 前无需等待防抖计时器。
function save() {
  count.textContent = content.value.length + ' / 10000';
  try {
    localStorage.setItem(
      storageKey,
      JSON.stringify({ title: title.value, content: content.value }),
    );
    status.textContent = '已自动保存';
  } catch {
    status.textContent = '保存失败，请先复制内容备份';
  }
}

restore();
title.addEventListener('input', save);
content.addEventListener('input', save);
