// 功能：校验和保存时钟外观偏好，迁移旧版秒数设置，管理可滚动的原生设置对话框。
const storageKey = 'pinpage-clock-preferences-v2';
const defaults = {
  style: 'digital',
  theme: 'light',
  surface: 'bare',
  seconds: false,
  date: true,
  hour12: false,
};
const choices = {
  style: ['digital', 'flip', 'analog'],
  theme: ['light', 'dark', 'sage'],
  surface: ['bare', 'card'],
};

/// 仅接收已知枚举及布尔值；旧版偏好保留，损坏或不可访问的存储回退为默认值。
function restore() {
  const result = { ...defaults };
  try {
    const raw = localStorage.getItem(storageKey);
    if (raw === null) {
      result.seconds = localStorage.getItem('pinpage-clock-seconds-v1') === 'true';
      return result;
    }
    const saved = JSON.parse(raw);
    if (!saved || typeof saved !== 'object' || Array.isArray(saved)) return result;
    for (const [key, values] of Object.entries(choices)) {
      if (values.includes(saved[key])) result[key] = saved[key];
    }
    for (const key of ['seconds', 'date', 'hour12']) {
      if (typeof saved[key] === 'boolean') result[key] = saved[key];
    }
  } catch {
    // 偏好读取失败不阻断时钟和临时设置。
  }
  return result;
}

export const preferences = restore();

/// 应用外观只修改本工具的数据属性，绝不写入宿主透明变量。
function apply() {
  for (const [key, value] of Object.entries(preferences))
    document.documentElement.dataset[key] = String(value);
}

/// 初始化表单与对话框，变更立即保存并通知时钟重新读取系统时间。
export function initializeSettings(onChange) {
  const dialog = document.querySelector('#settings-dialog');
  const form = document.querySelector('#clock-settings');
  const open = document.querySelector('#open-settings');
  const close = document.querySelector('#close-settings');
  for (const [key, value] of Object.entries(preferences)) {
    const control = form.elements.namedItem(key);
    if (typeof value === 'boolean') control.checked = value;
    else control.value = value;
  }
  apply();
  open.addEventListener('click', () => dialog.showModal());
  close.addEventListener('click', () => dialog.close());
  dialog.addEventListener('click', (event) => {
    const bounds = dialog.getBoundingClientRect();
    if (
      event.target === dialog &&
      (event.clientX < bounds.left ||
        event.clientX > bounds.right ||
        event.clientY < bounds.top ||
        event.clientY > bounds.bottom)
    )
      dialog.close();
  });
  form.addEventListener('submit', (event) => event.preventDefault());
  form.addEventListener('change', (event) => {
    const control = event.target;
    const key = control.name;
    if (!Object.hasOwn(defaults, key)) return;
    if (choices[key] && !choices[key].includes(control.value)) return;
    preferences[key] = typeof defaults[key] === 'boolean' ? control.checked : control.value;
    apply();
    try {
      localStorage.setItem(storageKey, JSON.stringify(preferences));
      document.querySelector('#save-status').textContent = '已保存 · 随窗口大小调整布局';
    } catch {
      document.querySelector('#save-status').textContent =
        '当前设置已生效，存储不可用，重新打开后可能恢复默认。';
    }
    onChange();
  });
}
