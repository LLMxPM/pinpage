<!-- 功能：编辑单个页面的桌面模式、无边框圆角、尺寸位置和独立锁定。 -->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue';
import AppIcon from '../../components/AppIcon.vue';
import { text } from '../../i18n/messages';
import type { PinConfig, PinSettingsInput, UiMessage } from '../../types';

const props = defineProps<{ pin: PinConfig; busy: boolean; error: UiMessage | null }>();
const emit = defineEmits<{ close: []; save: [input: PinSettingsInput] }>();
const { t } = useI18n();
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
        <h2 id="pin-settings-title">{{ t('ui.pinSettings.title') }}</h2>
        <button
          ref="heading"
          class="icon-button"
          :aria-label="t('ui.pinSettings.close')"
          :disabled="busy"
          @click="close"
        >
          <AppIcon name="close" />
        </button>
      </div>
      <form @submit.prevent="save">
        <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
        <label class="field-label"
          >{{ t('ui.pinSettings.name') }}<input v-model="input.name" required maxlength="80"
        /></label>
        <fieldset class="mode-picker">
          <legend>{{ t('ui.pinSettings.modeLegend') }}</legend>
          <label :class="{ active: input.mode === 'floating' }"
            ><input v-model="input.mode" type="radio" value="floating" /><span
              ><strong>{{ t('ui.pinSettings.floating') }}</strong
              ><small>{{ t('ui.pinSettings.floatingHint') }}</small></span
            ></label
          ><label :class="{ active: input.mode === 'desktop' }"
            ><input v-model="input.mode" type="radio" value="desktop" /><span
              ><strong>{{ t('ui.pinSettings.desktop') }}</strong
              ><small>{{ t('ui.pinSettings.desktopHint') }}</small></span
            ></label
          >
        </fieldset>
        <div class="pin-setting-checks">
          <label class="check-row"
            ><input v-model="showOnTop" type="checkbox" :disabled="input.mode === 'desktop'" />{{
              t('ui.pinSettings.top')
            }}</label
          ><label class="check-row"
            ><input v-model="showBorder" type="checkbox" :disabled="input.mode === 'desktop'" />{{
              t('ui.pinSettings.border')
            }}</label
          ><label class="check-row"
            ><input v-model="input.rounded" type="checkbox" :disabled="!borderless" />{{
              t('ui.pinSettings.rounded')
            }}</label
          >
        </div>
        <div class="geometry-fields">
          <label class="field-label"
            >{{ t('ui.pinSettings.width')
            }}<input
              v-model.number="input.width"
              type="number"
              min="160"
              max="3840"
              required /></label
          ><label class="field-label"
            >{{ t('ui.pinSettings.height')
            }}<input
              v-model.number="input.height"
              type="number"
              min="100"
              max="2160"
              required /></label
          ><label class="field-label"
            >{{ t('ui.pinSettings.positionX')
            }}<input v-model.number="input.x" type="number" required /></label
          ><label class="field-label"
            >{{ t('ui.pinSettings.positionY')
            }}<input v-model.number="input.y" type="number" required
          /></label>
        </div>
        <p class="geometry-hint">{{ t('ui.pinSettings.geometryHint') }}</p>
        <div class="lock-settings">
          <label
            ><input v-model="input.positionLocked" type="checkbox" /><span
              ><strong>{{ t('ui.pinSettings.lockPosition') }}</strong
              ><small>{{ t('ui.pinSettings.lockPositionHint') }}</small></span
            ></label
          ><label
            ><input v-model="input.sizeLocked" type="checkbox" /><span
              ><strong>{{ t('ui.pinSettings.lockSize') }}</strong
              ><small>{{ t('ui.pinSettings.lockSizeHint') }}</small></span
            ></label
          >
        </div>
        <p class="settings-hint">{{ t('ui.pinSettings.lockHint') }}</p>
        <div class="dialog-footer">
          <button type="button" class="secondary" :disabled="busy" @click="close">
            {{ t('ui.pinSettings.cancel') }}</button
          ><button class="primary" :disabled="busy" type="submit">
            {{ busy ? t('ui.pinSettings.busy') : t('ui.pinSettings.save') }}
          </button>
        </div>
      </form>
    </section>
  </div>
</template>
