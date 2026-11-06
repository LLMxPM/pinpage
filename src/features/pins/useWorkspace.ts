// 功能：协调管理界面的加载、原生操作、错误提示与事件订阅。
import { computed, onMounted, onUnmounted, ref } from 'vue';
import * as desktop from '../../services/desktop';
import type { CreatePin, SettingsInput, Snapshot } from '../../types';

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
  const settingsOpen = ref(false);
  let revision = 0;
  let disposed = false;
  let unlisten: (() => void) | undefined;
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;

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

  /// 保存设置成功后关闭弹层；失败时保留用户的输入。
  function saveSettings(input: SettingsInput) {
    return run(async () => {
      await desktop.updateSettings(input);
      settingsOpen.value = false;
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
  });

  return {
    snapshot,
    busy,
    loading,
    error,
    settingsOpen,
    create,
    action,
    saveSettings,
    quit: () => run(desktop.quitApp),
    pins: computed(() => snapshot.value.settings.pins),
    openCount: computed(
      () => Object.values(snapshot.value.runtime).filter((pin) => pin.open).length,
    ),
    topCount: computed(() => snapshot.value.settings.pins.filter((pin) => pin.alwaysOnTop).length),
  };
}
