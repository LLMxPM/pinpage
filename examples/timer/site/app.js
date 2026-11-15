// 功能：组织倒计时、秒表和时钟切换，保存运行状态并提供可选的页内完成提示音。
import {
  restoreTimer,
  remainingAt,
  settleTimer,
  restoreStopwatch,
  elapsedAt,
  formatDuration,
} from './time-state.js';
import { recordSession, renderRecords } from './records.js';
import './clock.js';

/** 获取工具的固定控件。 */
const $ = (selector) => document.querySelector(selector);
const TIMER_KEY = 'pinpage-timer-v1';
const TOOLS_KEY = 'pinpage-time-tools-v1';

/** 存储损坏时保持工具可用，保存错误由状态区提示。 */
function read(key) {
  try {
    return JSON.parse(localStorage.getItem(key) || 'null');
  } catch {
    return null;
  }
}
const timer = restoreTimer(read(TIMER_KEY));
const saved = read(TOOLS_KEY);
const stopwatch = restoreStopwatch(saved?.stopwatch);
let mode = ['timer', 'clock', 'stopwatch'].includes(saved?.mode)
  ? saved.mode
  : !read(TIMER_KEY) && hasClockPreferences()
    ? 'clock'
    : 'timer';
let sound = saved?.sound === true;
let audio;
let refreshTimer;
let recordDay = new Date().toDateString();
let pendingPreset = null;

/** 旧时钟副本更新为合并工具时，默认仍展示时钟。 */
function hasClockPreferences() {
  try {
    return (
      localStorage.getItem('pinpage-clock-preferences-v2') !== null ||
      localStorage.getItem('pinpage-clock-seconds-v1') !== null
    );
  } catch {
    return false;
  }
}

/** 操作后立即保存，失败时告知当前状态尚未持久化。 */
function persist() {
  try {
    localStorage.setItem(TIMER_KEY, JSON.stringify(timer));
    localStorage.setItem(TOOLS_KEY, JSON.stringify({ mode, sound, stopwatch }));
    $('#timer-save-status').textContent = '';
  } catch {
    $('#timer-save-status').textContent = '本地保存失败，关闭后可能丢失当前状态';
  }
}

/** 音频只在用户开启后解锁；不支持时保留视觉提示。 */
function unlockSound() {
  if (!sound) return;
  try {
    audio ??= new AudioContext();
    void audio.resume().catch(() => {});
  } catch {
    /* 保留视觉提示。 */
  }
}

/** 在已经解锁的音频上下文中播放短提示音。 */
function playSound() {
  if (!sound || audio?.state !== 'running') return;
  const oscillator = audio.createOscillator();
  const gain = audio.createGain();
  oscillator.frequency.value = 660;
  gain.gain.setValueAtTime(0.12, audio.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.001, audio.currentTime + 0.7);
  oscillator.connect(gain).connect(audio.destination);
  oscillator.start();
  oscillator.stop(audio.currentTime + 0.7);
}

/** 先结算再处理暂停，避免到期瞬间漏记；完成信号只产生一次。 */
function settle() {
  const finishedAt = timer.deadline;
  if (!settleTimer(timer, Date.now())) return;
  persist();
  void recordSession({ ...timer }, finishedAt);
  playSound();
  $('#completion').textContent =
    timer.kind === 'rest' ? '休息结束，可以开始下一轮了' : '本轮专注已完成，休息一下吧';
  $('#completion').hidden = false;
}

/** 仅更新变化的文本，减少常驻页面的无效绘制。 */
function setText(selector, value) {
  if ($(selector).textContent !== value) $(selector).textContent = value;
}

/** 按真实时间更新计时，页面切换不会暂停正在运行的任务。 */
function render() {
  clearTimeout(refreshTimer);
  settle();
  const value = remainingAt(timer, Date.now());
  setText('#time', formatDuration(value));
  setText(
    '#toggle',
    timer.deadline !== null
      ? '暂停'
      : timer.remaining === 0
        ? '再来一次'
        : timer.kind === 'rest'
          ? '开始休息'
          : '开始专注',
  );
  setText(
    '#status',
    timer.deadline !== null
      ? timer.kind === 'rest'
        ? '正在休息'
        : '专注进行中'
      : timer.remaining === 0
        ? '已完成'
        : timer.remaining < timer.duration
          ? '已暂停'
          : '准备就绪',
  );
  $('#timer-panel').classList.toggle('finished', timer.remaining === 0);
  $('#tab-timer').classList.toggle('has-completed', timer.remaining === 0);
  $('#tab-timer').setAttribute('aria-label', timer.remaining === 0 ? '计时，已完成' : '计时');
  setText(
    '#stopwatch-time',
    formatDuration(Math.floor(elapsedAt(stopwatch, Date.now()) / 1000) * 1000),
  );
  setText('#stopwatch-toggle', stopwatch.startedAt === null ? '开始 / 继续' : '暂停');
  if (mode !== 'clock')
    document.title = `${mode === 'timer' ? formatDuration(value) : $('#stopwatch-time').textContent} · 时间工具`;
  const today = new Date().toDateString();
  if (today !== recordDay) {
    recordDay = today;
    void renderRecords();
  }
  refreshTimer = setTimeout(render, 1000 - (Date.now() % 1000) + 15);
}

/** 模式独立于窗口布局保存，标签页支持标准方向键导航。 */
function selectMode(next) {
  mode = next;
  document.documentElement.dataset.mode = mode;
  for (const button of document.querySelectorAll('[data-mode-button]')) {
    const active = button.dataset.modeButton === mode;
    button.setAttribute('aria-selected', String(active));
    button.tabIndex = active ? 0 : -1;
  }
  for (const panel of document.querySelectorAll('[data-panel]'))
    panel.hidden = panel.dataset.panel !== mode;
  if (mode === 'clock') document.title = '桌面时钟 · 时间工具';
  persist();
  render();
}

/** 运行中再次选择相同时长才重置，避免误触丢失进度。 */
function setPreset(minutes, kind = 'focus') {
  settle();
  if (!Number.isFinite(minutes) || minutes < 1 || minutes > 1440) return;
  if (timer.deadline !== null && pendingPreset !== `${minutes}:${kind}`) {
    pendingPreset = `${minutes}:${kind}`;
    $('#timer-save-status').textContent = '正在计时，再次选择相同时长将重置本轮';
    return;
  }
  pendingPreset = null;
  Object.assign(timer, {
    duration: minutes * 60000,
    remaining: minutes * 60000,
    deadline: null,
    kind,
  });
  $('#completion').hidden = true;
  $('#custom-minutes').value = minutes;
  for (const button of document.querySelectorAll('#presets button'))
    button.setAttribute(
      'aria-pressed',
      String(Number(button.dataset.minutes) === minutes && button.dataset.kind === kind),
    );
  persist();
  render();
}

$('#toggle').addEventListener('click', () => {
  unlockSound();
  settle();
  pendingPreset = null;
  if (timer.deadline !== null) {
    timer.remaining = remainingAt(timer, Date.now());
    timer.deadline = null;
  } else {
    if (!timer.remaining) timer.remaining = timer.duration;
    timer.deadline = Date.now() + timer.remaining;
    $('#completion').hidden = true;
  }
  persist();
  render();
});
$('#reset').addEventListener('click', () => setPreset(timer.duration / 60000, timer.kind));
$('#custom-duration').addEventListener('submit', (event) => {
  event.preventDefault();
  setPreset(Number($('#custom-minutes').value));
});
$('#timer-sound').checked = sound;
$('#timer-sound').addEventListener('change', (event) => {
  sound = event.target.checked;
  unlockSound();
  persist();
});
$('#stopwatch-toggle').addEventListener('click', () => {
  if (stopwatch.startedAt === null) stopwatch.startedAt = Date.now();
  else {
    stopwatch.elapsed = elapsedAt(stopwatch, Date.now());
    stopwatch.startedAt = null;
  }
  persist();
  render();
});
$('#stopwatch-reset').addEventListener('click', () => {
  stopwatch.elapsed = 0;
  stopwatch.startedAt = null;
  persist();
  render();
});
for (const button of document.querySelectorAll('[data-mode-button]')) {
  button.addEventListener('click', () => selectMode(button.dataset.modeButton));
  button.addEventListener('keydown', (event) => {
    const buttons = [...document.querySelectorAll('[data-mode-button]')];
    const index = buttons.indexOf(button);
    const next =
      event.key === 'ArrowRight'
        ? (index + 1) % buttons.length
        : event.key === 'ArrowLeft'
          ? (index + buttons.length - 1) % buttons.length
          : event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? buttons.length - 1
              : -1;
    if (next < 0) return;
    event.preventDefault();
    selectMode(buttons[next].dataset.modeButton);
    buttons[next].focus();
  });
}

/** 相对路径加载固定预设，失败时仍保留自定义时长入口。 */
async function loadPresets() {
  try {
    const response = await fetch('./presets.json');
    if (!response.ok) throw new Error();
    for (const preset of await response.json()) {
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent = preset.label;
      button.dataset.minutes = preset.minutes;
      button.dataset.kind = preset.kind;
      button.setAttribute(
        'aria-pressed',
        String(timer.duration === preset.minutes * 60000 && timer.kind === preset.kind),
      );
      button.addEventListener('click', () => setPreset(preset.minutes, preset.kind));
      $('#presets').append(button);
    }
  } catch {
    $('#timer-save-status').textContent = '预设读取失败，可使用自定义时长';
  }
}
document.addEventListener('visibilitychange', render);
window.addEventListener('pageshow', render);
$('#custom-minutes').value = timer.duration / 60000;
selectMode(mode);
void loadPresets();
void renderRecords();
