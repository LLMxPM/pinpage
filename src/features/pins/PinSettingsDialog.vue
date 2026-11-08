<!-- 功能：将窗口即时设置与内容保存分区，关闭前确认放弃尚未生效的输入。 -->
<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import AppDialog from '../../components/AppDialog.vue';
import PinContentSettings from './PinContentSettings.vue';
import PinWindowSettings from './PinWindowSettings.vue';
import { text } from '../../i18n/messages';
import type { PinConfig, PinContentInput, UiMessage, WindowPatch, WindowPreset } from '../../types';

const props = defineProps<{
  pin: PinConfig;
  presets: WindowPreset[];
  busy: boolean;
  error: UiMessage | null;
  open: boolean;
  canUndo: boolean;
  applyWindow: (patch: WindowPatch) => Promise<PinConfig | null>;
  applyPreset: (id: string) => Promise<boolean>;
  savePreset: (name: string, id?: string) => Promise<boolean>;
  saveContent: (input: PinContentInput) => Promise<boolean | undefined>;
  undo: () => Promise<boolean>;
}>();
const emit = defineEmits<{ close: []; error: [message: UiMessage]; manage: [] }>();
const { t } = useI18n();
const formId = useId();
const tab = ref<'window' | 'content'>('window');
const contentDirty = ref(false);
const windowDirty = ref(false);
const windowUnapplied = ref(false);
const windowFailed = ref(false);
const windowSettings = ref<InstanceType<typeof PinWindowSettings> | null>(null);
/// 底栏与自动保存说明共用一行，展示已生效的实际几何，避免表单高度随输入抖动。
const actualGeometry = computed(() =>
  t('ui.pinSettings.unappliedGeometry', {
    width: Math.round(props.pin.width),
    height: Math.round(props.pin.height),
    x: props.pin.x ?? '—',
    y: props.pin.y ?? '—',
  }),
);
const localPending = ref(false);
const choosing = ref(false);
const confirming = ref(false);
const closeRequested = ref(false);

// 点击“完成”触发数字失焦提交时，等待提交结束后继续关闭，不要求用户再点一次。
watch(
  () => props.busy || localPending.value,
  (pending) => {
    if (!pending && closeRequested.value) {
      closeRequested.value = false;
      close();
    }
  },
);

/// 放弃未生效的窗口数字输入，交由表单组件回填实际值。
function resetWindowInput() {
  windowSettings.value?.resetInput();
}

/// 已生效窗口操作不会被“关闭”还原，只确认丢弃未提交的内容或数字输入。
function close() {
  if (choosing.value) return;
  if (props.busy || localPending.value) {
    closeRequested.value = true;
    return;
  }
  if (contentDirty.value || windowDirty.value) confirming.value = true;
  else emit('close');
}
</script>

<template>
  <AppDialog
    :title="t('ui.pinSettings.title')"
    :description="pin.name"
    :busy="busy || localPending || choosing"
    wide
    @close="close"
  >
    <template #heading-extra>
      <nav v-if="!confirming" class="settings-tabs" :aria-label="t('ui.pinSettings.title')">
        <button
          v-for="item in ['window', 'content'] as const"
          :key="item"
          type="button"
          :class="{ active: tab === item }"
          :aria-pressed="tab === item"
          @click="tab = item"
        >
          {{ t(item === 'window' ? 'ui.pinSettings.tabWindow' : 'ui.pinSettings.tabContent')
          }}<span v-if="item === 'content' && contentDirty"> ·</span>
        </button>
      </nav>
    </template>
    <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
    <div v-if="confirming" class="discard-confirm">
      <p>{{ t('ui.pinSettings.discardConfirm') }}</p>
      <p class="settings-hint">{{ t('ui.pinSettings.discardHint') }}</p>
    </div>
    <div v-show="!confirming">
      <PinWindowSettings
        v-show="tab === 'window'"
        ref="windowSettings"
        :pin="pin"
        :presets="presets"
        :open="open"
        :apply="applyWindow"
        :apply-preset="applyPreset"
        :save-preset="savePreset"
        @manage="emit('manage')"
        @dirty="windowDirty = $event"
        @pending="localPending = $event"
        @unapplied="windowUnapplied = $event"
        @failed="windowFailed = $event"
      />
      <PinContentSettings
        v-show="tab === 'content'"
        :pin="pin"
        :busy="busy"
        :form-id="formId"
        :save="saveContent"
        @error="emit('error', $event)"
        @dirty="contentDirty = $event"
        @pending="choosing = $event"
      />
    </div>
    <template #footer>
      <template v-if="confirming">
        <button type="button" class="secondary" @click="confirming = false">
          {{ t('ui.pinSettings.continueEditing') }}
        </button>
        <button type="button" class="primary" @click="emit('close')">
          {{ t('ui.pinSettings.discardAndClose') }}
        </button>
      </template>
      <template v-else>
        <div class="footer-left">
          <template v-if="tab === 'window'">
            <template v-if="windowUnapplied || windowFailed">
              <span class="footer-status" :class="{ 'error-text': windowFailed }" role="status">
                {{ windowFailed ? t('ui.pinSettings.status.failed') : actualGeometry }}
              </span>
              <button
                v-if="windowUnapplied"
                type="button"
                class="text-button"
                :disabled="localPending"
                @click="resetWindowInput"
              >
                {{ t('ui.pinSettings.restoreActual') }}
              </button>
            </template>
            <template v-else>
              <button
                v-if="canUndo"
                type="button"
                class="text-button"
                :disabled="busy || localPending"
                @click="undo()"
              >
                {{ t('ui.pinSettings.undoWindow') }}
              </button>
              <span class="footer-note">{{ t('ui.pinSettings.footerWindow') }}</span>
            </template>
          </template>
        </div>
        <button
          v-if="tab === 'content' && contentDirty"
          class="primary"
          type="submit"
          :form="formId"
          :disabled="busy || choosing"
        >
          {{ t('ui.pinSettings.saveContent') }}
        </button>
        <button
          type="button"
          :class="tab === 'content' && contentDirty ? 'secondary' : 'primary'"
          :disabled="choosing"
          @click="close"
        >
          {{ t('ui.pinSettings.done') }}
        </button>
      </template>
    </template>
  </AppDialog>
</template>
