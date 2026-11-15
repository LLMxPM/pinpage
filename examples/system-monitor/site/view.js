// 功能：渲染共享采样、真实采样点历史和磁盘选择，不执行额外系统采样。
/** 建立页面独立视图，只在数据变化时更新控件。 */
export function createMonitorView(els, state, preferences) {
  const HISTORY_LIMIT = 60;
  /** 仅在文本变化时写 DOM，避免每秒重复布局。 */
  function setText(el, text) {
    if (el.textContent !== text) el.textContent = text;
  }

  /** 将字节容量压缩为可扫读的数值与单位。 */
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

  /** 网络速率单位为每秒字节，明确区别于位速率。 */
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
    updateDisks(stats.disks);
    const cpu = Math.round(stats.cpu.usagePercent);
    setText(els.cpuPercent, String(cpu));
    setText(els.cpuCores, `${stats.cpu.cores} 核`);
    if (stats.sampledAt !== state.lastSample) {
      state.history.push(stats.cpu.usagePercent);
      state.lastSample = stats.sampledAt;
    }
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
    const disk = stats.disks.find((disk) => disk.label === preferences.disk) ?? stats.disks[0];
    setText(
      els.diskText,
      disk ? `${disk.label} ${formatBytes(disk.usedBytes)}/${formatBytes(disk.totalBytes)}` : '',
    );

    setText(
      els.batteryText,
      stats.battery
        ? `电池 ${stats.battery.percent}%${stats.battery.charging ? ' · 充电中' : ''}`
        : '未提供电池信息',
    );
    setText(els.uptimeText, formatUptime(stats.uptimeSecs));
  }

  /** 磁盘列表变化时才更新控件，避免每秒打断键盘选择。 */
  function updateDisks(disks) {
    const select = document.querySelector('#disk-select');
    const signature = JSON.stringify(disks.map((disk) => disk.label));
    if (select.dataset.signature === signature) return;
    select.dataset.signature = signature;
    select.replaceChildren();
    for (const disk of disks) {
      const option = document.createElement('option');
      option.value = disk.label;
      option.textContent = disk.label;
      select.append(option);
    }
    select.value = disks.some((disk) => disk.label === preferences.disk)
      ? preferences.disk
      : (disks[0]?.label ?? '');
    select.disabled = !disks.length;
  }

  return { render, drawSpark };
}
