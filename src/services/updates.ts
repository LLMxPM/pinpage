// 功能：集中封装升级 IPC 与仅面向管理界面的状态订阅。
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UpdateAction, UpdateSnapshot } from '../types/updates';

/// 管理界面重建后读取原生任务，不启动新检查或下载。
export function getUpdateSnapshot(): Promise<UpdateSnapshot> {
  return invoke('updates_snapshot');
}

/// 仅提交固定动作，来源、签名与安装包全部由 Rust 管理。
export function updateAction(action: UpdateAction, enabled?: boolean): Promise<void> {
  return invoke('update_action', { action, enabled });
}

/// 先订阅再读取快照，避免错过启动阶段的更新通知。
export function onUpdatesChanged(callback: (snapshot: UpdateSnapshot) => void) {
  return listen<UpdateSnapshot>('updates-changed', (event) => callback(event.payload));
}
