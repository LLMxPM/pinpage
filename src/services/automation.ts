// 功能：集中封装受信任工作台的本机控制与技能管理调用。
import { invoke } from '@tauri-apps/api/core';
export interface AutomationStatus {
  localControl: boolean;
  builtinVersion: string;
  installedVersion: string | null;
  target?: string;
  cliPath?: string;
  error?: string;
}
/// 读取不含凭据的本机控制和默认技能状态。
export function automationSnapshot(): Promise<AutomationStatus> {
  return invoke('automation_snapshot');
}
/// 明确保存用户选择的开关，失败时界面保留旧值。
export function automationControl(enabled: boolean): Promise<void> {
  return invoke('automation_control', { enabled });
}
/// 原生选择器决定自定义目录，结果返回最终版本、目标和是否变更。
export function automationSkill(
  action: string,
  custom: boolean,
): Promise<Record<string, unknown> | null> {
  return invoke('automation_skill', { action, custom });
}
