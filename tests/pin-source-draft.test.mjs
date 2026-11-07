// 功能：验证来源草稿只在用户确实改动时提交，避免保存外观设置时重新加载页面或重复复制副本。
import assert from 'node:assert/strict';
import test from 'node:test';
import { initialSourceDraft, resolveSourceInput } from '../src/features/pins/pinSourceDraft.ts';

/// 构造最小页面记录；测试只关心来源草稿，不涉及窗口与原生状态。
function urlPin(url = 'https://example.com/') {
  return { id: 'p-1', name: '看板', source: { kind: 'url', url } };
}

function bundledPin(entry = 'index.html') {
  return { id: 'p-2', name: '便签', source: { kind: 'bundled', entry } };
}

function preview(ticket = 'ticket-1', candidates = ['index.html']) {
  return { ticket, mode: 'directory', label: '站点', candidates, files: 1, bytes: 128 };
}

test('打开设置时沿用现有地址，未改动则不提交来源', () => {
  const pin = urlPin();
  const draft = initialSourceDraft(pin);
  assert.equal(draft.address, 'https://example.com/');
  assert.equal(resolveSourceInput(pin, draft), null);
});

test('地址首尾空白不算改动，换地址才提交', () => {
  const pin = urlPin();
  const draft = initialSourceDraft(pin);
  draft.address = '  https://example.com/  ';
  assert.equal(resolveSourceInput(pin, draft), null);
  draft.address = '127.0.0.1:3000';
  assert.deepEqual(resolveSourceInput(pin, draft), { kind: 'url', address: '127.0.0.1:3000' });
});

test('清空地址不提交来源，避免把非法空地址写入配置', () => {
  const pin = urlPin();
  const draft = initialSourceDraft(pin);
  draft.address = '   ';
  assert.equal(resolveSourceInput(pin, draft), null);
});

test('导入页需要新预览和确认入口，两者齐备才提交', () => {
  const pin = bundledPin();
  const draft = initialSourceDraft(pin);
  assert.equal(draft.address, '');
  assert.equal(resolveSourceInput(pin, draft), null);
  draft.preview = preview();
  assert.equal(resolveSourceInput(pin, draft), null);
  draft.entry = 'app.html';
  assert.deepEqual(resolveSourceInput(pin, draft), {
    kind: 'import',
    ticket: 'ticket-1',
    entry: 'app.html',
  });
});

test('切换页面时按新页面重建草稿，不残留上一页面的地址或预览', () => {
  const draft = initialSourceDraft(urlPin());
  draft.address = 'https://old.example.com';
  draft.preview = preview();
  draft.entry = 'index.html';
  const next = initialSourceDraft(bundledPin());
  assert.deepEqual(next, { address: '', preview: null, entry: '' });
});
