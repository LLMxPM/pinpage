// 功能：管理文件夹能力探测、授权与导航竞态，目录和文件动作仅通过受控宿主接口。
import { folderApi } from './folder-api.js';
import { DEMO_TREE } from './demo.js';
import { createFolderView } from './view.js';

/** 获取页面固定元素，不接受用户构造选择器。 */
const $ = (id) => document.getElementById(id);
const els = Object.fromEntries(
  Object.entries({
    rootLabel: 'root-label',
    breadcrumb: 'breadcrumb',
    demoBanner: 'demo-banner',
    errorBanner: 'error-banner',
    emptyState: 'empty-state',
    list: 'entry-list',
    status: 'status-text',
    pick: 'pick-folder',
    refresh: 'refresh',
    changeRoot: 'change-root',
    toggleHidden: 'toggle-hidden',
    emptyList: 'empty-list',
  }).map(([key, id]) => [key, $(id)]),
);
const state = {
  demo: !folderApi.available(),
  supported: false,
  rootLabel: '文件夹',
  segments: [],
  entries: [],
  truncated: false,
  showHidden: false,
  filter: '',
  sort: 'name',
  revision: 0,
  picking: false,
};
const view = createFolderView(state, els, { refresh, openEntry, revealEntry });
const ERRORS = {
  'errors.folderNoGrant': '尚未授权文件夹，请先选择',
  'errors.folderRootMissing': '授权文件夹已失效，请重新选择',
  'errors.folderOutside': '路径超出授权范围',
  'errors.folderNotFound': '条目不存在，可能已被移动或删除',
  'errors.folderReadFailed': '读取失败，请刷新重试',
  'errors.folderOpenFailed': '打开失败，请重试',
  'errors.folderPathInvalid': '所选文件夹无效，请重新选择',
};

/** 只恢复展示偏好，不存储或导出授权根目录。 */
function restore() {
  try {
    const saved = JSON.parse(localStorage.getItem('pinpage-folder-v1') || '{}');
    state.showHidden = saved?.showHidden === true;
    if (['name', 'modified', 'size'].includes(saved?.sort)) state.sort = saved.sort;
  } catch {
    /* 默认偏好可继续使用。 */
  }
  $('sort').value = state.sort;
}

/** 保存失败明确提示，但不影响只读浏览。 */
function save() {
  try {
    localStorage.setItem(
      'pinpage-folder-v1',
      JSON.stringify({ showHidden: state.showHidden, sort: state.sort }),
    );
  } catch {
    els.status.textContent = '偏好保存失败，浏览功能仍可使用';
  }
}

/** 显示错误并保留重选入口；失败导航不改变当前成功路径。 */
function showError(error) {
  els.errorBanner.textContent = ERRORS[error?.code] || '操作未完成，请重试';
  els.errorBanner.hidden = false;
  if (['errors.folderNoGrant', 'errors.folderRootMissing'].includes(error?.code)) {
    els.emptyState.hidden = false;
    els.list.hidden = true;
    els.breadcrumb.hidden = true;
    els.emptyList.hidden = true;
  }
}

/** 只接受最后一次导航响应；路径在成功后提交，避免连续点击造成路径漂移。 */
async function refresh(segments = state.segments) {
  if (!state.demo && !state.supported) return;
  const revision = ++state.revision;
  const target = [...segments];
  els.errorBanner.hidden = true;
  els.status.textContent = '正在读取…';
  try {
    const path = target.join('/');
    const listing = state.demo
      ? { rootLabel: '演示文件夹', entries: DEMO_TREE[path] ?? [], truncated: false }
      : await folderApi.list(path);
    if (revision !== state.revision) return;
    Object.assign(state, {
      segments: target,
      rootLabel: listing.rootLabel,
      entries: listing.entries,
      truncated: listing.truncated,
    });
    const keepListFocus =
      els.list.contains(document.activeElement) || els.breadcrumb.contains(document.activeElement);
    view.render();
    if (keepListFocus) els.list.focus();
  } catch (error) {
    if (revision !== state.revision) return;
    showError(error);
    els.status.textContent = '';
  }
}

/** 文件打开与定位复用宿主授权校验，演示环境只反馈提示。 */
async function act(path, reveal) {
  els.errorBanner.hidden = true;
  if (state.demo) {
    els.status.textContent = '演示模式不会打开真实文件或目录';
    return;
  }
  try {
    await (reveal ? folderApi.reveal(path) : folderApi.open(path));
  } catch (error) {
    showError(error);
  }
}
/** 通过系统默认程序打开授权范围内的文件。 */
function openEntry(path) {
  return act(path, false);
}
/** 将授权范围内条目交给资源管理器定位。 */
function revealEntry(path) {
  return act(path, true);
}

/** 授权取消保留当前状态；新目录确认后使旧导航响应失效。 */
async function pickRoot() {
  if (state.demo || !state.supported || state.picking) return;
  state.picking = true;
  els.pick.disabled = true;
  els.changeRoot.disabled = true;
  try {
    const label = await folderApi.pick();
    if (label !== null) {
      state.revision += 1;
      state.filter = '';
      $('filter').value = '';
      await refresh([]);
    }
  } catch (error) {
    showError(error);
  } finally {
    state.picking = false;
    els.pick.disabled = false;
    els.changeRoot.disabled = false;
  }
}

/** 以能力表而非版本或桥对象推断支持，宿主缺少能力时不伪装成演示。 */
async function initialize() {
  restore();
  if (!state.demo) {
    try {
      state.supported = (await folderApi.hostInfo()).capabilities?.folderRead >= 1;
    } catch {
      state.supported = false;
    }
    if (!state.supported) {
      els.errorBanner.hidden = false;
      els.errorBanner.textContent = '当前宿主未提供文件夹能力';
      els.pick.disabled = true;
      els.changeRoot.disabled = true;
      els.refresh.disabled = true;
      return;
    }
  } else {
    els.changeRoot.disabled = true;
  }
  await refresh();
}
els.pick.addEventListener('click', pickRoot);
els.changeRoot.addEventListener('click', pickRoot);
els.refresh.addEventListener('click', () => refresh());
els.toggleHidden.addEventListener('click', () => {
  state.showHidden = !state.showHidden;
  if (!els.list.hidden) view.render();
  save();
});
$('filter').addEventListener('input', (event) => {
  state.filter = event.target.value;
  if (!els.list.hidden) view.render();
});
$('sort').addEventListener('change', (event) => {
  state.sort = event.target.value;
  if (!els.list.hidden) view.render();
  save();
});
void initialize();
