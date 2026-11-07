// 功能：把消息码映射为翻译键，并把任意 IPC 拒绝值收敛为可渲染的显示消息。
import { i18n } from './index';
import { isAppMessage } from '../types/messages';
import type { AppMessage, UiMessage } from '../types/messages';

/// 未知消息的统一兜底键，保证界面永远有可读说明而不是 `[object Object]`。
const UNKNOWN_KEY = 'native.errors.unknown';

/// 创建界面本地消息；键必须是完整的 `ui.*` 或 `native.*` 路径。
export function message(key: string, params?: Record<string, string | number>): UiMessage {
  return { key, params };
}

/// 把 Rust 的结构化消息转换为界面显示消息。转换只改名，不翻译，
/// 因此通知历史在切换语言后可以按当前语言重新渲染。
export function fromAppMessage(source: AppMessage): UiMessage {
  return { key: `native.${source.code}`, params: source.args, detail: source.detail };
}

/// 把 IPC 拒绝值收敛为显示消息：本应用的结构化消息直接转换，
/// 其余（包括 Tauri 内部的字符串拒绝和非预期对象）走统一兜底并保留原始文字。
export function toUiMessage(reason: unknown): UiMessage {
  if (isAppMessage(reason)) return fromAppMessage(reason);
  const detail =
    reason instanceof Error
      ? reason.message
      : typeof reason === 'string'
        ? reason
        : JSON.stringify(reason);
  return { key: UNKNOWN_KEY, detail };
}

/// 按当前语言渲染显示消息；缺键回退中文，仍缺失时给出兜底说明并附上定位线索。
export function text(entry: UiMessage | null | undefined): string {
  if (!entry) return '';
  const { t, te } = i18n.global;
  const params = entry.params ?? {};
  if (!te(entry.key, 'zh-CN')) {
    return String(
      t('ui.message.detailSuffix', { text: String(t(UNKNOWN_KEY)), detail: entry.key }),
    );
  }
  const base = String(t(entry.key, params));
  if (!entry.detail) return base;
  return String(t('ui.message.detailSuffix', { text: base, detail: entry.detail }));
}
