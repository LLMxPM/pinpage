// 功能：渲染文件夹排序、筛选、面包屑和安全文本行，导航动作交给控制器。
/** 为当前页面建立独立视图，避免多个工具共享状态。 */
export function createFolderView(state, els, actions) {
  const { refresh, openEntry, revealEntry } = actions;
  /** 按名称自然排序，目录优先；文件名以文本节点渲染。 */
  function sortedEntries() {
    return state.entries
      .filter(
        (entry) =>
          (state.showHidden || !entry.hidden) &&
          entry.name.toLocaleLowerCase().includes(state.filter.toLocaleLowerCase()),
      )
      .sort((a, b) => {
        if (a.kind !== b.kind) return a.kind === 'dir' ? -1 : 1;
        if (state.sort === 'modified')
          return (
            b.modifiedMs - a.modifiedMs || a.name.localeCompare(b.name, 'zh-CN', { numeric: true })
          );
        if (state.sort === 'size')
          return (
            b.sizeBytes - a.sizeBytes || a.name.localeCompare(b.name, 'zh-CN', { numeric: true })
          );
        return a.name.localeCompare(b.name, 'zh-CN', { numeric: true });
      });
  }

  /** 将字节数格式化为紧凑单位，保留文件大小的量级信息。 */
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
          refresh(state.segments.slice(0, level.depth));
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
        refresh(state.segments.concat(entry.name));
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
    els.emptyList.hidden = entries.length > 0;
    els.emptyList.textContent = state.entries.length
      ? '没有符合筛选条件的项目'
      : '这个文件夹还是空的';
  }

  return { render, renderBreadcrumb };
}
