<!-- 功能：展示即时窗口设置与预设，向弹窗底栏同步未生效与失败状态，允许保存为预设。 -->
<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import WindowFields from './WindowFields.vue';
import PinPresetPicker from './PinPresetPicker.vue';
import { useLiveWindow } from './useLiveWindow';
import type { PinConfig, WindowPatch, WindowPreset } from '../../types';

const props = defineProps<{
  pin: PinConfig;
  presets: WindowPreset[];
  open: boolean;
  apply: (patch: WindowPatch) => Promise<PinConfig | null>;
  applyPreset: (id: string) => Promise<boolean>;
  savePreset: (name: string, id?: string) => Promise<boolean>;
}>();
const emit = defineEmits<{
  manage: [];
  dirty: [value: boolean];
  pending: [value: boolean];
  unapplied: [value: boolean];
  failed: [value: boolean];
}>();
const { t } = useI18n();
const live = useLiveWindow(() => props.pin, props.apply);
const presetDirty = ref(false);
const presetPending = ref(false);
watch(
  () => live.dirty || presetDirty.value,
  (value) => emit('dirty', value),
  { immediate: true },
);
watch(
  () => live.pending > 0 || presetPending.value,
  (value) => emit('pending', value),
);
// 底栏据此展示未生效数值与恢复入口，避免在表单里插入会造成高度抖动的一行。
watch(
  () => live.dirty,
  (value) => emit('unapplied', value),
  { immediate: true },
);
watch(
  () => live.failed,
  (value) => emit('failed', value),
  { immediate: true },
);

/// 预设操作等待即时字段落地，不会把正在输入的数值遗漏在保存结果之外。
async function applyPreset(id: string) {
  return (await live.flush()) && (await props.applyPreset(id));
}
/// 另存和覆盖都只读取已经确认生效的窗口快照。
async function savePreset(name: string, id?: string) {
  return (await live.flush()) && (await props.savePreset(name, id));
}
/// 放弃未生效的数字输入，由弹窗底栏的“恢复实际值”调用。
defineExpose({ resetInput: () => live.reset() });
</script>

<template>
  <div class="pin-settings-form window-settings">
    <p v-if="!open" class="settings-hint closed-hint">
      {{ t('ui.pinSettings.closedHint') }}
    </p>
    <PinPresetPicker
      :pin="pin"
      :presets="presets"
      :apply="applyPreset"
      :save="savePreset"
      @manage="emit('manage')"
      @dirty="presetDirty = $event"
      @pending="presetPending = $event"
    />
    <WindowFields :values="live.input" :position="live.input" @commit="live.commit" />
  </div>
</template>
