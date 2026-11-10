// 功能：PinPage 文件夹能力的薄封装，可整份复制到自己的页面使用。
// 在 PinPage 内走宿主 IPC；普通浏览器中报告不可用，由页面自行降级。

/** 判断是否运行在 PinPage 内（宿主桥是否存在）。 */
export function inPinPage() {
  const bridge = window.__TAURI_INTERNALS__;
  return Boolean(bridge && typeof bridge.invoke === 'function');
}

/** 把宿主抛出的结构化错误归一为 { code, args, detail }，其余按未知处理。 */
function normalizeError(error) {
  if (error && typeof error === 'object' && typeof error.code === 'string') {
    return {
      code: error.code,
      args: error.args && typeof error.args === 'object' ? error.args : {},
      detail: typeof error.detail === 'string' ? error.detail : null,
    };
  }
  return { code: 'errors.unknown', args: {}, detail: String(error) };
}

/** 调用宿主命令；不在 PinPage 内时以 page.hostUnavailable 拒绝。 */
async function invoke(command, args = {}) {
  if (!inPinPage()) {
    throw { code: 'page.hostUnavailable', args: {}, detail: null };
  }
  try {
    return await window.__TAURI_INTERNALS__.invoke(command, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

/** 文件夹能力入口；路径均为授权根目录内的相对路径，用 / 分隔。 */
export const folderApi = {
  available: inPinPage,
  /** 宿主信息与能力表，用于特性探测。 */
  hostInfo: () => invoke('host_info'),
  /** 打开原生选择器；确认返回目录显示名，取消返回 null。 */
  pick: () => invoke('folder_pick'),
  /** 列举目录；返回 { rootLabel, entries, truncated }。 */
  list: (path = '') => invoke('folder_list', { path }),
  /** 用系统默认程序打开文件。 */
  open: (path) => invoke('folder_open', { path }),
  /** 在资源管理器中打开条目所在目录。 */
  reveal: (path) => invoke('folder_reveal', { path }),
};
