// 功能：定义管理界面与 Rust IPC 共用的数据契约。
export type PageSource =
  | { kind: 'url'; url: string }
  | { kind: 'local'; entry: string; root: string }
  | { kind: 'example' };

export type SourceInput =
  { kind: 'url'; address: string } | { kind: 'local'; ticket: string } | { kind: 'example' };

export interface PinConfig {
  id: string;
  name: string;
  source: PageSource;
  width: number;
  height: number;
  x: number | null;
  y: number | null;
  scale: number;
  alwaysOnTop: boolean;
  decorations: boolean;
}

export interface PinRuntime {
  open: boolean;
  visible: boolean;
  loading: boolean;
  currentUrl: string;
  error: string | null;
}

export interface SettingsInput {
  restoreSession: boolean;
  managerShortcut: string;
  recoverShortcut: string;
}

export interface Settings extends SettingsInput {
  schemaVersion: number;
  pins: PinConfig[];
}

export interface Snapshot {
  settings: Settings;
  runtime: Record<string, PinRuntime>;
  notices: string[];
}

export interface CreatePin {
  name: string;
  source: SourceInput;
  alwaysOnTop: boolean;
  decorations: boolean;
}

export interface LocalChoice {
  ticket: string;
  entry: string;
  root: string;
}
