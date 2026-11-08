<!-- 功能：紧凑呈现页面来源与运行状态，分别提供开启、显示隐藏与带到最前操作，低频操作收纳到更多菜单。 -->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import AppIcon from '../../components/AppIcon.vue';
import PinMenu from './PinMenu.vue';
import { canBringToFront, primaryAction } from './pinActions';
import { presetLabel } from './presets';
import { fromAppMessage, text } from '../../i18n/messages';
import type { PinConfig, PinRuntime, WindowPreset } from '../../types';

const props = defineProps<{
  pin: PinConfig;
  presets: WindowPreset[];
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
const primary = computed(() => primaryAction(props.pin, props.runtime));
const bringFront = computed(() => canBringToFront(props.pin, props.runtime));
const hideable = computed(() => !!props.runtime?.open && !!props.runtime.visible);
const closable = computed(() => !!props.runtime?.open);
/// 只在预设仍存在时展示来源标记，删除后的悬挂引用不会留下空标签。
const activePreset = computed(() => {
  const found = props.presets.find((preset) => preset.id === props.pin.preset);
  return found ? presetLabel(found) : '';
});
</script>

<template>
  <article class="pin-card panel" :aria-busy="working">
    <div class="pin-card-header">
      <div class="source-icon" :class="pin.source.kind">
        <AppIcon :name="pin.source.kind === 'url' ? 'link' : 'folder'" :size="18" />
      </div>
      <div class="pin-title-block">
        <h2 :title="pin.name">{{ pin.name }}</h2>
        <p class="source-address" :title="sourceText">{{ sourceText }}</p>
      </div>
      <span class="status-pill" :class="status.tone"><i />{{ t(status.key) }}</span
      ><PinMenu
        :pin="pin"
        :presets="presets"
        :runtime="runtime"
        :busy="busy"
        @action="emit('action', pin.id, $event)"
        @export="emit('export', pin.id)"
      />
    </div>
    <div class="pin-meta">
      <span
        ><AppIcon :name="pin.mode === 'desktop' ? 'window' : 'expand'" :size="13" />{{
          pin.mode === 'desktop' ? t('ui.card.metaDesktopMode') : t('ui.card.metaFloating')
        }}</span
      ><span>{{ Math.round(pin.width) }} × {{ Math.round(pin.height) }}</span
      ><span v-if="pin.mode === 'floating' && pin.alwaysOnTop" class="top-label"
        ><AppIcon name="pin" :size="12" />{{ t('ui.card.metaPinned') }}</span
      ><span v-if="activePreset" class="preset-label" :title="activePreset"
        ><AppIcon name="check" :size="12" />{{
          t('ui.card.metaPreset', { name: activePreset })
        }}</span
      >
    </div>
    <p v-if="runtimeError" class="pin-error" role="status">{{ runtimeError }}</p>
    <div class="pin-actions">
      <button
        class="icon-button settings-button"
        :disabled="busy"
        :title="t('ui.card.settings')"
        :aria-label="t('ui.card.settings')"
        @click="emit('settings', pin.id)"
      >
        <AppIcon name="settings" :size="17" /></button
      ><button
        v-if="bringFront"
        class="icon-button locate-button"
        :disabled="busy"
        :title="t('ui.card.bringFront')"
        :aria-label="t('ui.card.bringFront')"
        @click="emit('action', pin.id, 'focus')"
      >
        <AppIcon name="target" :size="16" /></button
      ><button
        v-if="closable"
        class="secondary"
        :disabled="busy"
        :title="t('ui.card.closeTitle')"
        @click="emit('action', pin.id, 'close')"
      >
        <AppIcon name="power" :size="16" />{{ t('ui.card.close') }}</button
      ><button
        v-if="hideable"
        class="secondary"
        :disabled="busy"
        @click="emit('action', pin.id, 'hide')"
      >
        <AppIcon name="eye" :size="16" />{{ t('ui.card.hide') }}</button
      ><button
        v-if="primary"
        class="primary"
        :disabled="busy"
        @click="emit('action', pin.id, primary.operation)"
      >
        <AppIcon
          :name="working ? 'refresh' : primary.icon"
          :size="16"
          :class="{ spinning: working }"
        />{{ working ? t('ui.common.working') : t(primary.labelKey) }}
      </button>
    </div>
  </article>
</template>
