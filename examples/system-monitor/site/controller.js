// 功能：探测共享采样能力、串行轮询与过期提示，并保存指标和主题偏好。
import { monitorApi } from './monitor-api.js';
import { createMonitorView } from './view.js';
import { demoStats } from './demo.js';
import { sampleIsFresh } from './sample-state.js';

/** 获取固定控件，不使用采样内容构造选择器。 */
const $ = (id) => document.getElementById(id);
const els = Object.fromEntries(
  Object.entries({
    live: 'live-flag',
    liveText: 'live-text',
    demoBanner: 'demo-banner',
    cpuPercent: 'cpu-percent',
    cpuCores: 'cpu-cores',
    spark: 'cpu-spark',
    memoryText: 'memory-text',
    memoryBar: 'memory-bar',
    netRx: 'net-rx',
    netTx: 'net-tx',
    diskText: 'disk-text',
    batteryText: 'battery-text',
    uptimeText: 'uptime-text',
    errorText: 'error-text',
  }).map(([key, id]) => [key, $(id)]),
);
const state = {
  demo: !monitorApi.available(),
  history: [],
  lastSample: null,
  timer: null,
  loading: false,
  stats: null,
  supported: false,
};
const preferences = {
  theme: 'dark',
  disk: '',
  network: true,
  battery: true,
  uptime: true,
  showDisk: true,
};
const view = createMonitorView(els, state, preferences);
try {
  const saved = JSON.parse(localStorage.getItem('pinpage-monitor-v1') || '{}');
  if (['light', 'dark'].includes(saved?.theme)) preferences.theme = saved.theme;
  if (typeof saved?.disk === 'string' && saved.disk.length <= 100) preferences.disk = saved.disk;
  for (const key of ['network', 'battery', 'uptime', 'showDisk'])
    if (typeof saved?.[key] === 'boolean') preferences[key] = saved[key];
} catch {
  /* 默认指标仍可使用。 */
}

/** 展示偏好只影响页面，始终使用宿主同一份共享采样。 */
function apply() {
  document.documentElement.dataset.theme = preferences.theme;
  $('monitor-theme').value = preferences.theme;
  for (const key of ['network', 'battery', 'uptime', 'showDisk'])
    $(`show-${key}`).checked = preferences[key];
  for (const element of document.querySelectorAll('.network-item'))
    element.hidden = !preferences.network;
  els.diskText.hidden = !preferences.showDisk;
  els.batteryText.hidden = !preferences.battery;
  els.uptimeText.hidden = !preferences.uptime;
}

/** 保存失败时保留当前指标设置并在设置面板中提示。 */
function persist() {
  try {
    localStorage.setItem('pinpage-monitor-v1', JSON.stringify(preferences));
    $('settings-status').textContent = '设置已保存';
  } catch {
    $('settings-status').textContent = '设置保存失败，当前外观仅临时生效';
  }
}

/** 同步样本时效，错误时保留上一轮可用读数。 */
function setLive(ok, text) {
  els.live.classList.toggle('stale', !ok);
  els.liveText.textContent = text;
}

/** 串行读取共享缓存；恢复可见时不与尚未完成的请求重叠。 */
async function tick() {
  if (state.loading || (!state.demo && !state.supported)) return;
  clearTimeout(state.timer);
  state.loading = true;
  try {
    state.stats = state.demo ? demoStats() : await monitorApi.stats();
    view.render(state.stats);
    els.errorText.hidden = true;
    const fresh = sampleIsFresh(state.stats.sampledAt);
    setLive(fresh, state.demo ? '演示' : fresh ? '实时' : '数据已过期');
    els.live.title = `采样时间：${new Date(state.stats.sampledAt).toLocaleTimeString('zh-CN')}`;
  } catch (error) {
    setLive(false, error?.code === 'errors.statsUnavailable' ? '等待采样' : '读取失败');
    els.errorText.textContent = '读取失败，保留上次数据并重试';
    els.errorText.hidden = error?.code === 'errors.statsUnavailable';
  } finally {
    state.loading = false;
    state.timer = setTimeout(tick, document.visibilityState === 'visible' ? 1000 : 5000);
  }
}

/** 宿主缺少能力时展示不可用，普通浏览器才展示演示数据。 */
async function initialize() {
  apply();
  if (state.demo) els.demoBanner.hidden = false;
  else {
    try {
      state.supported = (await monitorApi.hostInfo()).capabilities?.systemStats >= 1;
    } catch {
      state.supported = false;
    }
    if (!state.supported) {
      setLive(false, '能力不可用');
      els.errorText.hidden = false;
      els.errorText.textContent = '当前宿主未提供系统采样能力';
      return;
    }
  }
  await tick();
}
$('monitor-settings').addEventListener('click', () => $('settings-dialog').showModal());
$('close-settings').addEventListener('click', () => $('settings-dialog').close());
$('monitor-theme').addEventListener('change', (event) => {
  preferences.theme = event.target.value;
  apply();
  persist();
});
$('disk-select').addEventListener('change', (event) => {
  preferences.disk = event.target.value;
  persist();
  if (state.stats) view.render(state.stats);
});
for (const key of ['network', 'battery', 'uptime', 'showDisk'])
  $(`show-${key}`).addEventListener('change', (event) => {
    preferences[key] = event.target.checked;
    apply();
    persist();
  });
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') void tick();
});
window.addEventListener('pageshow', () => {
  if (state.stats) void tick();
});
window.addEventListener('resize', view.drawSpark);
void initialize();
