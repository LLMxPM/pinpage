// 功能：按系统真实时间更新三种时钟，仅刷新变化的内容，并在页面恢复或休眠后校正。
import { initializeSettings, preferences } from './settings.js';

const accessible = document.querySelector('#clock-accessible');
const date = document.querySelector('#clock-date');
const weekday = document.querySelector('#clock-weekday');
const period = document.querySelector('#clock-period');
const digits = Object.fromEntries(
  ['digital', 'flip'].map((style) => [
    style,
    Object.fromEntries(
      ['hour', 'minute', 'second'].map((unit) => [
        unit,
        document.querySelector(`#${style}-${unit}`),
      ]),
    ),
  ]),
);
const hands = Object.fromEntries(
  ['hour', 'minute', 'second'].map((unit) => [unit, document.querySelector(`#${unit}-hand`)]),
);
const dateFormat = new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric' });
const fullDateFormat = new Intl.DateTimeFormat('zh-CN', {
  year: 'numeric',
  month: 'long',
  day: 'numeric',
});
const weekdayFormat = new Intl.DateTimeFormat('zh-CN', { weekday: 'long' });
let refreshTimer;

/// 内容相同时跳过 DOM 写入，避免常驻页面无效绘制。
function update(target, value) {
  if (target.textContent !== value) target.textContent = value;
}

/// 仅在指针角度变化时写入 SVG 属性，不使用高频动画或扫秒循环。
function rotate(target, angle) {
  const value = `rotate(${angle} 120 120)`;
  if (target.getAttribute('transform') !== value) target.setAttribute('transform', value);
}

/// 生成静态表盘刻度，一次创建后不再重建。
function initializeDial() {
  const ticks = document.querySelector('#dial-ticks');
  const fragment = document.createDocumentFragment();
  for (let index = 0; index < 60; index += 1) {
    const tick = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    const major = index % 5 === 0;
    for (const [name, value] of Object.entries({
      x1: 120,
      y1: major ? 14 : 16,
      x2: 120,
      y2: major ? 23 : 20,
      transform: `rotate(${index * 6} 120 120)`,
      class: `dial-tick${major ? ' major' : ''}`,
    }))
      tick.setAttribute(name, String(value));
    fragment.append(tick);
  }
  ticks.append(fragment);
}

/// 每轮读取真实时间；不显示秒数时对齐下一分钟，显示时对齐下一秒，后台也保持计时。
function render() {
  clearTimeout(refreshTimer);
  const now = new Date();
  const hour = now.getHours();
  const minute = now.getMinutes();
  const second = now.getSeconds();
  const values = {
    hour: String(preferences.hour12 ? hour % 12 || 12 : hour).padStart(2, '0'),
    minute: String(minute).padStart(2, '0'),
    second: String(second).padStart(2, '0'),
  };
  if (preferences.style === 'analog') {
    rotate(hands.hour, (hour % 12) * 30 + minute * 0.5 + (preferences.seconds ? second / 120 : 0));
    rotate(hands.minute, minute * 6 + (preferences.seconds ? second * 0.1 : 0));
    if (preferences.seconds) rotate(hands.second, second * 6);
  } else {
    for (const [unit, value] of Object.entries(values)) {
      if (unit !== 'second' || preferences.seconds) update(digits[preferences.style][unit], value);
    }
  }
  update(date, dateFormat.format(now));
  update(weekday, weekdayFormat.format(now));
  period.hidden = !preferences.hour12;
  update(period, hour < 12 ? '上午' : '下午');
  const timeText = `${values.hour}:${values.minute}${preferences.seconds ? `:${values.second}` : ''}`;
  update(
    accessible,
    `${fullDateFormat.format(now)} ${weekdayFormat.format(now)} ${preferences.hour12 ? period.textContent + ' ' : ''}${timeText}`,
  );
  const stamp = now.toISOString().slice(0, preferences.seconds ? 19 : 16);
  const dateTime = `${stamp}${preferences.seconds ? '' : ':00'}Z`;
  if (accessible.getAttribute('datetime') !== dateTime)
    accessible.setAttribute('datetime', dateTime);
  const interval = preferences.seconds ? 1000 : 60000;
  refreshTimer = setTimeout(render, interval - (Date.now() % interval) + 20);
}

initializeDial();
initializeSettings(render);
document.addEventListener('visibilitychange', render);
window.addEventListener('pageshow', render);
window.addEventListener('focus', render);
render();
