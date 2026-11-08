// 功能：定义管理界面与 Rust IPC 共用的数据契约。
import type { AppMessage } from './messages';

export type { AppMessage, UiMessage } from './messages';

/// 用户语言偏好，与 Rust `LocalePreference` 一致。
export type LocalePreference = 'zh-CN' | 'en-US' | 'system';

/// Rust 已解析的有效语言。
export type AppLocale = 'zh-CN' | 'en-US';

/// 窗口左右鼠标手势共用的修饰键，与 Rust `GestureModifier` 一致。
export type GestureModifier = 'alt' | 'ctrl' | 'shift' | 'disabled';

/// 内置示例来源，与 Rust 的 ExampleId 白名单一致，不接受任意资源路径。
export type ExampleId = 'timer' | 'notes' | 'checklist' | 'clock';

export type PageSource = { kind: 'url'; url: string } | { kind: 'bundled'; entry: string };

export type SourceInput =
  { kind: 'url'; address: string } | { kind: 'import'; ticket: string; entry: string };

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
  enabled: boolean;
  mode: 'floating' | 'desktop';
  rounded: boolean;
  positionLocked: boolean;
  sizeLocked: boolean;
}

export interface PinSettingsInput {
  name: string;
  mode: PinConfig['mode'];
  alwaysOnTop: boolean;
  decorations: boolean;
  rounded: boolean;
  positionLocked: boolean;
  sizeLocked: boolean;
  width: number;
  height: number;
  x: number;
  y: number;
  /// 省略表示沿用现有来源；只有用户改动地址或重新导入时才提交。
  source?: SourceInput;
}

export interface PinRuntime {
  open: boolean;
  visible: boolean;
  loading: boolean;
  currentUrl: string;
  /// 只携带消息码与参数，切换语言后按当前语言重新渲染。
  error: AppMessage | null;
}

export interface SettingsInput {
  locale: LocalePreference;
  gestureModifier: GestureModifier;
  restoreSession: boolean;
}

export interface Settings extends SettingsInput {
  schemaVersion: number;
  pins: PinConfig[];
}

export interface Snapshot {
  settings: Settings;
  runtime: Record<string, PinRuntime>;
  notices: AppMessage[];
  /// 启动时由 Rust 解析，前端不再自行猜测系统语言。
  resolvedLocale: AppLocale;
}

export interface CreatePin {
  name: string;
  source: SourceInput;
  alwaysOnTop: boolean;
  decorations: boolean;
  mode?: PinConfig['mode'];
}

/// 原生选择器扫描后的导入预览，用户确认入口后才真正复制文件。
export interface ImportPreview {
  ticket: string;
  mode: 'directory' | 'file' | 'package' | 'example';
  label: string;
  candidates: string[];
  files: number;
  bytes: number;
}

/// 导出结果；withData 为 false 表示只打包了站点副本。
export interface ExportResult {
  path: string;
  files: number;
  withData: boolean;
  /// 页面数据快照的字节数，未包含数据时为 0。
  dataBytes: number;
}
