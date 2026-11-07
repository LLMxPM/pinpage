// 功能：定义跨 Rust 与前端传递的结构化消息契约，以及界面可直接渲染的显示消息。
/// Rust 侧的稳定消息契约：`code` 指向 native 资源键，`args` 为受控文本参数，
/// `detail` 只承载系统或第三方诊断原文，三者都不含已翻译句子，因此切换语言可重新渲染。
export interface AppMessage {
  code: string;
  args?: Record<string, string | number>;
  detail?: string;
}

/// 界面统一使用的显示消息：`key` 是完整翻译键，参数一律按文本插值。
export interface UiMessage {
  key: string;
  params?: Record<string, string | number>;
  detail?: string;
}

/// 判别是否为 Rust 返回的结构化消息，避免把普通字符串误当对象渲染。
export function isAppMessage(value: unknown): value is AppMessage {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as AppMessage).code === 'string' &&
    (value as AppMessage).code.length > 0
  );
}
