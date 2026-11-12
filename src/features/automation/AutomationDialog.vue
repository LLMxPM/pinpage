<!-- 功能：提供独立的智能体与 CLI 面板，分组展示控制权限、技能版本、安装与导出操作。 -->
<script setup lang="ts">
import { nextTick, ref, useId, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import AppDialog from '../../components/AppDialog.vue';
import AppIcon from '../../components/AppIcon.vue';
import { desktopAvailable } from '../../services/desktop';
import { useAutomation } from './useAutomation';

const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
const { status, busy, disabled, error, result, toggle, run } = useAutomation();
const feedback = ref<HTMLElement>();
const controlHintId = useId();

/// 操作反馈出现时滚动到提示，避免长面板把失败原因或导出位置藏在视区外。
watch([error, result], async () => {
  await nextTick();
  feedback.value?.scrollIntoView({ block: 'nearest' });
});
</script>

<template>
  <AppDialog :title="t('ui.automation.title')" :busy="busy" large @close="emit('close')">
    <div class="automation-panel">
      <p v-if="!desktopAvailable" class="notice warning-notice" role="status">
        <AppIcon name="info" :size="17" />{{ t('ui.automation.desktopOnly') }}
      </p>
      <label class="setting-toggle automation-control">
        <span>
          <strong>{{ t('ui.automation.control') }}</strong>
          <small :id="controlHintId">{{ t('ui.automation.description') }}</small>
        </span>
        <input
          class="switch"
          type="checkbox"
          role="switch"
          :aria-label="t('ui.automation.control')"
          :aria-describedby="controlHintId"
          :checked="status?.localControl ?? false"
          :disabled="disabled"
          @change="toggle"
        />
      </label>

      <section class="automation-section" :aria-label="t('ui.automation.skillTitle')">
        <h3><AppIcon name="note" :size="18" />{{ t('ui.automation.skillTitle') }}</h3>
        <p class="automation-help">{{ t('ui.automation.skillHint') }}</p>
        <div class="automation-versions">
          <span
            >{{ t('ui.automation.builtinVersion') }}
            <b>{{ status?.builtinVersion ?? '—' }}</b></span
          >
          <span :class="{ installed: status?.installedVersion }">
            {{ t('ui.automation.installedVersion') }}
            <b>{{ status ? (status.installedVersion ?? t('ui.automation.notInstalled')) : '—' }}</b>
          </span>
        </div>
        <dl class="automation-paths">
          <dt>{{ t('ui.automation.skillPath') }}</dt>
          <dd>
            <code>{{ status?.target ?? '—' }}</code>
          </dd>
          <dt>{{ t('ui.automation.cliPath') }}</dt>
          <dd>
            <code>{{ status?.cliPath ?? '—' }}</code>
          </dd>
        </dl>
        <p v-if="status?.error" class="automation-help" role="alert">{{ status.error }}</p>
        <div class="automation-actions">
          <button
            type="button"
            class="primary"
            :disabled="disabled"
            @click="run(status?.installedVersion ? 'update' : 'install')"
          >
            <AppIcon :name="status?.installedVersion ? 'refresh' : 'download'" :size="16" />
            {{ t(status?.installedVersion ? 'ui.automation.update' : 'ui.automation.install') }}
          </button>
        </div>
        <p class="automation-help">{{ t('ui.automation.reloadHint') }}</p>
      </section>

      <section class="automation-section" :aria-label="t('ui.automation.customTitle')">
        <h3><AppIcon name="folder" :size="18" />{{ t('ui.automation.customTitle') }}</h3>
        <p class="automation-help">{{ t('ui.automation.customHint') }}</p>
        <div class="automation-actions">
          <button
            type="button"
            class="secondary"
            :disabled="disabled"
            @click="run('install', true)"
          >
            {{ t('ui.automation.customInstall') }}
          </button>
          <button type="button" class="secondary" :disabled="disabled" @click="run('update', true)">
            {{ t('ui.automation.customUpdate') }}
          </button>
          <button type="button" class="secondary" :disabled="disabled" @click="run('status', true)">
            {{ t('ui.automation.customStatus') }}
          </button>
        </div>
      </section>

      <section class="automation-section" :aria-label="t('ui.automation.exportTitle')">
        <h3><AppIcon name="download" :size="18" />{{ t('ui.automation.exportTitle') }}</h3>
        <p class="automation-help">{{ t('ui.automation.exportHint') }}</p>
        <div class="automation-actions">
          <button type="button" class="secondary" :disabled="disabled" @click="run('exportZip')">
            {{ t('ui.automation.exportZip') }}
          </button>
          <button
            type="button"
            class="secondary"
            :disabled="disabled"
            @click="run('exportDirectory')"
          >
            {{ t('ui.automation.exportDirectory') }}
          </button>
        </div>
      </section>

      <div v-if="error || result" ref="feedback" class="automation-feedback">
        <div v-if="error" role="alert" class="notice error-notice">
          <AppIcon name="info" :size="18" /><span>{{ error }}</span>
        </div>
        <div v-if="result" class="notice success-notice automation-result" role="status">
          <AppIcon name="check" :size="18" />
          <div>
            <strong>{{ result.message }}</strong>
            <p v-if="result.version">
              {{ t('ui.automation.resultVersion', { version: result.version }) }}
            </p>
            <code v-if="result.path">{{ result.path }}</code>
          </div>
        </div>
      </div>
    </div>
    <template #footer>
      <span v-if="busy" class="automation-working" role="status">
        <AppIcon class="spinning" name="refresh" :size="15" />{{ t('ui.common.working') }}
      </span>
      <button type="button" class="primary" :disabled="busy" @click="emit('close')">
        {{ t('ui.common.close') }}
      </button>
    </template>
  </AppDialog>
</template>

<style scoped>
.automation-panel {
  display: grid;
  gap: 24px;
}
.automation-panel .notice {
  margin: 0;
}
.automation-feedback {
  display: grid;
  gap: 12px;
}
.automation-control {
  margin: 0;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: #f8f9fb;
}
.automation-section + .automation-section {
  padding-top: 22px;
  border-top: 1px solid var(--line);
}
.automation-section h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
}
.automation-section h3 svg {
  color: var(--muted);
}
.automation-help {
  margin-top: 8px;
  color: var(--muted);
  font-size: 12px;
  line-height: 1.7;
}
.automation-versions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}
.automation-versions span {
  padding: 5px 9px;
  border-radius: 6px;
  background: #f0f2f5;
  color: #5d6979;
  font-size: 12px;
}
.automation-versions b {
  margin-left: 5px;
  font-weight: 600;
}
.automation-versions .installed {
  background: #edf6f0;
  color: #387458;
}
.automation-paths {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 10px 16px;
  margin: 14px 0 0;
  padding: 12px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  font-size: 12px;
  line-height: 1.7;
}
.automation-paths dt {
  color: var(--muted);
}
.automation-paths dd {
  min-width: 0;
  margin: 0;
}
code {
  font-family: Consolas, 'Microsoft YaHei', monospace;
  font-size: 12px;
  overflow-wrap: anywhere;
  user-select: text;
}
.automation-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}
.automation-result {
  align-items: flex-start;
}
.automation-result > div {
  min-width: 0;
}
.automation-result p,
.automation-result code {
  display: block;
  margin-top: 5px;
  line-height: 1.6;
}
.automation-working {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  margin-right: auto;
  color: var(--muted);
  font-size: 12px;
}
@media (max-width: 540px) {
  .automation-paths {
    grid-template-columns: minmax(0, 1fr);
    gap: 4px;
  }
  .automation-paths dt:not(:first-child) {
    margin-top: 8px;
  }
  .automation-actions button {
    white-space: normal;
    text-align: left;
  }
}
</style>
