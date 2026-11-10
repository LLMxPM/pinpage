// 功能：轮询共享系统采样并渲染面板；可见时每秒刷新，隐藏时降为每 5 秒。
import { monitorApi } from './monitor-api.js';

const HISTORY_LIMIT = 60;
const VISIBLE_DELAY = 1000;
const HIDDEN_DELAY = 5000;

const els = {
  live: document.getElementById('live-flag'),
  liveText: document.getElementById('live-text'),
  demoBanner: document.getElementById('demo-banner'),
  cpuPercent: document.getElementById('cpu-percent'),
  cpuCores: document.getElementById('cpu-cores'),
  spark: document.getElementById('cpu-spark'),
  memoryText: document.getElementById('memory-text'),
  memoryBar: document.getElementById('memory-bar'),
  netRx: document.getElementById('net-rx'),
  netTx: document.getElementById('net-tx'),
  diskText: document.getElementById('disk-text'),
  batteryText: document.getElementById('battery-text'),
  uptimeText: document.getElementById('uptime-text'),
  errorText: document.getElementById('error-text'),
};

const state = {
  demo: !monitorApi.available(),
  history: [],
  timer: null,
  demoTick: 0,
};

/** 仅在文本变化时写 DOM，避免每秒重复布局。 */
function setText(el, text) {
  if (el.textContent !== text) el.textContent = text;
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit = -1;
  do {
    value /= 1024;
    unit += 1;
  } while (value >= 1024 && unit < units.length - 1);
  return `${value >= 100 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`;
}

function formatRate(bytesPerSec) {
  return `${formatBytes(bytesPerSec)}/s`;
}

/** 运行时长：超过一天显示天，否则显示小时与分钟。 */
function formatUptime(secs) {
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  if (days > 0) return `已运行 ${days} 天 ${hours} 小时`;
  if (hours > 0) return `已运行 ${hours} 小时 ${minutes} 分`;
  return `已运行 ${minutes} 分钟`;
}

/** 按设备像素比重设画布，保证高分屏曲线清晰。 */
function setupCanvas() {
  const ratio = window.devicePixelRatio || 1;
  const width = els.spark.clientWidth || 120;
  const height = els.spark.clientHeight || 34;
  els.spark.width = Math.round(width * ratio);
  els.spark.height = Math.round(height * ratio);
  const ctx = els.spark.getContext('2d');
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  return { ctx, width, height };
}

/** 绘制 CPU 迷你曲线；历史铺满画布宽度，缓冲满后按 60 点滚动。 */
function drawSpark() {
  const { ctx, width, height } = setupCanvas();
  ctx.clearRect(0, 0, width, height);
  if (state.history.length < 2) return;
  const span = Math.max(1, state.history.length - 1);
  ctx.beginPath();
  state.history.forEach((value, index) => {
    const x = (index / span) * width;
    const y = height - 2 - (Math.min(100, Math.max(0, value)) / 100) * (height - 4);
    if (index === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.strokeStyle = '#86c58c';
  ctx.lineWidth = 1.5;
  ctx.lineJoin = 'round';
  ctx.stroke();
}

/** 渲染一次采样快照。 */
function render(stats) {
  const cpu = Math.round(stats.cpu.usagePercent);
  setText(els.cpuPercent, String(cpu));
  setText(els.cpuCores, `${stats.cpu.cores} 核`);
  state.history.push(stats.cpu.usagePercent);
  if (state.history.length > HISTORY_LIMIT) state.history.shift();
  drawSpark();

  const memPercent = stats.memory.totalBytes
    ? (stats.memory.usedBytes / stats.memory.totalBytes) * 100
    : 0;
  setText(
    els.memoryText,
    `${formatBytes(stats.memory.usedBytes)} / ${formatBytes(stats.memory.totalBytes)}`,
  );
  els.memoryBar.style.width = `${Math.min(100, memPercent).toFixed(1)}%`;
  els.memoryBar.className = memPercent >= 90 ? 'hot' : memPercent >= 75 ? 'warn' : '';

  setText(els.netRx, formatRate(stats.network.rxBytesPerSec));
  setText(els.netTx, formatRate(stats.network.txBytesPerSec));
  const disk = stats.disks[0];
  setText(
    els.diskText,
    disk ? `${disk.label} ${formatBytes(disk.usedBytes)}/${formatBytes(disk.totalBytes)}` : '',
  );

  setText(
    els.batteryText,
    stats.battery
      ? `电池 ${stats.battery.percent}%${stats.battery.charging ? ' · 充电中' : ''}`
      : '电源已接通',
  );
  setText(els.uptimeText, formatUptime(stats.uptimeSecs));
}

/** 浏览器预览用的演示数据：缓慢波动的合成值，明确标注不读取真实系统。 */
function demoStats() {
  state.demoTick += 1;
  const t = state.demoTick;
  const wave = Math.sin(t / 6) * 14 + Math.sin(t / 2.3) * 5;
  const totalMemory = 16 * 1024 ** 3;
  return {
    sampledAt: Date.now(),
    cpu: { usagePercent: Math.max(2, Math.min(96, 30 + wave)), cores: 8 },
    memory: { totalBytes: totalMemory, usedBytes: Math.round(totalMemory * 0.56) },
    network: {
      rxBytesPerSec: Math.round(180000 + Math.abs(wave) * 22000),
      txBytesPerSec: Math.round(26000 + Math.abs(wave) * 4000),
    },
    disks: [{ label: 'C:', totalBytes: 512 * 1024 ** 3, usedBytes: 287 * 1024 ** 3 }],
    battery: { percent: 82, charging: true },
    uptimeSecs: 2 * 86400 + 5 * 3600 + t,
  };
}

/** 标记数据新鲜度；出错时保留上一次数值并提示。 */
function setLive(ok, label) {
  els.live.classList.toggle('stale', !ok);
  setText(els.liveText, label);
}

/** 单次拉取；启动初期采样未就绪时等待，不视为故障。 */
async function tick() {
  try {
    const stats = state.demo ? demoStats() : await monitorApi.stats();
    els.errorText.hidden = true;
    render(stats);
    setLive(true, state.demo ? '演示' : '实时');
  } catch (error) {
    const code = error && error.code ? error.code : 'errors.unknown';
    if (code === 'errors.statsUnavailable') {
      setLive(false, '等待采样');
    } else {
      setLive(false, '已暂停');
      setText(els.errorText, '读取系统数据失败，正在重试');
      els.errorText.hidden = false;
    }
  }
  schedule();
}

/** 按可见性安排下一次轮询；隐藏时降低频率但不中断。 */
function schedule() {
  clearTimeout(state.timer);
  const delay = document.visibilityState === 'visible' ? VISIBLE_DELAY : HIDDEN_DELAY;
  state.timer = setTimeout(tick, delay);
}

document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') {
    clearTimeout(state.timer);
    tick();
  } else {
    schedule();
  }
});
window.addEventListener('resize', drawSpark);

if (state.demo) {
  els.demoBanner.hidden = false;
} else {
  monitorApi
    .hostInfo()
    .then((info) => {
      els.live.title = `PinPage v${info.appVersion} · ${info.platform}`;
    })
    .catch(() => {});
}
tick();
