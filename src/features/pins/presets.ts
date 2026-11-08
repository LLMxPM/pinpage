// 功能：把窗口预设转换为界面显示名，并判断外观字段是否与某个预设一致。
import { i18n } from '../../i18n';
import type { WindowPreset } from '../../types';

/// 内置预设的显示名来自语言资源，自定义预设使用用户输入的名称。
export function presetLabel(preset: WindowPreset): string {
  if (preset.name) return preset.name;
  const key = `native.window.presets.${preset.id}`;
  const { t, te } = i18n.global;
  return te(key) ? String(t(key)) : preset.id;
}
