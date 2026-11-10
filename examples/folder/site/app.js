// 功能：文件夹组件的状态机与渲染；授权、导航、打开与定位都经过 folder-api。
import { folderApi } from './folder-api.js';

const STORE_KEY = 'pinpage-folder-v1';

/** 消息码到页面文案；页面内容默认中文，不随工作台语言切换。 */
const ERROR_TEXT = {
  'errors.folderNoGrant': '尚未授权文件夹，请先选择',
  'errors.folderRootMissing': '授权的文件夹已失效，请重新选择',
  'errors.folderOutside': '路径超出授权范围',
  'errors.folderNotFound': '条目不存在，可能已被移动或删除',
  'errors.folderReadFailed': '读取文件夹失败，请刷新重试',
  'errors.folderOpenFailed': '打开失败，请重试',
  'errors.folderPathInvalid': '所选文件夹无效，请重新选择',
  'errors.statsUnavailable': '系统数据暂不可用',
  'page.hostUnavailable': '当前不在 PinPage 中，无法使用文件夹能力',
};

/** 浏览器预览用的演示目录树，明确标注，不读取真实文件。 */
const DEMO_TREE = {
  '': [
    { name: '项目', kind: 'dir', sizeBytes: 0, modifiedMs: Date.now() - 86400000, hidden: false },
    { name: '图片', kind: 'dir', sizeBytes: 0, modifiedMs: Date.now() - 3600000, hidden: false },
    {
      name: '说明.md',
      kind: 'file',
      sizeBytes: 2048,
      modifiedMs: Date.now() - 7200000,
      hidden: false,
    },
    {
      name: '预算.xlsx',
      kind: 'file',
      sizeBytes: 48211,
      modifiedMs: Date.now() - 172800000,
      hidden: false,
    },
  ],
  项目: [
    { name: 'src', kind: 'dir', sizeBytes: 0, modifiedMs: Date.now() - 600000, hidden: false },
    {
      name: 'index.html',
      kind: 'file',
      sizeBytes: 3120,
      modifiedMs: Date.now() - 600000,
      hidden: false,
    },
    {
      name: 'README.md',
      kind: 'file',
      sizeBytes: 1204,
      modifiedMs: Date.now() - 90000000,
      hidden: false,
    },
  ],
  '项目/src': [
    {
      name: 'main.js',
      kind: 'file',
      sizeBytes: 8231,
      modifiedMs: Date.now() - 300000,
      hidden: false,
    },
    {
      name: 'style.css',
      kind: 'file',
      sizeBytes: 2210,
      modifiedMs: Date.now() - 300000,
      hidden: false,
    },
  ],
  图片: [
    {
      name: '壁纸.png',
      kind: 'file',
      sizeBytes: 1048576,
      modifiedMs: Date.now() - 259200000,
      hidden: false,
    },
    {
      name: '截图.jpg',
      kind: 'file',
      sizeBytes: 324511,
      modifiedMs: Date.now() - 5400000,
      hidden: false,
    },
  ],
};

const els = {
  rootLabel: document.getElementById('root-label'),
  breadcrumb: document.getElementById('breadcrumb'),
  demoBanner: document.getElementById('demo-banner'),
  errorBanner: document.getElementById('error-banner'),
  emptyState: document.getElementById('empty-state'),
  list: document.getElementById('entry-list'),
  status: document.getElementById('status-text'),
  pick: document.getElementById('pick-folder'),
  refresh: document.getElementById('refresh'),
  changeRoot: document.getElementById('change-root'),
  toggleHidden: document.getElementById('toggle-hidden'),
};

const state = {
  demo: !folderApi.available(),
  rootLabel: '文件夹',
  segments: [],
  entries: [],
  truncated: false,
  showHidden: false,
  busy: false,
};

/** 读取本地偏好；损坏或缺失时回退默认值。 */
function loadPrefs() {
  try {
    const raw = JSON.parse(localStorage.getItem(STORE_KEY) || '{}');
    if (raw && typeof raw.showHidden === 'boolean') state.showHidden = raw.showHidden;
  } catch {
    /* 存储不可用时保持默认 */
  }
}

/** 保存偏好；失败不阻断浏览。 */
function savePrefs() {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify({ showHidden: state.showHidden }));
  } catch {
    /* 忽略配额或隐私模式错误 */
  }
}

/** 当前相对路径；根目录为空字符串。 */
function currentPath() {
  return state.segments.join('/');
}

/** 列出目录：演示模式走内存树，否则走宿主命令。 */
async function listPath(path) {
  if (state.demo) {
    const entries = DEMO_TREE[path];
    if (!entries) throw { code: 'errors.folderNotFound' };
    return { rootLabel: '演示文件夹', entries, truncated: false };
  }
  return folderApi.list(path);
}

/** 显示错误横幅；未授权与根失效同时给出重选入口。 */
function showError(error) {
  const code = error && error.code ? error.code : 'errors.unknown';
  els.errorBanner.textContent = ERROR_TEXT[code] || '操作未完成，请重试';
  els.errorBanner.hidden = false;
  if (code === 'errors.folderNoGrant' || code === 'errors.folderRootMissing') {
    els.emptyState.hidden = false;
    els.list.hidden = true;
    els.breadcrumb.hidden = true;
  }
}

function clearError() {
  els.errorBanner.hidden = true;
}

/** 按名称自然排序，目录优先；文件名以文本节点渲染。 */
function sortedEntries() {
  return state.entries
    .filter((entry) => state.showHidden || !entry.hidden)
    .sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === 'dir' ? -1 : 1;
      return a.name.localeCompare(b.name, 'zh-CN', { numeric: true });
    });
}

function formatSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit = -1;
  do {
    value /= 1024;
    unit += 1;
  } while (value >= 1024 && unit < units.length - 1);
  return `${value >= 100 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`;
}

const dateFormatter = new Intl.DateTimeFormat('zh-CN', {
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
});

/** 扩展名徽章分类；未知扩展使用中性样式。 */
function extBadge(name) {
  const dot = name.lastIndexOf('.');
  if (dot <= 0 || dot === name.length - 1) return null;
  const ext = name.slice(dot + 1).toLowerCase();
  if (ext.length > 5) return null;
  const groups = {
    'ext-code': ['html', 'js', 'ts', 'css', 'json', 'md', 'vue', 'rs', 'py'],
    'ext-image': ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico'],
    'ext-doc': ['txt', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'csv'],
    'ext-media': ['mp3', 'mp4', 'wav', 'mkv', 'flac'],
    'ext-archive': ['zip', '7z', 'rar', 'tar', 'gz', 'pinpage'],
  };
  const badge = document.createElement('span');
  badge.className = 'ext-badge';
  for (const [className, list] of Object.entries(groups)) {
    if (list.includes(ext)) badge.classList.add(className);
  }
  badge.textContent = ext.slice(0, 4);
  return badge;
}

/** 渲染面包屑：根显示名加各级目录，点击回退到对应层级。 */
function renderBreadcrumb() {
  els.breadcrumb.textContent = '';
  if (state.segments.length === 0) {
    els.breadcrumb.hidden = true;
    return;
  }
  els.breadcrumb.hidden = false;
  const levels = [{ label: state.rootLabel, depth: 0 }].concat(
    state.segments.map((name, index) => ({ label: name, depth: index + 1 })),
  );
  levels.forEach((level, index) => {
    if (index > 0) {
      const sep = document.createElement('span');
      sep.className = 'crumb-sep';
      sep.textContent = '›';
      sep.setAttribute('aria-hidden', 'true');
      els.breadcrumb.append(sep);
    }
    const button = document.createElement('button');
    button.className = 'crumb';
    button.textContent = level.label;
    button.title = level.label;
    if (level.depth === state.segments.length) {
      button.setAttribute('aria-current', 'page');
    } else {
      button.addEventListener('click', () => {
        state.segments = state.segments.slice(0, level.depth);
        refresh();
      });
    }
    els.breadcrumb.append(button);
  });
}

/** 渲染单个条目行：主按钮打开或进入目录，侧按钮在资源管理器中定位。 */
function renderRow(entry) {
  const path = state.segments.concat(entry.name).join('/');
  const row = document.createElement('li');
  row.className = 'entry-row' + (entry.hidden ? ' is-hidden' : '');

  const main = document.createElement('button');
  main.className = 'entry-main';
  const icon = document.createElement('span');
  icon.className = entry.kind === 'dir' ? 'icon-folder' : 'icon-file';
  icon.setAttribute('aria-hidden', 'true');
  const text = document.createElement('span');
  text.className = 'entry-text';
  const name = document.createElement('span');
  name.className = 'entry-name';
  name.textContent = entry.name;
  name.title = entry.name;
  const meta = document.createElement('span');
  meta.className = 'entry-meta';
  const parts = [];
  if (entry.kind === 'file') parts.push(formatSize(entry.sizeBytes));
  if (entry.modifiedMs) parts.push(dateFormatter.format(new Date(entry.modifiedMs)));
  if (entry.hidden) parts.push('隐藏');
  meta.textContent = parts.join(' · ');
  text.append(name, meta);
  main.append(icon);
  if (entry.kind === 'file') {
    const badge = extBadge(entry.name);
    if (badge) main.append(badge);
  }
  main.append(text);
  main.setAttribute(
    'aria-label',
    entry.kind === 'dir' ? `进入目录 ${entry.name}` : `打开 ${entry.name}`,
  );
  main.addEventListener('click', () => {
    if (entry.kind === 'dir') {
      state.segments.push(entry.name);
      refresh();
    } else {
      openEntry(path);
    }
  });

  const reveal = document.createElement('button');
  reveal.className = 'entry-reveal';
  reveal.setAttribute('aria-label', `在资源管理器中显示 ${entry.name}`);
  reveal.title = '在资源管理器中显示';
  reveal.innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z"/></svg>';
  reveal.addEventListener('click', () => revealEntry(path));

  row.append(main, reveal);
  return row;
}

/** 全量渲染当前状态。 */
function render() {
  els.rootLabel.textContent = state.rootLabel;
  els.rootLabel.title = state.rootLabel;
  els.demoBanner.hidden = !state.demo;
  els.toggleHidden.setAttribute('aria-pressed', String(state.showHidden));
  renderBreadcrumb();
  els.list.textContent = '';
  const entries = sortedEntries();
  for (const entry of entries) els.list.append(renderRow(entry));
  els.list.hidden = false;
  els.emptyState.hidden = true;
  const notes = [`${entries.length} 个项目`];
  if (state.truncated) notes.push('条目过多，仅显示前 2000 项');
  if (state.demo) notes.push('演示数据');
  els.status.textContent = notes.join(' · ');
}

/** 刷新当前目录；未授权时进入空状态，其余错误保留横幅提示。 */
async function refresh() {
  if (state.busy) return;
  state.busy = true;
  clearError();
  els.status.textContent = '正在读取…';
  try {
    const listing = await listPath(currentPath());
    state.rootLabel = listing.rootLabel;
    state.entries = listing.entries;
    state.truncated = listing.truncated;
    render();
  } catch (error) {
    state.entries = [];
    state.truncated = false;
    els.list.textContent = '';
    showError(error);
    if (error && error.code === 'errors.folderNoGrant') {
      els.status.textContent = '';
    } else {
      renderBreadcrumb();
      els.status.textContent = '';
    }
  } finally {
    state.busy = false;
  }
}

/** 打开文件；演示模式仅提示。 */
async function openEntry(path) {
  clearError();
  if (state.demo) {
    els.status.textContent = '演示模式不会打开真实文件';
    return;
  }
  try {
    await folderApi.open(path);
  } catch (error) {
    showError(error);
  }
}

/** 在资源管理器中定位；演示模式仅提示。 */
async function revealEntry(path) {
  clearError();
  if (state.demo) {
    els.status.textContent = '演示模式不会打开真实目录';
    return;
  }
  try {
    await folderApi.reveal(path);
  } catch (error) {
    showError(error);
  }
}

/** 发起授权选择；取消时保持现状，确认后回到根目录。 */
async function pickRoot() {
  if (state.busy || state.demo) return;
  state.busy = true;
  els.pick.disabled = true;
  clearError();
  try {
    const label = await folderApi.pick();
    if (label !== null) {
      state.segments = [];
      state.busy = false;
      await refresh();
    }
  } catch (error) {
    showError(error);
  } finally {
    state.busy = false;
    els.pick.disabled = false;
  }
}

els.pick.addEventListener('click', pickRoot);
els.changeRoot.addEventListener('click', pickRoot);
els.refresh.addEventListener('click', () => refresh());
els.toggleHidden.addEventListener('click', () => {
  state.showHidden = !state.showHidden;
  savePrefs();
  els.toggleHidden.setAttribute('aria-pressed', String(state.showHidden));
  if (!els.list.hidden) render();
});

loadPrefs();
refresh();
