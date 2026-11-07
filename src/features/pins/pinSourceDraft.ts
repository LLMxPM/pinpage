// 功能：判定页面来源草稿是否需要提交，只在用户确实改动时产出来源输入。
//
// 保存外观设置时不应重新加载页面，也不应重复复制导入副本，
// 因此“提交了 source 字段”必须等价于“用户主动改动过来源”。
import type { ImportPreview, PinConfig, SourceInput } from '../../types';

/// 来源草稿的可编辑部分：网址地址，以及重新选择的导入预览与确认入口。
export interface SourceDraftState {
  address: string;
  preview: ImportPreview | null;
  entry: string;
}

/// 表单初始值：网址页填入当前地址，导入页不预置任何改动。
export function initialSourceDraft(pin: PinConfig): SourceDraftState {
  return {
    address: pin.source.kind === 'url' ? pin.source.url : '',
    preview: null,
    entry: '',
  };
}

/// 把草稿折算为待提交的来源输入；返回 null 表示沿用现有来源。
///
/// 网址页只比较去空白后的地址文本，是否合法由 Rust 的 `normalize_url` 判定；
/// 导入页必须同时具备新的预览结果和入口，缺任一项都视为未改动。
export function resolveSourceInput(pin: PinConfig, draft: SourceDraftState): SourceInput | null {
  if (pin.source.kind === 'url') {
    const address = draft.address.trim();
    return address && address !== pin.source.url ? { kind: 'url', address } : null;
  }
  const preview = draft.preview;
  if (!preview || !draft.entry) return null;
  return { kind: 'import', ticket: preview.ticket, entry: draft.entry };
}
