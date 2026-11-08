<!-- 功能：复用窗口控件编辑预设草稿，以九宫格与百分比配置显示器内的相对位置。 -->
<script setup lang="ts">
import { computed, reactive } from 'vue';
import { useI18n } from 'vue-i18n';
import WindowFields from './WindowFields.vue';
import { fallbackWindowFields } from './platform';
import { presetLabel } from './presets';
import type { PlatformCapabilities, PresetInput, WindowPreset } from '../../types';

const props = defineProps<{
  preset: WindowPreset;
  capabilities: PlatformCapabilities;
  copy: boolean;
  formId: string;
  busy: boolean;
}>();
const emit = defineEmits<{ save: [input: PresetInput]; dirty: [value: boolean] }>();
const { t } = useI18n();
const { id: _id, builtin: _builtin, ...fields } = props.preset;
// 旧平台预设先回退到当前能力组合；只有显式保存后才替换原模板。
const supported = fallbackWindowFields(fields, props.capabilities);
const input = reactive<PresetInput>({
  ...supported,
  name: props.copy
    ? Array.from(t('ui.presets.copyName', { name: presetLabel(props.preset) }))
        .slice(0, 40)
        .join('')
    : presetLabel(props.preset),
});
const fx = computed({
  get: () => Math.round(input.fx * 10000) / 100,
  set: (value: number) => {
    input.fx = value / 100;
  },
});
const fy = computed({
  get: () => Math.round(input.fy * 10000) / 100,
  set: (value: number) => {
    input.fy = value / 100;
  },
});
const anchors = [
  'topLeft',
  'top',
  'topRight',
  'left',
  'center',
  'right',
  'bottomLeft',
  'bottom',
  'bottomRight',
] as const;

/// 用固定相对比例设置九个常见落点，任意百分比仍可通过下方数字微调。
function anchor(index: number) {
  input.fx = (index % 3) / 2;
  input.fy = Math.floor(index / 3) / 2;
}

/// 提交的是模板字段快照，不读取或修改任何页面的运行状态。
function save() {
  emit('save', { ...input, name: input.name.trim() });
}
</script>

<template>
  <form
    :id="formId"
    class="pin-settings-form"
    @submit.prevent="save"
    @input="emit('dirty', true)"
    @change="emit('dirty', true)"
  >
    <fieldset class="form-fields" :disabled="busy">
      <label class="field-label"
        >{{ t('ui.presets.name') }}<input v-model="input.name" maxlength="40" required autofocus
      /></label>
      <p class="settings-hint">{{ t('ui.presets.snapshotHint') }}</p>
      <label class="check-row preset-fullscreen"
        ><input v-model="input.fullscreen" type="checkbox" />{{ t('ui.presets.fullscreen') }}</label
      >
      <WindowFields
        :values="input"
        :fullscreen="input.fullscreen"
        :capabilities="capabilities"
        :disabled="busy"
      />
      <div class="form-section">
        <div class="section-head">
          <h3>{{ t('ui.presets.relativePosition') }}</h3>
          <label
            class="setting-toggle"
            :title="!capabilities.positionLock ? t('ui.pinSettings.platformLimited') : undefined"
            ><strong>{{ t('ui.pinSettings.lockPosition') }}</strong
            ><input
              v-model="input.positionLocked"
              class="switch"
              type="checkbox"
              role="switch"
              :disabled="!capabilities.positionLock"
          /></label>
        </div>
        <fieldset class="form-fields" :disabled="input.fullscreen">
          <div class="anchor-grid">
            <button
              v-for="(label, index) in anchors"
              :key="label"
              type="button"
              :class="{
                active: input.fx === (index % 3) / 2 && input.fy === Math.floor(index / 3) / 2,
              }"
              @click="
                anchor(index);
                emit('dirty', true);
              "
            >
              {{ t(`ui.presets.anchors.${label}`) }}
            </button>
          </div>
          <div class="geometry-fields">
            <label class="field-label"
              >{{ t('ui.presets.horizontalPercent')
              }}<input v-model.number="fx" type="number" min="0" max="100" step="0.01" required
            /></label>
            <label class="field-label"
              >{{ t('ui.presets.verticalPercent')
              }}<input v-model.number="fy" type="number" min="0" max="100" step="0.01" required
            /></label>
          </div>
        </fieldset>
      </div>
    </fieldset>
  </form>
</template>
