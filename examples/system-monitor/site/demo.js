// 功能：为普通浏览器提供显式标记的监控演示数据。
let state = { demoTick: 0 };
/** 返回一轮合成采样，不读取真实系统。 */
export function demoStats() {
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
