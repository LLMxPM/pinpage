// 功能：合并原生窗口回传与设置草稿，更新未编辑字段并保留用户尚未提交的修改。
import { reactive, watch } from 'vue';
import type { PinConfig, PinSettingsInput, WindowValues } from '../../types';

/// 把窗口快照转换为表单值，逻辑尺寸取整，尚未定位的页面使用默认位置。
/// 预设标记不进入表单，避免“基于哪个预设”与可编辑字段混在一起。
export function pinToInput(pin: PinConfig): PinSettingsInput {
  return {
    name: pin.name,
    mode: pin.mode,
    alwaysOnTop: pin.alwaysOnTop,
    decorations: pin.decorations,
    rounded: pin.rounded,
    positionLocked: pin.positionLocked,
    sizeLocked: pin.sizeLocked,
    width: Math.round(pin.width),
    height: Math.round(pin.height),
    x: pin.x ?? 60,
    y: pin.y ?? 60,
  };
}

/// 只返回窗口可编辑字段，撤销与即时提交不会触碰内容和开启状态。
export function windowValues(pin: PinConfig): WindowValues {
  const { name: _name, source: _source, preset: _preset, ...values } = pinToInput(pin);
  return values;
}

/// 同步当前页面的可编辑字段；切换页面时重置，原页面回传时只更新未改动字段。
export function usePinSettingsForm(getPin: () => PinConfig) {
  let id = getPin().id;
  let baseline = pinToInput(getPin());
  const input = reactive<PinSettingsInput>({ ...baseline });
  const fields = Object.keys(baseline) as (keyof PinSettingsInput)[];

  /// 用户输入仍与上一快照相同时跟随更新，避免保存其他设置时还原旧几何信息。
  function syncField<K extends keyof PinSettingsInput>(key: K, next: PinSettingsInput) {
    if (Object.is(input[key], baseline[key])) input[key] = next[key];
  }

  watch(
    () => ({ id: getPin().id, values: pinToInput(getPin()) }),
    (next) => {
      if (next.id !== id) Object.assign(input, next.values);
      else for (const key of fields) syncField(key, next.values);
      id = next.id;
      baseline = next.values;
    },
  );
  return input;
}
