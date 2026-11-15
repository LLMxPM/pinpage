// 功能：校验计时与秒表状态，按真实时间计算流逝量，兼容旧版计时器存储。
export const MAX_DURATION = 86400000;

/** 恢复旧版计时器字段，补全专注类型；异常数据使用独立默认状态。 */
export function restoreTimer(raw) {
  const fallback = { duration: 1500000, remaining: 1500000, deadline: null, kind: 'focus' };
  if (
    !raw ||
    !Number.isFinite(raw.duration) ||
    raw.duration <= 0 ||
    raw.duration > MAX_DURATION ||
    !Number.isFinite(raw.remaining) ||
    raw.remaining < 0 ||
    raw.remaining > raw.duration ||
    (raw.deadline !== null &&
      (!Number.isFinite(raw.deadline) || raw.deadline < 0 || raw.deadline > 8640000000000000))
  )
    return fallback;
  return {
    duration: raw.duration,
    remaining: raw.remaining,
    deadline: raw.deadline,
    kind: raw.kind === 'rest' ? 'rest' : 'focus',
  };
}

/** 休眠后仍以截止时间计算剩余量，回调不参与时间累加。 */
export function remainingAt(state, now) {
  return state.deadline === null ? state.remaining : Math.max(0, state.deadline - now);
}

/** 结算到期状态且只返回一次完成信号，供调用方持久化与记录。 */
export function settleTimer(state, now) {
  if (state.deadline === null || remainingAt(state, now) > 0) return false;
  state.deadline = null;
  state.remaining = 0;
  return true;
}

/** 只接受有限、非负的秒表状态，开始时间用于关闭重开后的恢复。 */
export function restoreStopwatch(raw) {
  return {
    elapsed:
      Number.isFinite(raw?.elapsed) && raw.elapsed >= 0 && raw.elapsed <= 31536000000
        ? raw.elapsed
        : 0,
    startedAt:
      Number.isFinite(raw?.startedAt) && raw.startedAt >= 0 && raw.startedAt <= Date.now()
        ? raw.startedAt
        : null,
  };
}

/** 展示时钳制系统时间回拨，避免秒表出现负数。 */
export function elapsedAt(state, now) {
  return state.elapsed + (state.startedAt === null ? 0 : Math.max(0, now - state.startedAt));
}

/** 将毫秒转换成时间字符串，超过一小时显示小时。 */
export function formatDuration(milliseconds) {
  const total = Math.max(0, Math.ceil(milliseconds / 1000));
  const seconds = String(total % 60).padStart(2, '0');
  const minutes = String(Math.floor(total / 60) % 60).padStart(2, '0');
  return total >= 3600
    ? `${Math.floor(total / 3600)}:${minutes}:${seconds}`
    : `${minutes}:${seconds}`;
}
