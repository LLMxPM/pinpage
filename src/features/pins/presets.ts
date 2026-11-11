// 功能：把窗口预设转换为界面显示名与尺寸摘要，并判断外观字段是否与某个预设一致。
import { i18n } from '../../i18n';
import type { WindowPreset } from '../../types';

/// 预设均为用户自定义项，显示名直接使用用户输入的名称。
export function presetLabel(preset: WindowPreset): string {
  return preset.name;
}

/// 同名预设用尺寸区分：全屏显示换算说明，其余显示保存时的窗口宽高。
export function presetSummary(preset: WindowPreset): string {
  if (preset.fullscreen) return String(i18n.global.t('ui.presets.fullscreen'));
  return `${Math.round(preset.width)} × ${Math.round(preset.height)}`;
}
