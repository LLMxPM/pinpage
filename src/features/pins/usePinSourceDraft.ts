// 功能：为页面设置提供来源草稿，网址页记录地址改动，导入页记录重新选择的来源与入口。
import { computed, ref, watch } from 'vue';
import { useImportSource } from '../launcher/useImportSource';
import { initialSourceDraft, resolveSourceInput } from './pinSourceDraft';
import type { PinConfig, UiMessage } from '../../types';

/// 来源草稿与共用选择器的组合；`onError` 由界面接入统一提示通道。
export function usePinSourceDraft(
  getPin: () => PinConfig,
  options: { onError: (entry: UiMessage) => void },
) {
  const address = ref('');
  const picker = useImportSource({ onError: options.onError });

  /// 切换到另一个页面时重置草稿；同一页面的外部改动不覆盖正在编辑的地址。
  function restart() {
    address.value = initialSourceDraft(getPin()).address;
    picker.reset();
  }
  watch(() => getPin().id, restart, { immediate: true });

  /// 折算待提交来源：未改动时为 null，保存外观设置因此不会重新加载页面。
  const input = computed(() =>
    resolveSourceInput(getPin(), {
      address: address.value,
      preview: picker.preview.value,
      entry: picker.entry.value,
    }),
  );

  return {
    address,
    input,
    dirty: computed(() => {
      const source = getPin().source;
      return source.kind === 'url' ? address.value.trim() !== source.url : input.value !== null;
    }),
    preview: picker.preview,
    entry: picker.entry,
    entryOptions: picker.entryOptions,
    choosing: picker.choosing,
    choose: picker.choose,
    reset: restart,
  };
}
