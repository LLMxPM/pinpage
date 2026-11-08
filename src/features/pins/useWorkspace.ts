// 功能：协调管理界面的语言应用、原生操作、消息提示与事件订阅。
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { applyLocale } from '../../i18n';
import { fromAppMessage, message, toUiMessage } from '../../i18n/messages';
import * as desktop from '../../services/desktop';
import { formatBytes } from '../../services/format';
import { createOperationQueue } from './operationQueue';
import { useWindowActions } from './useWindowActions';
import type {
  CreatePin,
  ExampleId,
  PinContentInput,
  SettingsInput,
  Snapshot,
  UiMessage,
} from '../../types';

/// 空快照与 Rust 默认配置保持一致，浏览器预览按中文呈现。
function emptySnapshot(): Snapshot {
  return {
    settings: {
      schemaVersion: 1,
      locale: 'zh-CN',
      gestureModifier: 'alt',
      restoreSession: true,
      pins: [],
    },
    runtime: {},
    notices: [],
    presets: [],
    resolvedLocale: 'zh-CN',
  };
}

/// 管理快照与操作状态，避免过期异步请求覆盖最新窗口列表。
export function useWorkspace() {
  const snapshot = ref<Snapshot>(emptySnapshot());
  const pending = ref<string | null>(null);
  const busy = computed(() => pending.value !== null);
  const loading = ref(desktop.desktopAvailable);
  /// 首个快照应用有效语言之前不渲染业务界面，减少英文用户看到中文闪烁。
  const ready = ref(!desktop.desktopAvailable);
  const error = ref<UiMessage | null>(null);
  /// 成功类提示单独一条通道，避免和红色错误混在一起。
  const toast = ref<UiMessage | null>(null);
  const settingsOpen = ref(false);
  const editingPinId = ref<string | null>(null);
  let revision = 0;
  let disposed = false;
  let unlisten: (() => void) | undefined;
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  /// 短暂展示操作结果，超时后自动收起。提示保存消息键和参数，不提前固化文字。
  function notify(entry: UiMessage) {
    toast.value = entry;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(dismissToast, 6000);
  }

  /// 收起提示条；窗口字段撤销由页面设置底栏提供，不随提示条保存。
  function dismissToast() {
    toast.value = null;
  }

  /// 保留最近一次请求的结果，原生窗口事件突发时不渲染旧快照。
  async function refresh() {
    if (!desktop.desktopAvailable || disposed) return;
    const current = ++revision;
    try {
      const next = await desktop.getSnapshot();
      if (disposed || current !== revision) return;
      snapshot.value = next;
      applyLocale(next.resolvedLocale);
    } catch (reason) {
      error.value = toUiMessage(reason);
    } finally {
      loading.value = false;
    }
  }

  const enqueue = createOperationQueue();

  /// 统一排队并呈现桌面操作错误，完成后刷新实际状态。
  function run(operation: () => Promise<void>, key = 'app'): Promise<boolean> {
    return enqueue(async () => {
      if (!desktop.desktopAvailable) {
        error.value = message('ui.desktop.desktopRequired');
        return false;
      }
      pending.value = key;
      error.value = null;
      try {
        await operation();
        return true;
      } catch (reason) {
        error.value = toUiMessage(reason);
        return false;
      } finally {
        await refresh();
        pending.value = null;
      }
    });
  }

  /// 创建页面，来源验证和系统窗口创建均在原生层完成。
  function create(input: CreatePin) {
    return run(() => desktop.createPin(input), 'create');
  }

  /// 将单窗操作交给后端，避免前端乐观修改后与真实状态脱节。
  function action(id: string, operation: string) {
    return run(() => desktop.pinAction(id, operation), `${id}:${operation}`);
  }

  const windowActions = useWindowActions(snapshot, run, notify);

  /// 导出导入页的站点副本与页面数据，结果通过提示条反馈。
  async function exportPin(id: string) {
    return run(async () => {
      const result = await desktop.exportPage(id);
      if (result) {
        notify(
          result.withData
            ? message('ui.toast.exportedWithData', {
                count: result.files,
                size: formatBytes(result.dataBytes),
                path: result.path,
              })
            : message('ui.toast.exportedWithoutData', {
                count: result.files,
                path: result.path,
              }),
        );
      }
    }, `${id}:export`);
  }

  /// 导入内置示例并立即创建页面，示例与用户导入页走同一条路径。
  async function createExample(example: ExampleId = 'timer') {
    return run(async () => {
      const preview = await desktop.importPage('example', example);
      if (!preview) return;
      await desktop.createPin({
        name: preview.label,
        source: { kind: 'import', ticket: preview.ticket, entry: preview.candidates[0] },
        alwaysOnTop: true,
        decorations: false,
      });
    }, `example:${example}`);
  }

  /// 保存设置成功后关闭弹层；失败时保留用户的输入和当前语言。
  function saveSettings(input: SettingsInput) {
    return run(async () => {
      await desktop.updateSettings(input);
      settingsOpen.value = false;
    });
  }

  /// 内容保存成功后保持弹窗打开，失败时保留正在编辑的草稿。
  async function savePin(input: PinContentInput) {
    const id = editingPinId.value;
    if (!id) return;
    return run(async () => {
      await desktop.updateContent(id, input);
    });
  }

  onMounted(async () => {
    if (!desktop.desktopAvailable) return;
    try {
      unlisten = await desktop.onPinsChanged(() => {
        clearTimeout(refreshTimer);
        refreshTimer = setTimeout(() => {
          void refresh();
        }, 80);
      });
      if (disposed) unlisten();
    } catch (reason) {
      error.value = toUiMessage(reason);
    }
    await refresh();
    if (!disposed) ready.value = true;
  });
  onUnmounted(() => {
    disposed = true;
    unlisten?.();
    clearTimeout(refreshTimer);
    clearTimeout(toastTimer);
  });

  return {
    snapshot,
    busy,
    pending,
    loading,
    ready,
    error,
    toast,
    notices: computed<UiMessage[]>(() => snapshot.value.notices.map(fromAppMessage)),
    settingsOpen,
    editingPinId,
    editingPin: computed(() =>
      snapshot.value.settings.pins.find((pin) => pin.id === editingPinId.value),
    ),
    savePin,
    create,
    createExample,
    exportPin,
    action,
    ...windowActions,
    dismissToast,
    saveSettings,
    quit: () => run(desktop.quitApp),
    pins: computed(() => snapshot.value.settings.pins),
    presets: computed(() => snapshot.value.presets),
    openCount: computed(
      () => Object.values(snapshot.value.runtime).filter((pin) => pin.open).length,
    ),
  };
}
