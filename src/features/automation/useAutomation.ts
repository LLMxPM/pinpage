// 功能：管理智能体面板的读取、即时控制开关与技能操作状态。
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  automationSnapshot,
  automationControl,
  automationSkill,
  type AutomationStatus,
} from '../../services/automation';
import { desktopAvailable } from '../../services/desktop';
import { toUiMessage, text } from '../../i18n/messages';

/// 面板每次打开读取最新状态；原生操作结束前禁止重复提交或关闭面板。
export function useAutomation() {
  const { t } = useI18n();
  const status = ref<AutomationStatus>();
  const busy = ref(desktopAvailable);
  const error = ref('');
  const result = ref<{ message: string; path?: string; version?: string }>();
  const disabled = computed(() => busy.value || !desktopAvailable || !status.value);

  /// 保留结构化原生错误中的详情，方便定位目录冲突或读写失败。
  function showError(value: unknown) {
    error.value = text(toUiMessage(value));
  }

  /// 默认目录状态与自定义目录操作结果分开，选择其他目录不会改变默认安装目标。
  async function refresh() {
    status.value = await automationSnapshot();
  }

  onMounted(async () => {
    if (!desktopAvailable) return;
    try {
      await refresh();
    } catch (value) {
      showError(value);
    } finally {
      busy.value = false;
    }
  });

  /// 开关立即保存；失败后恢复上一次已确认的状态，不影响技能操作的可用性。
  async function toggle(event: Event) {
    const input = event.target as HTMLInputElement;
    if (disabled.value) return;
    busy.value = true;
    error.value = '';
    result.value = undefined;
    try {
      await automationControl(input.checked);
      if (status.value) status.value.localControl = input.checked;
      await refresh();
    } catch (value) {
      showError(value);
      input.checked = status.value?.localControl ?? false;
    } finally {
      busy.value = false;
    }
  }

  /// 安装、更新、查看与导出共用原生选择器；取消选择时不显示成功提示。
  async function run(action: string, custom = false) {
    if (disabled.value) return;
    busy.value = true;
    error.value = '';
    result.value = undefined;
    try {
      const value = await automationSkill(action, custom);
      if (value) {
        result.value = {
          message: t(
            action === 'status'
              ? 'ui.automation.inspected'
              : value.changed === false
                ? 'ui.automation.unchanged'
                : 'ui.automation.completed',
          ),
          path: String(value.target ?? value.path ?? ''),
          version: String(
            value.sourceVersion ??
              value.installedVersion ??
              value.skillVersion ??
              (action === 'status' ? t('ui.automation.notInstalled') : ''),
          ),
        };
        await refresh();
      }
    } catch (value) {
      showError(value);
    } finally {
      busy.value = false;
    }
  }

  return { status, busy, disabled, error, result, toggle, run };
}
