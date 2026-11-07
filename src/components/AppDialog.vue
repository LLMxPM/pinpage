<!-- 功能：统一弹窗外观、滚动区域、焦点约束、Esc 关闭和关闭后焦点恢复。 -->
<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import AppIcon from './AppIcon.vue';

const props = defineProps<{
  title: string;
  description?: string;
  busy?: boolean;
  wide?: boolean;
}>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
const titleId = useId();
const dialog = ref<HTMLElement>();
const previousFocus = document.activeElement as HTMLElement | null;
const root = document.getElementById('app');
const previousInert = root?.inert ?? false;
const previousOverflow = document.body.style.overflow;

/// 操作进行中保留弹窗与输入，避免用户误以为任务已取消。
function close() {
  if (!props.busy) emit('close');
}

/// 在当前弹窗内循环 Tab 焦点，Escape 与遮罩使用相同关闭约束。
function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault();
    close();
    return;
  }
  if (event.key !== 'Tab') return;
  const fields = Array.from(
    dialog.value?.querySelectorAll<HTMLElement>(
      'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), summary, [tabindex="0"]',
    ) ?? [],
  ).filter((field) => field.getClientRects().length);
  const first = fields[0];
  const last = fields[fields.length - 1];
  if (!first) {
    event.preventDefault();
    dialog.value?.focus();
  } else if (
    event.shiftKey &&
    (document.activeElement === first || document.activeElement === dialog.value)
  ) {
    event.preventDefault();
    last?.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

onMounted(async () => {
  if (root) root.inert = true;
  document.body.style.overflow = 'hidden';
  await nextTick();
  (dialog.value?.querySelector<HTMLElement>('[autofocus]') ?? dialog.value)?.focus();
});
onUnmounted(() => {
  if (root) root.inert = previousInert;
  document.body.style.overflow = previousOverflow;
  if (previousFocus?.isConnected) previousFocus.focus();
});
</script>

<template>
  <Teleport to="body">
    <div class="modal-backdrop" @click.self="close" @keydown="onKeydown">
      <section
        ref="dialog"
        class="app-dialog panel"
        :class="{ wide }"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId"
        tabindex="-1"
        :aria-busy="busy"
      >
        <header class="dialog-heading">
          <div>
            <h2 :id="titleId">{{ title }}</h2>
            <p v-if="description">{{ description }}</p>
          </div>
          <button
            type="button"
            class="icon-button"
            :disabled="busy"
            :aria-label="t('ui.common.closeDialog')"
            @click="close"
          >
            <AppIcon name="close" />
          </button>
        </header>
        <div class="dialog-body"><slot /></div>
        <footer v-if="$slots.footer" class="dialog-footer"><slot name="footer" /></footer>
      </section>
    </div>
  </Teleport>
</template>
