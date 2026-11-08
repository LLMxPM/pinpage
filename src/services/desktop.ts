// 功能：集中封装桌面 IPC，浏览器预览不伪装为已连接的原生应用。
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  CreatePin,
  ExampleId,
  ExportResult,
  ImportPreview,
  PinSettingsInput,
  PinContentInput,
  WindowPatch,
  WindowChange,
  PresetInput,
  SettingsInput,
  Snapshot,
} from '../types';

export const desktopAvailable = isTauri();

/// 读取 Rust 层持有的配置及窗口运行状态。
export function getSnapshot(): Promise<Snapshot> {
  return invoke('snapshot');
}

/// 打开原生选择器导入网页；内置示例不经过选择器，直接返回预览。
export function importPage(
  mode: ImportPreview['mode'],
  example?: ExampleId,
): Promise<ImportPreview | null> {
  return invoke('import_page', { mode, example });
}

/// 把导入页的站点副本与页面数据导出为页面包，取消保存时返回 null。
export function exportPage(id: string): Promise<ExportResult | null> {
  return invoke('export_page', { id });
}

/// 创建一个独立悬浮窗，本地来源使用选择器授权票据。
export function createPin(input: CreatePin): Promise<void> {
  return invoke('create_pin', { input });
}

/// 请求原生窗口操作，实际权限与目标有效性由 Rust 再次校验。
export function pinAction(id: string, action: string): Promise<void> {
  return invoke('pin_action', { id, action });
}

/// 保存单页的桌面模式、圆角、几何位置与锁定设置。
export function updatePin(id: string, input: PinSettingsInput): Promise<void> {
  return invoke('update_pin', { id, input });
}

/// 把页面当前外观与位置存为自定义预设，返回新预设 ID。
export function savePreset(id: string, name: string, presetId?: string): Promise<string> {
  return invoke('save_preset', { id, name, presetId: presetId ?? null });
}

/// 局部应用窗口设置，返回原生层确认的最终状态。
export function updateWindow(
  id: string,
  input: WindowPatch,
  expected?: WindowPatch,
): Promise<WindowChange> {
  return invoke('update_window', { id, input, expected: expected ?? null });
}

/// 内容单独保存，来源未变时不重新加载页面。
export function updateContent(id: string, input: PinContentInput): Promise<void> {
  return invoke('update_content', { id, input });
}

/// 编辑自定义预设或创建副本，不联动现有窗口。
export function writePreset(input: PresetInput, presetId?: string): Promise<string> {
  return invoke('write_preset', { input, presetId: presetId ?? null });
}

/// 删除自定义预设；内置预设由 Rust 拒绝删除。
export function deletePreset(presetId: string): Promise<void> {
  return invoke('delete_preset', { presetId });
}

/// 更新语言、启动恢复与窗口手势。
export function updateSettings(input: SettingsInput): Promise<void> {
  return invoke('update_settings', { input });
}

/// 退出应用并保留仍打开的会话。
export function quitApp(): Promise<void> {
  return invoke('quit_app');
}

/// 订阅仅发往管理界面的状态通知，返回生命周期清理函数。
export function onPinsChanged(callback: () => void) {
  return listen('pins-changed', callback);
}
