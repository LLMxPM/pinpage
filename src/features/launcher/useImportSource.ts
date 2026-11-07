// 功能：封装原生导入选择器的预览状态、入口候选与错误收敛，供新建页面与更新来源共用。
import { computed, ref } from 'vue';
import { message, toUiMessage } from '../../i18n/messages';
import { desktopAvailable, importPage } from '../../services/desktop';
import type { ExampleId, ImportPreview, UiMessage } from '../../types';

/// 选择器结果回调：`onError` 必须提供，`onPicked` 用于新建时按来源推导默认名称。
export function useImportSource(options: {
  onError: (entry: UiMessage) => void;
  onPicked?: (preview: ImportPreview) => void;
}) {
  const preview = ref<ImportPreview | null>(null);
  const entry = ref('');
  const choosing = ref(false);
  const entryOptions = computed(
    () => preview.value?.candidates.map((value) => ({ value, label: value })) ?? [],
  );

  /// 丢弃上次预览；票据随选择器重新打开而失效，避免提交过期来源。
  function reset() {
    preview.value = null;
    entry.value = '';
  }

  /// 打开原生选择器或登记内置来源；用户取消时保留上次结果，不产生错误。
  async function choose(mode: ImportPreview['mode'], example?: ExampleId) {
    if (!desktopAvailable) {
      options.onError(message('ui.form.needDesktop'));
      return;
    }
    choosing.value = true;
    try {
      const selected = await importPage(mode, example);
      if (!selected) return;
      preview.value = selected;
      entry.value = selected.candidates[0] ?? '';
      options.onPicked?.(selected);
    } catch (reason) {
      options.onError(toUiMessage(reason));
    } finally {
      choosing.value = false;
    }
  }

  return { preview, entry, choosing, entryOptions, choose, reset };
}
