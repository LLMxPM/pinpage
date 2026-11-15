// 功能：PinPage 系统监控能力的薄封装，可整份复制到自己的页面使用。
// 数据来自宿主共享采样缓存，页面只读，不触发额外采样。

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

/** 监控能力入口；采样约每秒一次，所有页面共享同一份数据。 */
export const monitorApi = {
  available: inPinPage,
  /** 宿主信息与能力表，用于特性探测。 */
  hostInfo: () => invoke('host_info'),
  /** 最近一次系统采样快照；启动初期可能以 errors.statsUnavailable 拒绝。 */
  stats: () => invoke('system_stats'),
};
