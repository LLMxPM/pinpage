<!-- 功能：复用窗口外观与尺寸、位置控件；即时设置按字段提交，预设编辑保留为草稿。 -->
<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import HintTip from '../../components/HintTip.vue';
import { MAX_HEIGHT, MAX_WIDTH, MIN_HEIGHT, MIN_WIDTH } from './geometryLimits';
import { dragHintKey, hasLimitedWindowFeatures } from './platform';
import type { PlatformCapabilities, WindowValues } from '../../types';

/// 预设编辑没有绝对坐标，只有页面设置传入 position，用于合并“尺寸与位置”分组。
interface PositionValues {
  x: number;
  y: number;
  positionLocked: boolean;
}
const props = defineProps<{
  values: Omit<WindowValues, 'x' | 'y' | 'positionLocked'>;
  position?: PositionValues;
  capabilities: PlatformCapabilities;
  fullscreen?: boolean;
  disabled?: boolean;
}>();
const emit = defineEmits<{ commit: [key: keyof WindowValues] }>();
const { t } = useI18n();
const dragHint = computed(() => t(dragHintKey(props.capabilities)));
const limited = computed(() => hasLimitedWindowFeatures(props.capabilities));
const borderless = computed(() => props.values.mode === 'desktop' || !props.values.decorations);
const showBorder = computed({
  get: () => props.values.mode === 'floating' && props.values.decorations,
  set: (value: boolean) => {
    props.values.decorations = value;
  },
});
const showOnTop = computed({
  get: () => props.values.mode === 'floating' && props.values.alwaysOnTop,
  set: (value: boolean) => {
    props.values.alwaysOnTop = value;
  },
});

/// 尺寸与坐标允许临时空值，仅在回车或失焦且合法时提交。
function commitInteger(event: Event, key: 'width' | 'height' | 'x' | 'y') {
  const field = event.target as HTMLInputElement;
  if (field.checkValidity()) emit('commit', key);
  else field.reportValidity();
}

/// 回车结束当前字段编辑，复用失焦校验而不提交整个弹窗。
function blurField(event: Event) {
  (event.target as HTMLInputElement).blur();
}
</script>

<template>
  <fieldset class="form-fields" :disabled="disabled">
    <div class="form-section">
      <h3>{{ t('ui.pinSettings.appearance') }}</h3>
      <fieldset class="mode-picker">
        <legend>{{ t('ui.pinSettings.modeLegend') }}</legend>
        <label
          v-for="mode in ['floating', 'desktop'] as const"
          :key="mode"
          :class="{ active: values.mode === mode }"
          :title="
            mode === 'desktop' && !capabilities.desktopMode
              ? t('ui.pinSettings.platformLimited')
              : undefined
          "
        >
          <input
            v-model="values.mode"
            type="radio"
            :value="mode"
            :disabled="mode === 'desktop' && !capabilities.desktopMode"
            @change="emit('commit', 'mode')"
          />
          <strong>{{
            t(mode === 'floating' ? 'ui.pinSettings.floating' : 'ui.pinSettings.desktop')
          }}</strong>
        </label>
      </fieldset>
      <div class="pin-setting-checks">
        <label class="check-row"
          ><input
            v-model="showOnTop"
            type="checkbox"
            :disabled="values.mode === 'desktop'"
            @change="emit('commit', 'alwaysOnTop')"
          />{{ t('ui.pinSettings.top') }}</label
        >
        <label class="check-row"
          ><input
            v-model="showBorder"
            type="checkbox"
            :disabled="values.mode === 'desktop'"
            @change="emit('commit', 'decorations')"
          />{{ t('ui.pinSettings.border') }}</label
        >
        <span class="check-with-tip">
          <label
            class="check-row"
            :title="!capabilities.rounded ? t('ui.pinSettings.platformLimited') : undefined"
            ><input
              v-model="values.rounded"
              type="checkbox"
              :disabled="!borderless || !capabilities.rounded"
              @change="emit('commit', 'rounded')"
            />{{ t('ui.pinSettings.rounded') }}</label
          >
          <HintTip :text="dragHint" />
        </span>
      </div>
      <p v-if="limited" class="settings-hint">{{ t('ui.pinSettings.platformLimited') }}</p>
    </div>
    <div class="form-section">
      <div class="section-head">
        <h3>{{ t(position ? 'ui.pinSettings.geometry' : 'ui.pinSettings.size') }}</h3>
        <div class="lock-toggles">
          <label
            class="setting-toggle"
            :title="!capabilities.sizeLock ? t('ui.pinSettings.platformLimited') : undefined"
            ><strong>{{ t('ui.pinSettings.lockSize') }}</strong
            ><input
              v-model="values.sizeLocked"
              class="switch"
              type="checkbox"
              role="switch"
              :disabled="!capabilities.sizeLock"
              @change="emit('commit', 'sizeLocked')"
          /></label>
          <label
            v-if="position"
            class="setting-toggle"
            :title="!capabilities.positionLock ? t('ui.pinSettings.platformLimited') : undefined"
            ><strong>{{ t('ui.pinSettings.lockPosition') }}</strong
            ><input
              v-model="position.positionLocked"
              class="switch"
              type="checkbox"
              role="switch"
              :disabled="!capabilities.positionLock"
              @change="emit('commit', 'positionLocked')"
          /></label>
        </div>
      </div>
      <div class="geometry-fields">
        <label class="field-label"
          >{{ t('ui.pinSettings.width')
          }}<input
            v-model.number="values.width"
            type="number"
            :min="MIN_WIDTH"
            :max="MAX_WIDTH"
            required
            :disabled="fullscreen"
            @blur="commitInteger($event, 'width')"
            @keydown.enter.prevent="blurField"
        /></label>
        <label class="field-label"
          >{{ t('ui.pinSettings.height')
          }}<input
            v-model.number="values.height"
            type="number"
            :min="MIN_HEIGHT"
            :max="MAX_HEIGHT"
            required
            :disabled="fullscreen"
            @blur="commitInteger($event, 'height')"
            @keydown.enter.prevent="blurField"
        /></label>
        <template v-if="position">
          <label class="field-label"
            >{{ t('ui.pinSettings.positionX')
            }}<input
              v-model.number="position.x"
              type="number"
              min="-2147483648"
              max="2147483647"
              step="1"
              required
              @blur="commitInteger($event, 'x')"
              @keydown.enter.prevent="blurField"
          /></label>
          <label class="field-label"
            >{{ t('ui.pinSettings.positionY')
            }}<input
              v-model.number="position.y"
              type="number"
              min="-2147483648"
              max="2147483647"
              step="1"
              required
              @blur="commitInteger($event, 'y')"
              @keydown.enter.prevent="blurField"
          /></label>
        </template>
      </div>
    </div>
  </fieldset>
</template>
