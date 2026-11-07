<!-- 功能：编辑单个页面的桌面模式、无边框圆角、尺寸位置和独立锁定。 -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue';
import AppIcon from '../../components/AppIcon.vue';
import type { PinConfig, PinSettingsInput } from '../../types';
const props = defineProps<{ pin: PinConfig; busy: boolean; error: string }>();
const emit = defineEmits<{ close: []; save: [input: PinSettingsInput] }>();
const heading = ref<HTMLButtonElement>();
const input = reactive<PinSettingsInput>({
  name: props.pin.name,
  mode: props.pin.mode,
  alwaysOnTop: props.pin.alwaysOnTop,
  decorations: props.pin.decorations,
  rounded: props.pin.rounded,
  positionLocked: props.pin.positionLocked,
  sizeLocked: props.pin.sizeLocked,
  width: Math.round(props.pin.width),
  height: Math.round(props.pin.height),
  x: props.pin.x ?? 60,
  y: props.pin.y ?? 60,
});
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
const previousFocus = document.activeElement as HTMLElement | null;

/// 关闭后恢复卡片焦点，键盘用户可以继续管理当前页面。
function close() {
  if (!props.busy) emit('close');
}

/// 提交完整可编辑字段，原生层再次校验范围并执行带回滚的设置。
function save() {
  emit('save', { ...input });
}

/// 保持键盘焦点位于当前设置弹层，避免误操作后方窗口列表。
function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault();
    close();
  }
  if (event.key !== 'Tab') return;
  const dialog = heading.value?.closest('section');
  const fields = Array.from(
    dialog?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), select:not([disabled])',
    ) ?? [],
  );
  const first = fields[0];
  const last = fields[fields.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last?.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first?.focus();
  }
}
onMounted(() => {
  heading.value?.focus();
  document.addEventListener('keydown', onKeydown);
});
onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown);
  previousFocus?.focus();
});
</script>

<template>
  <div class="modal-backdrop" @click.self="close">
    <section
      class="settings-dialog pin-settings-dialog panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="pin-settings-title"
    >
      <div class="dialog-heading">
        <h2 id="pin-settings-title">页面设置</h2>
        <button
          ref="heading"
          class="icon-button"
          aria-label="关闭页面设置"
          :disabled="busy"
          @click="close"
        >
          <AppIcon name="close" />
        </button>
      </div>
      <form @submit.prevent="save">
        <div v-if="error" class="notice error-notice" role="alert">{{ error }}</div>
        <label class="field-label"
          >页面名称<input v-model="input.name" required maxlength="80"
        /></label>
        <fieldset class="mode-picker">
          <legend>显示模式</legend>
          <label :class="{ active: input.mode === 'floating' }"
            ><input v-model="input.mode" type="radio" value="floating" /><span
              ><strong>浮动窗口</strong><small>可置顶，方便随时查看</small></span
            ></label
          ><label :class="{ active: input.mode === 'desktop' }"
            ><input v-model="input.mode" type="radio" value="desktop" /><span
              ><strong>固定在桌面</strong><small>留在桌面层，普通窗口盖在上方</small></span
            ></label
          >
        </fieldset>
        <div class="pin-setting-checks">
          <label class="check-row"
            ><input
              v-model="showOnTop"
              type="checkbox"
              :disabled="input.mode === 'desktop'"
            />始终置顶</label
          ><label class="check-row"
            ><input
              v-model="showBorder"
              type="checkbox"
              :disabled="input.mode === 'desktop'"
            />显示系统边框</label
          ><label class="check-row"
            ><input
              v-model="input.rounded"
              type="checkbox"
              :disabled="!borderless"
            />无边框圆角</label
          >
        </div>
        <div class="geometry-fields">
          <label class="field-label"
            >宽度<input
              v-model.number="input.width"
              type="number"
              min="160"
              max="3840"
              required /></label
          ><label class="field-label"
            >高度<input
              v-model.number="input.height"
              type="number"
              min="100"
              max="2160"
              required /></label
          ><label class="field-label"
            >横向位置 X<input v-model.number="input.x" type="number" required /></label
          ><label class="field-label"
            >纵向位置 Y<input v-model.number="input.y" type="number" required
          /></label>
        </div>
        <p class="geometry-hint">宽高随系统缩放，X / Y 为屏幕像素，支持负坐标。</p>
        <div class="lock-settings">
          <label
            ><input v-model="input.positionLocked" type="checkbox" /><span
              ><strong>锁定位置</strong><small>禁止拖动和系统吸附移动</small></span
            ></label
          ><label
            ><input v-model="input.sizeLocked" type="checkbox" /><span
              ><strong>锁定大小</strong><small>禁止拖动边缘改变尺寸</small></span
            ></label
          >
        </div>
        <p class="settings-hint">
          锁定后仍可在此调整数值。关闭页面会保留这些设置，再次开启时恢复。
        </p>
        <div class="dialog-footer">
          <button type="button" class="secondary" :disabled="busy" @click="close">取消</button
          ><button class="primary" :disabled="busy" type="submit">
            {{ busy ? '正在应用…' : '保存并应用' }}
          </button>
        </div>
      </form>
    </section>
  </div>
</template>
