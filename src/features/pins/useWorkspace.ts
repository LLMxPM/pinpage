// 功能：协调管理界面的加载、原生操作、错误提示与事件订阅。
import { computed, onMounted, onUnmounted, ref } from 'vue';
import * as desktop from '../../services/desktop';
import { formatBytes } from '../../services/format';
import type { CreatePin, PinSettingsInput, SettingsInput, Snapshot } from '../../types';

/// 管理快照与操作状态，避免过期异步请求覆盖最新窗口列表。
export function useWorkspace() {
  const snapshot = ref<Snapshot>({
    settings: {
      schemaVersion: 1,
      restoreSession: true,
      managerShortcut: 'Ctrl+Alt+P',
      recoverShortcut: 'Ctrl+Alt+B',
      pins: [],
    },
    runtime: {},
    notices: [],
  });
  const busy = ref(false);
  const loading = ref(desktop.desktopAvailable);
  const error = ref('');
  /// 成功类提示单独一条通道，避免和红色错误混在一起。
  const toast = ref('');
  const settingsOpen = ref(false);
  const editingPinId = ref<string | null>(null);
  let revision = 0;
  let disposed = false;
  let unlisten: (() => void) | undefined;
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  /// 短暂展示导出结果，超时后自动收起。
  function notify(message: string) {
    toast.value = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast.value = '';
    }, 6000);
  }

  /// 保留最近一次请求的结果，原生窗口事件突发时不渲染旧快照。
  async function refresh() {
    if (!desktop.desktopAvailable || disposed) return;
    const current = ++revision;
    try {
      const next = await desktop.getSnapshot();
      if (!disposed && current === revision) snapshot.value = next;
    } catch (reason) {
      error.value = String(reason);
    } finally {
      loading.value = false;
    }
  }

  /// 统一呈现桌面操作错误，完成后刷新实际状态。
  async function run(operation: () => Promise<void>) {
    if (busy.value) return;
    if (!desktop.desktopAvailable) {
      error.value = '请通过桌面应用使用此功能：pnpm tauri dev';
      return;
    }
    busy.value = true;
    error.value = '';
    try {
      await operation();
    } catch (reason) {
      error.value = String(reason);
    } finally {
      busy.value = false;
      await refresh();
    }
  }

  /// 创建页面，来源验证和系统窗口创建均在原生层完成。
  function create(input: CreatePin) {
    return run(() => desktop.createPin(input));
  }

  /// 将单窗操作交给后端，避免前端乐观修改后与真实状态脱节。
  function action(id: string, operation: string) {
    return run(() => desktop.pinAction(id, operation));
  }

  /// 导出导入页的站点副本与页面数据，结果通过提示条反馈。
  async function exportPin(id: string) {
    if (busy.value) return;
    if (!desktop.desktopAvailable) {
      error.value = '请通过桌面应用使用此功能：pnpm tauri dev';
      return;
    }
    busy.value = true;
    error.value = '';
    try {
      const result = await desktop.exportPage(id);
      if (result) {
        notify(
          result.withData
            ? `已导出 ${result.files} 个文件，页面数据 ${formatBytes(result.dataBytes)}：${result.path}`
            : `已导出 ${result.files} 个文件，页面数据未包含在内：${result.path}`,
        );
      }
    } catch (reason) {
      error.value = String(reason);
    } finally {
      busy.value = false;
      await refresh();
    }
  }

  /// 导入内置示例并立即创建页面，示例与用户导入页走同一条路径。
  async function createExample() {
    return run(async () => {
      const preview = await desktop.importPage('example');
      if (!preview) return;
      await desktop.createPin({
        name: preview.label,
        source: { kind: 'import', ticket: preview.ticket, entry: preview.candidates[0] },
        alwaysOnTop: true,
        decorations: true,
      });
    });
  }

  /// 保存设置成功后关闭弹层；失败时保留用户的输入。
  function saveSettings(input: SettingsInput) {
    return run(async () => {
      await desktop.updateSettings(input);
      settingsOpen.value = false;
    });
  }

  /// 单页设置成功后关闭弹窗，失败时保留正在编辑的内容。
  function savePin(input: PinSettingsInput) {
    const id = editingPinId.value;
    if (!id) return;
    return run(async () => {
      await desktop.updatePin(id, input);
      editingPinId.value = null;
    });
  }

  onMounted(async () => {
    if (desktop.desktopAvailable) {
      try {
        unlisten = await desktop.onPinsChanged(() => {
          clearTimeout(refreshTimer);
          refreshTimer = setTimeout(() => {
            void refresh();
          }, 80);
        });
        if (disposed) unlisten();
      } catch (reason) {
        error.value = String(reason);
      }
      await refresh();
    }
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
    loading,
    error,
    toast,
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
    saveSettings,
    quit: () => run(desktop.quitApp),
    pins: computed(() => snapshot.value.settings.pins),
    openCount: computed(
      () => Object.values(snapshot.value.runtime).filter((pin) => pin.open).length,
    ),
    topCount: computed(
      () =>
        snapshot.value.settings.pins.filter(
          (pin) =>
            snapshot.value.runtime[pin.id]?.open && pin.mode === 'floating' && pin.alwaysOnTop,
        ).length,
    ),
  };
}
