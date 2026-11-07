<!-- 功能：使用统一弹窗编辑页面显示模式、窗口几何和独立锁定，失败时保留输入。 -->
<script setup lang="ts">
import { computed, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import AppDialog from '../../components/AppDialog.vue';
import { text } from '../../i18n/messages';
import { usePinSettingsForm } from './usePinSettingsForm';
import type { PinConfig, PinSettingsInput, UiMessage } from '../../types';

const props = defineProps<{ pin: PinConfig; busy: boolean; error: UiMessage | null }>();
const emit = defineEmits<{ close: []; save: [input: PinSettingsInput] }>();
const { t } = useI18n();
const formId = useId();
const input = usePinSettingsForm(() => props.pin);
const borderless = computed(() => input.mode === 'desktop' || !input.decorations);
const showBorder = computed({
  get: () => input.mode === 'floating' && input.decorations,
  set: (value: boolean) => {
    input.decorations = value;
  },
});
const showOnTop = computed({
  get: () => input.mode === 'floating' && input.alwaysOnTop,
  set: (value: boolean) => {
    input.alwaysOnTop = value;
  },
});

/// 提交完整字段，原生层再次校验范围并执行带回滚的窗口设置。
function save() {
  emit('save', { ...input });
}
</script>

<template>
  <AppDialog
    :title="t('ui.pinSettings.title')"
    :description="pin.name"
    :busy="busy"
    wide
    @close="emit('close')"
  >
    <form :id="formId" @submit.prevent="save">
      <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
      <fieldset class="form-fields" :disabled="busy">
        <label class="field-label"
          >{{ t('ui.pinSettings.name')
          }}<input v-model="input.name" required maxlength="80" autofocus
        /></label>
        <fieldset class="mode-picker">
          <legend>{{ t('ui.pinSettings.modeLegend') }}</legend>
          <label :class="{ active: input.mode === 'floating' }"
            ><input v-model="input.mode" type="radio" value="floating" /><span
              ><strong>{{ t('ui.pinSettings.floating') }}</strong></span
            ></label
          >
          <label :class="{ active: input.mode === 'desktop' }"
            ><input v-model="input.mode" type="radio" value="desktop" /><span
              ><strong>{{ t('ui.pinSettings.desktop') }}</strong></span
            ></label
          >
        </fieldset>
        <div class="pin-setting-checks">
          <label class="check-row"
            ><input v-model="showOnTop" type="checkbox" :disabled="input.mode === 'desktop'" />{{
              t('ui.pinSettings.top')
            }}</label
          >
          <label class="check-row"
            ><input v-model="showBorder" type="checkbox" :disabled="input.mode === 'desktop'" />{{
              t('ui.pinSettings.border')
            }}</label
          >
          <label class="check-row"
            ><input v-model="input.rounded" type="checkbox" :disabled="!borderless" />{{
              t('ui.pinSettings.rounded')
            }}</label
          >
        </div>
        <div class="form-section">
          <h3>{{ t('ui.pinSettings.geometry') }}</h3>
          <div class="geometry-fields">
            <label class="field-label"
              >{{ t('ui.pinSettings.width')
              }}<input v-model.number="input.width" type="number" min="160" max="3840" required
            /></label>
            <label class="field-label"
              >{{ t('ui.pinSettings.height')
              }}<input v-model.number="input.height" type="number" min="100" max="2160" required
            /></label>
            <label class="field-label"
              >{{ t('ui.pinSettings.positionX')
              }}<input v-model.number="input.x" type="number" required
            /></label>
            <label class="field-label"
              >{{ t('ui.pinSettings.positionY')
              }}<input v-model.number="input.y" type="number" required
            /></label>
          </div>
        </div>
        <div class="lock-settings">
          <label class="setting-toggle"
            ><span
              ><strong>{{ t('ui.pinSettings.lockPosition') }}</strong></span
            ><input v-model="input.positionLocked" class="switch" type="checkbox" role="switch"
          /></label>
          <label class="setting-toggle"
            ><span
              ><strong>{{ t('ui.pinSettings.lockSize') }}</strong></span
            ><input v-model="input.sizeLocked" class="switch" type="checkbox" role="switch"
          /></label>
        </div>
      </fieldset>
    </form>
    <template #footer>
      <button type="button" class="secondary" :disabled="busy" @click="emit('close')">
        {{ t('ui.common.cancel') }}</button
      ><button class="primary" :disabled="busy" type="submit" :form="formId">
        {{ busy ? t('ui.pinSettings.busy') : t('ui.pinSettings.save') }}
      </button>
    </template>
  </AppDialog>
</template>
