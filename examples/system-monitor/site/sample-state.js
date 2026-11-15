// 功能：判断监控样本时效，避免成功读取旧缓存后仍显示实时。
/** 超过五秒、未来异常或缺失的时间戳均不作为实时采样。 */
export function sampleIsFresh(sampledAt, now = Date.now()) {
  return Number.isFinite(sampledAt) && now - sampledAt >= -2000 && now - sampledAt <= 5000;
}
