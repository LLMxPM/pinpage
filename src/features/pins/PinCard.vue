<!-- 功能：呈现页面来源、运行状态和明确的主操作，将低频操作收纳到更多菜单。 -->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import AppIcon from '../../components/AppIcon.vue';
import PinMenu from './PinMenu.vue';
import { fromAppMessage, text } from '../../i18n/messages';
import type { PinConfig, PinRuntime } from '../../types';

const props = defineProps<{
  pin: PinConfig;
  runtime?: PinRuntime;
  busy: boolean;
  working: boolean;
}>();
const emit = defineEmits<{
  action: [id: string, operation: string];
  settings: [id: string];
  export: [id: string];
}>();
const { t } = useI18n();
const sourceText = computed(() =>
  props.pin.source.kind === 'url'
    ? props.pin.source.url
    : t('ui.card.importedSource', { entry: props.pin.source.entry }),
);
const runtimeError = computed(() =>
  props.runtime?.error ? text(fromAppMessage(props.runtime.error)) : '',
);
const status = computed(() => {
  if (props.runtime?.error) return { tone: 'warning', key: 'ui.card.statusAttention' };
  if (!props.runtime?.open)
    return {
      tone: 'muted',
      key: props.pin.enabled ? 'ui.card.statusNotOpen' : 'ui.card.statusClosed',
    };
  if (props.runtime.loading) return { tone: 'loading', key: 'ui.card.statusLoading' };
  return props.runtime.visible
    ? { tone: 'running', key: 'ui.card.statusRunning' }
    : { tone: 'muted', key: 'ui.card.statusHidden' };
});
const primary = computed(() => {
  if (!props.runtime?.open) return { operation: 'open', icon: 'power', key: 'ui.card.open' };
  if (props.runtime.error) return { operation: 'reload', icon: 'refresh', key: 'ui.card.retry' };
  return {
    operation: 'focus',
    icon: 'eye',
    key: props.runtime.visible && props.pin.mode === 'floating' ? 'ui.card.locate' : 'ui.card.show',
  };
});
</script>

<template>
  <article class="pin-card panel" :aria-busy="working">
    <div class="pin-card-header">
      <div class="source-icon" :class="pin.source.kind">
        <AppIcon :name="pin.source.kind === 'url' ? 'link' : 'folder'" :size="21" />
      </div>
      <span class="status-pill" :class="status.tone"><i />{{ t(status.key) }}</span
      ><PinMenu
        :pin="pin"
        :runtime="runtime"
        :busy="busy"
        @action="emit('action', pin.id, $event)"
        @export="emit('export', pin.id)"
      />
    </div>
    <h2 :title="pin.name">{{ pin.name }}</h2>
    <p class="source-address" :title="sourceText">{{ sourceText }}</p>
    <div class="pin-meta">
      <span
        ><AppIcon :name="pin.mode === 'desktop' ? 'window' : 'expand'" :size="13" />{{
          pin.mode === 'desktop' ? t('ui.card.metaDesktopMode') : t('ui.card.metaFloating')
        }}</span
      ><span>{{ Math.round(pin.width) }} × {{ Math.round(pin.height) }}</span
      ><span v-if="pin.mode === 'floating' && pin.alwaysOnTop" class="top-label"
        ><AppIcon name="pin" :size="12" />{{ t('ui.card.metaPinned') }}</span
      >
    </div>
    <p v-if="runtimeError" class="pin-error" role="status">{{ runtimeError }}</p>
    <div class="pin-actions">
      <button class="secondary" :disabled="busy" @click="emit('settings', pin.id)">
        <AppIcon name="settings" :size="16" />{{ t('ui.card.settings') }}</button
      ><button class="primary" :disabled="busy" @click="emit('action', pin.id, primary.operation)">
        <AppIcon
          :name="working ? 'refresh' : primary.icon"
          :size="16"
          :class="{ spinning: working }"
        />{{ working ? t('ui.common.working') : t(primary.key) }}
      </button>
    </div>
  </article>
</template>
