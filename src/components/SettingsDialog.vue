<!-- 功能：编辑启动恢复、语言与快捷键，在独立区域提供退出应用入口并展示操作错误。 -->
<script setup lang="ts">
import { computed, reactive, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import AppDialog from './AppDialog.vue';
import AppIcon from './AppIcon.vue';
import AppSelect from './AppSelect.vue';
import { text } from '../i18n/messages';
import { desktopAvailable } from '../services/desktop';
import type { SettingsInput, UiMessage } from '../types';

const props = defineProps<{ settings: SettingsInput; busy: boolean; error: UiMessage | null }>();
const emit = defineEmits<{ close: []; save: [input: SettingsInput]; quit: [] }>();
const { t } = useI18n();
const input = reactive<SettingsInput>({ ...props.settings });
const formId = useId();
const languageOptions = computed(
  () =>
    [
      { value: 'zh-CN', label: t('ui.settings.languageZh') },
      { value: 'en-US', label: t('ui.settings.languageEn') },
      { value: 'system', label: t('ui.settings.languageSystem') },
    ] as const,
);

/// 仅提交可编辑字段，快捷键冲突时保持弹窗和原有输入。
function save() {
  emit('save', {
    locale: input.locale,
    restoreSession: input.restoreSession,
    managerShortcut: input.managerShortcut.trim(),
    recoverShortcut: input.recoverShortcut.trim(),
  });
}
</script>

<template>
  <AppDialog :title="t('ui.settings.title')" :busy="busy" @close="emit('close')">
    <form :id="formId" @submit.prevent="save">
      <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
      <fieldset :disabled="busy" class="form-fields">
        <label class="setting-toggle"
          ><span
            ><strong>{{ t('ui.settings.restoreSession') }}</strong></span
          ><input v-model="input.restoreSession" class="switch" type="checkbox" role="switch"
        /></label>
        <AppSelect
          v-model="input.locale"
          :label="t('ui.settings.language')"
          :options="languageOptions"
          :disabled="busy"
          autofocus
        />
        <div class="form-section">
          <h3>{{ t('ui.settings.shortcuts') }}</h3>
          <label class="field-label"
            >{{ t('ui.settings.managerShortcut')
            }}<input
              v-model="input.managerShortcut"
              required
              :placeholder="t('ui.settings.managerPlaceholder')"
          /></label>
          <label class="field-label"
            >{{ t('ui.settings.recoverShortcut')
            }}<input
              v-model="input.recoverShortcut"
              required
              :placeholder="t('ui.settings.recoverPlaceholder')"
          /></label>
        </div>
      </fieldset>
    </form>
    <div class="settings-quit">
      <p>{{ t('ui.settings.quitHint') }}</p>
      <button
        type="button"
        class="secondary"
        :disabled="busy || !desktopAvailable"
        @click="emit('quit')"
      >
        <AppIcon name="power" :size="16" />{{ t('ui.settings.quit') }}
      </button>
    </div>
    <template #footer>
      <button type="button" class="secondary" :disabled="busy" @click="emit('close')">
        {{ t('ui.common.cancel') }}</button
      ><button class="primary" :disabled="busy" type="submit" :form="formId">
        {{ busy ? t('ui.common.working') : t('ui.settings.save') }}
      </button>
    </template>
  </AppDialog>
</template>
