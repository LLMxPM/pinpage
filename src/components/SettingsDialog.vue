<!-- 功能：编辑会话恢复与全局快捷键，保存失败时保留输入。 -->
<script setup lang="ts">
import { reactive } from 'vue';
import AppIcon from './AppIcon.vue';
import type { SettingsInput } from '../types';
const props = defineProps<{ settings: SettingsInput; busy: boolean }>();
const emit = defineEmits<{ close: []; save: [input: SettingsInput] }>();
const input = reactive<SettingsInput>({ ...props.settings });

/// 仅提交可编辑设置，窗口配置由后端独立维护。
function save() {
  emit('save', {
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
        <h2 id="settings-title">工作台设置</h2>
        <button class="icon-button" aria-label="关闭设置" @click="emit('close')">
          <AppIcon name="close" />
        </button>
      </div>
      <form @submit.prevent="save">
        <label class="setting-toggle"
          ><span
            ><strong>恢复上次会话</strong><small>启动时重新打开上次退出前保留的页面。</small></span
          ><input v-model="input.restoreSession" type="checkbox" /></label
        ><label class="field-label"
          >打开管理界面<input
            v-model="input.managerShortcut"
            autofocus
            required
            placeholder="Ctrl+Alt+P" /></label
        ><label class="field-label"
          >找回全部窗口<input v-model="input.recoverShortcut" required placeholder="Ctrl+Alt+B"
        /></label>
        <p class="settings-hint">
          找回操作会恢复标准边框，并将窗口居中。快捷键被占用时会保留原有设置。
        </p>
        <div class="dialog-footer">
          <button type="button" class="secondary" @click="emit('close')">取消</button
          ><button class="primary" :disabled="busy" type="submit">保存设置</button>
        </div>
      </form>
    </section>
  </div>
</template>
