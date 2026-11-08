// 功能：共享升级快照与订阅，打开关闭设置不会丢失进度或重复启动任务。
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { desktopAvailable } from '../../services/desktop';
import * as desktop from '../../services/updates';
import { toUiMessage } from '../../i18n/messages';
import type { UiMessage } from '../../types';
import type { UpdateAction, UpdateSnapshot } from '../../types/updates';
import { version } from '../../../package.json';

const snapshot = ref<UpdateSnapshot>({
  currentVersion: version,
  supported: false,
  status: 'idle',
  latestVersion: null,
  notes: null,
  downloaded: 0,
  total: null,
  error: null,
  preferences: { autoCheck: true, lastCheck: null, ignoredVersion: null },
});
let notifications = 0;

/// 拉取期间若已有较新事件到达，保留事件快照，防止进度和状态倒退。
async function refresh(canApply: () => boolean = () => true) {
  const before = notifications;
  const next = await desktop.getUpdateSnapshot();
  if (canApply() && before === notifications) snapshot.value = next;
}

/// 应用根节点持有唯一订阅；状态变化只影响升级区域和可用更新提示。
export function useUpdateSubscription() {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  onMounted(async () => {
    if (!desktopAvailable) return;
    try {
      unlisten = await desktop.onUpdatesChanged((value) => {
        notifications += 1;
        snapshot.value = value;
      });
      if (disposed) {
        unlisten();
        return;
      }
      await refresh(() => !disposed);
    } catch {
      /* 原生连接错误由工作台快照通道呈现，避免重复提示。 */
    }
  });
  onUnmounted(() => {
    disposed = true;
    unlisten?.();
  });
  return {
    updateAvailable: computed(() => ['available', 'ready'].includes(snapshot.value.status)),
  };
}

/// 设置区域负责动作错误与安装前确认，后台下载由 Rust 独立维持。
export function useUpdates() {
  const pending = ref(false);
  const error = ref<UiMessage | null>(null);
  const confirmInstall = ref(false);

  /// 动作完成后拉取实际状态；后端拒绝的原因保留在升级区域。
  async function run(action: UpdateAction, enabled?: boolean) {
    if (pending.value || !desktopAvailable) return;
    pending.value = true;
    error.value = null;
    try {
      await desktop.updateAction(action, enabled);
      await refresh();
      if (action === 'install') confirmInstall.value = false;
    } catch (reason) {
      error.value = toUiMessage(reason);
    } finally {
      pending.value = false;
    }
  }

  return { snapshot, pending, error, confirmInstall, run };
}
