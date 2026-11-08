<!-- 功能：在窗口设置中应用、另存和覆盖预设，并在保存成功后才清除名称。 -->
<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import AppIcon from '../../components/AppIcon.vue';
import { presetLabel, presetSummary } from './presets';
import type { PinConfig, WindowPreset } from '../../types';

const props = defineProps<{
  pin: PinConfig;
  presets: WindowPreset[];
  apply: (id: string) => Promise<boolean>;
  save: (name: string, presetId?: string) => Promise<boolean>;
  disabled?: boolean;
}>();
const emit = defineEmits<{ manage: []; dirty: [value: boolean]; pending: [value: boolean] }>();
const { t } = useI18n();
const naming = ref(false);
const name = ref('');
const saving = ref(false);
const overwrite = ref<WindowPreset | null>(null);
// 记录最近选择项，调整窗口成为自定义后仍可明确覆盖原模板。
const selectedId = ref(props.pin.preset);
const selected = computed(() => props.presets.find((preset) => preset.id === selectedId.value));
/// 同名预设（常见于保存过的副本）追加尺寸，避免胶囊看起来完全重复。
const duplicateLabels = computed(() => {
  const counts = new Map<string, number>();
  for (const preset of props.presets) {
    const label = presetLabel(preset);
    counts.set(label, (counts.get(label) ?? 0) + 1);
  }
  return new Set([...counts].filter(([, count]) => count > 1).map(([label]) => label));
});
/// 只有重名时才补充尺寸，普通预设保持短标签。
function chipLabel(preset: WindowPreset) {
  const label = presetLabel(preset);
  return duplicateLabels.value.has(label) ? `${label} · ${presetSummary(preset)}` : label;
}
watch(
  () => props.pin.preset,
  (id) => {
    if (id) selectedId.value = id;
  },
);
watch(
  () => naming.value && name.value.trim().length > 0,
  (value) => emit('dirty', value),
);
watch(saving, (value) => emit('pending', value));

/// 应用成功后才更新覆盖目标，失败保留原先选择。
async function pick(id: string) {
  if (await props.apply(id)) selectedId.value = id;
}

/// 名称和目标在请求成功前保留，失败后可直接重试。
async function save() {
  const target = overwrite.value;
  const label = target ? presetLabel(target) : name.value.trim();
  if (!label || saving.value) return;
  saving.value = true;
  try {
    if (await props.save(label, target?.id)) {
      naming.value = false;
      overwrite.value = null;
      name.value = '';
      selectedId.value = props.pin.preset;
    }
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="preset-section">
    <div class="section-head">
      <h3>{{ t('ui.pinSettings.presets') }}</h3>
      <button
        type="button"
        class="text-button"
        :disabled="disabled || saving"
        @click="emit('manage')"
      >
        {{ t('ui.presets.manage') }}
      </button>
    </div>
    <div class="preset-chips">
      <button
        v-for="preset in presets"
        :key="preset.id"
        type="button"
        class="preset-chip"
        :class="{ active: pin.preset === preset.id }"
        :title="`${presetLabel(preset)} · ${presetSummary(preset)}`"
        :disabled="disabled || saving"
        @click="pick(preset.id)"
      >
        <AppIcon v-if="pin.preset === preset.id" name="check" :size="14" />{{ chipLabel(preset) }}
      </button>
    </div>
    <div v-if="overwrite" class="preset-confirm" role="status">
      <p>{{ t('ui.presets.overwriteConfirm', { name: chipLabel(overwrite) }) }}</p>
      <p class="settings-hint">{{ t('ui.presets.snapshotHint') }}</p>
      <div class="inline-actions">
        <button type="button" class="secondary" :disabled="saving" @click="overwrite = null">
          {{ t('ui.common.cancel') }}</button
        ><button type="button" class="primary" :disabled="saving" @click="save">
          {{ t('ui.presets.confirmUpdate') }}
        </button>
      </div>
    </div>
    <div v-else-if="naming" class="preset-save-row">
      <input
        v-model="name"
        :placeholder="t('ui.pinSettings.presetNamePlaceholder')"
        maxlength="40"
        :disabled="saving"
        @keydown.enter.prevent="save"
      />
      <button type="button" class="secondary" :disabled="saving" @click="naming = false">
        {{ t('ui.common.cancel') }}
      </button>
      <button
        type="button"
        class="primary"
        :disabled="!name.trim() || saving || disabled"
        @click="save"
      >
        {{ t('ui.presets.save') }}
      </button>
    </div>
    <div v-else class="inline-actions">
      <button
        type="button"
        class="preset-save-toggle"
        :disabled="disabled || saving"
        @click="naming = true"
      >
        <AppIcon name="plus" :size="14" />{{ t('ui.pinSettings.presetSave') }}
      </button>
      <button
        v-if="selected && !selected.builtin && !pin.preset"
        type="button"
        class="text-button"
        :disabled="disabled || saving"
        @click="overwrite = selected"
      >
        {{ t('ui.presets.updateFromWindow', { name: chipLabel(selected) }) }}
      </button>
    </div>
  </div>
</template>
