<!-- 功能：收纳页面低频操作，支持外部点击和 Escape 收起，并把删除交给确认流程。 -->
<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import AppIcon from '../../components/AppIcon.vue';
import type { PinConfig, PinRuntime } from '../../types';

defineProps<{ pin: PinConfig; runtime?: PinRuntime; busy: boolean }>();
const emit = defineEmits<{ action: [operation: string]; export: [] }>();
const { t } = useI18n();
const menu = ref<HTMLDetailsElement>();
const opensUp = ref(false);
const menuHeight = ref('');

/// 根据入口上下方的可用空间展开菜单，靠近窗口底部时向上展开。
async function placeMenu() {
  if (!menu.value?.open) return;
  opensUp.value = false;
  menuHeight.value = '';
  await nextTick();
  const trigger = menu.value?.querySelector('summary')?.getBoundingClientRect();
  const popup = menu.value?.querySelector('.menu-popup');
  if (!trigger || !popup) return;
  const below = window.innerHeight - trigger.bottom - 16;
  const above = trigger.top - 16;
  opensUp.value = popup.scrollHeight > below && above > below;
  menuHeight.value = `${Math.max(80, opensUp.value ? above : below)}px`;
}

/// 先恢复摘要焦点再通知工作台，确保确认弹窗关闭后能回到稳定的入口。
function pick(operation: string) {
  if (menu.value) menu.value.open = false;
  menu.value?.querySelector('summary')?.focus();
  if (operation === 'export') emit('export');
  else emit('action', operation);
}

/// 点击其他区域或切换键盘焦点时关闭当前菜单。
function outside(event: Event) {
  if (event.target instanceof Node && !menu.value?.contains(event.target) && menu.value)
    menu.value.open = false;
}

/// Escape 只在菜单已展开时消费，避免拦截其他弹窗的关闭。
function escape(event: KeyboardEvent) {
  if (event.key === 'Escape' && menu.value?.open) {
    event.preventDefault();
    menu.value.open = false;
    menu.value.querySelector('summary')?.focus();
  }
}

onMounted(() => {
  document.addEventListener('pointerdown', outside);
  document.addEventListener('focusin', outside);
  document.addEventListener('keydown', escape);
  window.addEventListener('resize', placeMenu);
});
onUnmounted(() => {
  document.removeEventListener('pointerdown', outside);
  document.removeEventListener('focusin', outside);
  document.removeEventListener('keydown', escape);
  window.removeEventListener('resize', placeMenu);
});
</script>

<template>
  <details ref="menu" class="pin-menu" :class="{ 'opens-up': opensUp }" @toggle="placeMenu">
    <summary
      class="icon-button"
      :aria-label="t('ui.card.moreAria', { name: pin.name })"
      :aria-disabled="busy"
      @click="busy && $event.preventDefault()"
    >
      <AppIcon name="more" :size="21" />
    </summary>
    <div class="menu-popup" :style="{ maxHeight: menuHeight }">
      <button v-if="runtime?.open" :disabled="busy" @click="pick('visibility')">
        <AppIcon name="eye" :size="16" />{{ t(runtime.visible ? 'ui.card.hide' : 'ui.card.show') }}
      </button>
      <button :disabled="busy || !runtime?.open" @click="pick('reload')">
        <AppIcon name="refresh" :size="16" />{{ t('ui.card.actionReload') }}
      </button>
      <button
        v-if="pin.mode === 'floating'"
        :disabled="busy || !runtime?.open"
        @click="pick('top')"
      >
        <AppIcon name="pin" :size="16" />{{
          t(pin.alwaysOnTop ? 'ui.card.actionUntop' : 'ui.card.actionTop')
        }}
      </button>
      <button
        v-if="pin.mode === 'floating'"
        :disabled="busy || !runtime?.open"
        @click="pick('border')"
      >
        <AppIcon name="window" :size="16" />{{
          t(pin.decorations ? 'ui.card.removeBorder' : 'ui.card.restoreBorder')
        }}
      </button>
      <button :disabled="busy || !runtime?.open" @click="pick('recover')">
        <AppIcon name="expand" :size="16" />{{ t('ui.card.recover') }}
      </button>
      <button
        v-if="pin.source.kind === 'bundled'"
        :disabled="busy || !runtime?.open"
        :title="t(runtime?.open ? 'ui.card.actionExportTitle' : 'ui.card.exportNeedsOpen')"
        @click="pick('export')"
      >
        <AppIcon name="download" :size="16" />{{ t('ui.card.actionExport') }}
      </button>
      <button v-if="runtime?.open" :disabled="busy" @click="pick('close')">
        <AppIcon name="power" :size="16" />{{ t('ui.card.close') }}
      </button>
      <div class="menu-divider" />
      <button class="danger-text" :disabled="busy" @click="pick('delete')">
        <AppIcon name="trash" :size="16" />{{ t('ui.card.deleteTitle') }}
      </button>
    </div>
  </details>
</template>
