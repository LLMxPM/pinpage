<!-- 功能：编辑界面语言、会话恢复与全局快捷键，保存失败时保留输入。 -->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { reactive } from 'vue';
import AppIcon from './AppIcon.vue';
import type { LocalePreference, SettingsInput } from '../types';

const props = defineProps<{ settings: SettingsInput; busy: boolean }>();
const emit = defineEmits<{ close: []; save: [input: SettingsInput] }>();
const { t } = useI18n();
const input = reactive<SettingsInput>({ ...props.settings });

/// 语言选项固定使用各自自称，避免切换后找不到恢复入口。
const languages: { value: LocalePreference; key: string }[] = [
  { value: 'zh-CN', key: 'ui.settings.languageZh' },
  { value: 'en-US', key: 'ui.settings.languageEn' },
  { value: 'system', key: 'ui.settings.languageSystem' },
];

/// 仅提交可编辑设置，窗口配置由后端独立维护。
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
  <div class="modal-backdrop" @click.self="emit('close')" @keydown.esc="emit('close')">
    <section
      class="settings-dialog panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
      tabindex="-1"
    >
      <div class="dialog-heading">
        <h2 id="settings-title">{{ t('ui.settings.title') }}</h2>
        <button class="icon-button" :aria-label="t('ui.settings.close')" @click="emit('close')">
          <AppIcon name="close" />
        </button>
      </div>
      <form @submit.prevent="save">
        <label class="setting-toggle"
          ><span
            ><strong>{{ t('ui.settings.restoreSession') }}</strong
            ><small>{{ t('ui.settings.restoreSessionHint') }}</small></span
          ><input v-model="input.restoreSession" type="checkbox" /></label
        ><label class="field-label"
          >{{ t('ui.settings.language')
          }}<select v-model="input.locale" :aria-label="t('ui.settings.language')">
            <option v-for="item in languages" :key="item.value" :value="item.value">
              {{ t(item.key) }}
            </option>
          </select></label
        >
        <p class="settings-hint">{{ t('ui.settings.languageHint') }}</p>
        <label class="field-label"
          >{{ t('ui.settings.managerShortcut')
          }}<input
            v-model="input.managerShortcut"
            autofocus
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
        <p class="settings-hint">{{ t('ui.settings.shortcutHint') }}</p>
        <div class="dialog-footer">
          <button type="button" class="secondary" @click="emit('close')">
            {{ t('ui.settings.cancel') }}</button
          ><button class="primary" :disabled="busy" type="submit">
            {{ t('ui.settings.save') }}
          </button>
        </div>
      </form>
    </section>
  </div>
</template>
