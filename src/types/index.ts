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
  /// 当前匹配的预设 ID；窗口参数调整后清空，修改内容不影响它。
  preset: string | null;
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
  /// 与某个预设完全一致时带上该 ID，否则为 null 表示“自定义”。
  preset?: string | null;
}

/// 窗口即时设置不携带名称或来源，避免覆盖内容草稿与最新几何信息。
export type WindowValues = Omit<PinSettingsInput, 'name' | 'source' | 'preset'>;
export type WindowPatch = Partial<WindowValues>;

/// 原生事务前后快照，供撤销使用，避免依赖可能滞后的前端回传。
export interface WindowChange {
  before: PinConfig;
  after: PinConfig;
}

/// 页面内容独立提交，只有来源改动时才重新加载页面。
export interface PinContentInput {
  name: string;
  source?: SourceInput;
}

/// 预设编辑输入；省略 ID 创建副本，提供 ID 更新自定义项。
export interface PresetInput extends PresetFields {
  name: string;
}

/// 预设包含的外观与几何字段，与 Rust `PresetFields` 一致。
export interface PresetFields {
  mode: 'floating' | 'desktop';
  alwaysOnTop: boolean;
  decorations: boolean;
  rounded: boolean;
  positionLocked: boolean;
  sizeLocked: boolean;
  width: number;
  height: number;
  /// 目标显示器工作区内的归一化位置：0 贴左/上，1 贴右/下。
  fx: number;
  fy: number;
  /// 全屏项在应用时按目标显示器整屏尺寸换算，忽略 width/height。
  fullscreen: boolean;
}

/// 窗口预设；内置项 name 为 null，由语言资源提供显示名。
export interface WindowPreset extends PresetFields {
  id: string;
  name: string | null;
  builtin: boolean;
}

/// 无边框窗口的默认或固定拖动修饰键，与 Rust `PlatformDragModifier` 一致。
export type PlatformDragModifier = 'alt' | 'option';

/// 当前构建实际支持的窗口能力，与 Rust `PlatformCapabilities` 一致。
export interface PlatformCapabilities {
  desktopMode: boolean;
  rounded: boolean;
  positionLock: boolean;
  sizeLock: boolean;
  /// 是否允许配置手势修饰键；固定手势由 dragModifier 表达。
  windowGestures: boolean;
  /// null 表示没有原生修饰键手势，无边框窗口仍可通过托盘找回。
  dragModifier: PlatformDragModifier | null;
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
  /// 内置与自定义预设，由 Rust 统一定义，前端只负责渲染。
  presets: WindowPreset[];
  /// 启动时由 Rust 解析，前端不再自行猜测系统语言。
  resolvedLocale: AppLocale;
  /// 当前构建支持的窗口能力，所有窗口入口共用同一来源。
  capabilities: PlatformCapabilities;
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
