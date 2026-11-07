<!-- 功能：呈现单个悬浮窗的来源、状态和原生操作入口。 -->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed } from 'vue';
import AppIcon from '../../components/AppIcon.vue';
import { fromAppMessage, text } from '../../i18n/messages';
import type { PinConfig, PinRuntime } from '../../types';

const props = defineProps<{ pin: PinConfig; runtime?: PinRuntime; busy: boolean }>();
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
/// 运行错误只携带消息码，渲染时按当前语言翻译。
const runtimeError = computed(() =>
  props.runtime?.error ? text(fromAppMessage(props.runtime.error)) : '',
);
const status = computed(() =>
  props.runtime?.error
    ? t('ui.card.statusAttention')
    : !props.runtime?.open
      ? props.pin.enabled
        ? t('ui.card.statusNotOpen')
        : t('ui.card.statusClosed')
      : props.runtime.loading
        ? t('ui.card.statusLoading')
        : props.runtime.visible
          ? t('ui.card.statusRunning')
          : t('ui.card.statusHidden'),
);

/// 向工作台提交窗口 ID 与操作，前端不自行修改原生状态。
function action(operation: string) {
  emit('action', props.pin.id, operation);
}
</script>

<template>
  <article class="pin-card panel">
    <div class="pin-card-header">
      <div class="source-icon" :class="pin.source.kind">
        <AppIcon :name="pin.source.kind === 'url' ? 'window' : 'folder'" :size="23" />
      </div>
      <span
        class="status-pill"
        :class="{ warning: runtime?.error, muted: !runtime?.open || !runtime?.visible }"
        ><i></i>{{ status }}</span
      ><button
        class="icon-button close-pin"
        :disabled="busy"
        :aria-label="t('ui.card.deleteAria', { name: pin.name })"
        :title="t('ui.card.deleteTitle')"
        @click="action('delete')"
      >
        <AppIcon name="close" :size="17" />
      </button>
    </div>
    <h3>{{ pin.name }}</h3>
    <p class="source-address" :title="sourceText">{{ sourceText }}</p>
    <p v-if="runtimeError" class="pin-error" role="status">{{ runtimeError }}</p>
    <div class="pin-meta">
      <span>{{ Math.round(pin.width) }} × {{ Math.round(pin.height) }}</span
      ><span>{{
        pin.mode === 'desktop'
          ? t('ui.card.metaDesktopMode')
          : pin.decorations
            ? t('ui.card.metaBordered')
            : t('ui.card.metaBorderless')
      }}</span
      ><span v-if="pin.mode === 'floating' && pin.alwaysOnTop" class="top-label"
        ><AppIcon name="pin" :size="12" />{{ t('ui.card.metaPinned') }}</span
      >
      <span v-if="pin.positionLocked">{{ t('ui.card.metaPositionLocked') }}</span
      ><span v-if="pin.sizeLocked">{{ t('ui.card.metaSizeLocked') }}</span>
      <span v-if="(pin.mode === 'desktop' || !pin.decorations) && pin.rounded">
        {{ t('ui.card.metaRounded') }}
      </span>
    </div>
    <div class="pin-actions">
      <label class="pin-enable"
        ><input
          type="checkbox"
          role="switch"
          :aria-label="
            t(runtime?.open ? 'ui.card.switchOnAria' : 'ui.card.switchOffAria', { name: pin.name })
          "
          :checked="!!runtime?.open"
          :disabled="busy"
          @change="action(runtime?.open ? 'close' : 'open')"
        /><span>{{ runtime?.open ? t('ui.card.switchOn') : t('ui.card.switchOff') }}</span></label
      >
      <div class="action-group">
        <button
          class="icon-button"
          :disabled="busy"
          :title="t('ui.card.actionSettings')"
          :aria-label="t('ui.card.actionSettings')"
          @click="emit('settings', pin.id)"
        >
          <AppIcon name="settings" :size="16" />
        </button>
        <button
          class="icon-button"
          :disabled="busy || !runtime?.open"
          :title="t('ui.card.actionFocus')"
          :aria-label="t('ui.card.actionFocus')"
          @click="action('focus')"
        >
          <AppIcon name="expand" :size="16" />
        </button>
        <button
          class="icon-button"
          :class="{ selected: pin.mode === 'floating' && pin.alwaysOnTop }"
          :disabled="busy || !runtime?.open || pin.mode === 'desktop'"
          :title="pin.alwaysOnTop ? t('ui.card.actionUntop') : t('ui.card.actionTop')"
          :aria-label="pin.alwaysOnTop ? t('ui.card.actionUntop') : t('ui.card.actionTop')"
          @click="action('top')"
        >
          <AppIcon name="pin" :size="16" /></button
        ><button
          class="icon-button"
          :disabled="busy || !runtime?.open"
          :title="t('ui.card.actionBorder')"
          :aria-label="t('ui.card.actionBorder')"
          @click="action('border')"
        >
          <AppIcon name="window" :size="16" /></button
        ><button
          class="icon-button"
          :disabled="busy || !runtime?.open"
          :title="t('ui.card.actionReload')"
          :aria-label="t('ui.card.actionReload')"
          @click="action('reload')"
        >
          <AppIcon name="refresh" :size="16" /></button
        ><button
          class="icon-button"
          :disabled="busy || !runtime?.open || pin.source.kind !== 'bundled'"
          :title="
            pin.source.kind === 'bundled'
              ? t('ui.card.actionExportTitle')
              : t('ui.card.actionExportDisabled')
          "
          :aria-label="t('ui.card.actionExport')"
          @click="emit('export', pin.id)"
        >
          <AppIcon name="download" :size="16" /></button
        ><button
          class="icon-button"
          :disabled="busy || !runtime?.open"
          :title="t('ui.card.actionVisibility')"
          :aria-label="t('ui.card.actionVisibilityAria')"
          @click="action('visibility')"
        >
          <AppIcon name="eye" :size="16" />
        </button>
      </div>
    </div>
  </article>
</template>
