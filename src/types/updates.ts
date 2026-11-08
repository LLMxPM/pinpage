// 功能：定义升级 IPC 快照、偏好与动作，与 Rust 升级状态保持一致。
import type { AppMessage } from './messages';

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'upToDate'
  | 'available'
  | 'downloading'
  | 'verifying'
  | 'ready'
  | 'installing'
  | 'ignored'
  | 'error';

export type UpdateAction =
  'check' | 'download' | 'install' | 'ignore' | 'autoCheck' | 'openRelease';

export interface UpdateSnapshot {
  currentVersion: string;
  supported: boolean;
  status: UpdateStatus;
  latestVersion: string | null;
  notes: string | null;
  downloaded: number;
  total: number | null;
  error: AppMessage | null;
  preferences: {
    autoCheck: boolean;
    lastCheck: number | null;
    ignoredVersion: string | null;
  };
}
