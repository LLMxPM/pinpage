<!-- 功能：展示版本、GitHub 更新说明与下载进度，安装前明确确认页面关闭影响。 -->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { fromAppMessage, text } from '../../i18n/messages';
import { desktopAvailable } from '../../services/desktop';
import { formatBytes } from '../../services/format';
import { useUpdates } from './useUpdates';

const props = defineProps<{ busy: boolean }>();
const { t, locale } = useI18n();
const { snapshot, pending, error, confirmInstall, run } = useUpdates();
const working = computed(
  () =>
    props.busy ||
    pending.value ||
    ['checking', 'downloading', 'verifying', 'installing'].includes(snapshot.value.status),
);
const canDownload = computed(
  () =>
    ['available', 'ignored', 'error'].includes(snapshot.value.status) &&
    !!snapshot.value.latestVersion,
);
const progress = computed(() =>
  snapshot.value.total
    ? Math.min(100, (snapshot.value.downloaded / snapshot.value.total) * 100)
    : undefined,
);
const lastCheck = computed(() =>
  snapshot.value.preferences.lastCheck
    ? new Date(snapshot.value.preferences.lastCheck * 1000).toLocaleString(locale.value)
    : t('ui.updates.neverChecked'),
);
const visibleError = computed(
  () => error.value ?? (snapshot.value.error ? fromAppMessage(snapshot.value.error) : null),
);

/// 复选框直接持久化升级偏好，不依赖设置表单的保存按钮。
function changeAutoCheck(event: Event) {
  void run('autoCheck', (event.target as HTMLInputElement).checked);
}
</script>

<template>
  <div class="update-settings">
    <label class="setting-toggle">
      <span
        >{{ t('ui.updates.autoCheck')
        }}<small class="instant-tag">{{ t('ui.updates.autoCheckHint') }}</small></span
      >
      <input
        type="checkbox"
        role="switch"
        class="switch"
        :checked="snapshot.preferences.autoCheck"
        :disabled="working || !snapshot.supported"
        @change="changeAutoCheck"
      />
    </label>
    <p v-if="!snapshot.supported" class="update-muted">{{ t('ui.updates.unsupported') }}</p>
    <template v-else>
      <p class="update-muted">{{ t('ui.updates.lastCheck', { time: lastCheck }) }}</p>
      <p role="status">
        {{ t(`ui.updates.status.${snapshot.status}`)
        }}<span v-if="snapshot.latestVersion"> · v{{ snapshot.latestVersion }}</span>
      </p>
      <div
        v-if="['downloading', 'verifying', 'ready'].includes(snapshot.status)"
        class="update-progress"
      >
        <progress :value="progress" max="100" :aria-label="t('ui.updates.downloadProgress')" />
        <span
          >{{ formatBytes(snapshot.downloaded)
          }}<template v-if="snapshot.total"> / {{ formatBytes(snapshot.total) }}</template></span
        >
      </div>
      <details v-if="snapshot.notes">
        <summary>{{ t('ui.updates.releaseNotes') }}</summary>
        <pre>{{ snapshot.notes }}</pre>
      </details>
    </template>
    <p v-if="visibleError" class="notice error-notice" role="alert">{{ text(visibleError) }}</p>
    <div class="update-actions">
      <button
        type="button"
        class="secondary"
        :disabled="working || !snapshot.supported || snapshot.status === 'ready'"
        @click="run('check')"
      >
        {{ t('ui.updates.check') }}
      </button>
      <button
        v-if="canDownload"
        type="button"
        class="primary"
        :disabled="working"
        @click="run('download')"
      >
        {{ t('ui.updates.download') }}
      </button>
      <button
        v-if="['available', 'ready'].includes(snapshot.status)"
        type="button"
        class="secondary"
        :disabled="working"
        @click="run('ignore')"
      >
        {{ t('ui.updates.ignore') }}
      </button>
      <button
        v-if="snapshot.status === 'ready' && !confirmInstall"
        type="button"
        class="primary"
        :disabled="working"
        @click="confirmInstall = true"
      >
        {{ t('ui.updates.install') }}
      </button>
      <button
        type="button"
        class="secondary"
        :disabled="!desktopAvailable || pending"
        @click="run('openRelease')"
      >
        {{ t('ui.updates.openRelease') }}
      </button>
    </div>
    <div
      v-if="confirmInstall && snapshot.status === 'ready'"
      class="update-confirm"
      role="group"
      :aria-label="t('ui.updates.install')"
    >
      <p>{{ t('ui.updates.installWarning') }}</p>
      <div class="update-actions">
        <button type="button" class="secondary" :disabled="working" @click="confirmInstall = false">
          {{ t('ui.common.cancel') }}
        </button>
        <button type="button" class="primary" :disabled="working" @click="run('install')">
          {{ t('ui.updates.confirmInstall') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.update-settings {
  display: grid;
  gap: 12px;
}
.update-settings .setting-toggle {
  margin-bottom: 0;
}
.update-settings p {
  margin: 0;
  font-size: 13px;
  line-height: 1.7;
  overflow-wrap: anywhere;
}
.update-muted {
  color: var(--muted);
}
.update-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.update-progress {
  display: grid;
  gap: 6px;
  font-size: 12px;
  color: var(--muted);
}
.update-progress progress {
  width: 100%;
  accent-color: var(--accent);
}
.update-settings details {
  font-size: 13px;
}
.update-settings summary {
  cursor: pointer;
}
.update-settings pre {
  max-height: 180px;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  font: inherit;
  line-height: 1.7;
}
.update-confirm {
  display: grid;
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--accent-soft);
}
</style>
