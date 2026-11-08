// 功能：把窗口预设转换为界面显示名与尺寸摘要，并判断外观字段是否与某个预设一致。
import { i18n } from '../../i18n';
import type { WindowPreset } from '../../types';

/// 内置预设的显示名来自语言资源，自定义预设使用用户输入的名称。
export function presetLabel(preset: WindowPreset): string {
  if (preset.name) return preset.name;
  const key = `native.window.presets.${preset.id}`;
  const { t, te } = i18n.global;
  return te(key) ? String(t(key)) : preset.id;
}

/// 同名预设用尺寸区分：全屏显示换算说明，其余显示保存时的窗口宽高。
export function presetSummary(preset: WindowPreset): string {
  if (preset.fullscreen) return String(i18n.global.t('ui.presets.fullscreen'));
  return `${Math.round(preset.width)} × ${Math.round(preset.height)}`;
}
