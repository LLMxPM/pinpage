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
const emit = defineEmits<{ commit: [key: keyof WindowValues | (keyof WindowValues)[]] }>();
const { t } = useI18n();
const dragHint = computed(() => t(dragHintKey(props.capabilities)));
const limited = computed(() => hasLimitedWindowFeatures(props.capabilities));
const showOnTop = computed({
  get: () => props.values.mode === 'floating' && props.values.alwaysOnTop,
  set: (value: boolean) => {
    props.values.alwaysOnTop = value;
  },
});

/// 边框样式三态：系统边框 / 无边框 / 无边框圆角，替代原先两个互斥勾选。
type FrameStyle = 'system' | 'borderless' | 'rounded';
const frameOptions = [
  { value: 'system', key: 'ui.pinSettings.frameSystem' },
  { value: 'borderless', key: 'ui.pinSettings.frameBorderless' },
  { value: 'rounded', key: 'ui.pinSettings.frameRounded' },
] as const;
const frameStyle = computed<FrameStyle>({
  get: () => {
    if (props.values.mode === 'floating' && props.values.decorations) return 'system';
    return props.values.rounded ? 'rounded' : 'borderless';
  },
  set: (style) => {
    if (style === 'system') {
      props.values.decorations = true;
      props.values.rounded = false;
    } else {
      // 桌面模式固定无边框，但保留浮动时的系统边框偏好，切回浮动可沿用。
      if (props.values.mode === 'floating') props.values.decorations = false;
      props.values.rounded = style === 'rounded';
    }
  },
});
/// 桌面模式禁用系统边框；圆角受平台能力裁剪，其余样式始终可选。
function frameDisabled(style: FrameStyle): boolean {
  if (style === 'system') return props.values.mode === 'desktop';
  if (style === 'rounded') return !props.capabilities.rounded;
  return false;
}
function frameTitle(style: FrameStyle): string | undefined {
  if (style === 'system' && props.values.mode === 'desktop')
    return t('ui.pinSettings.frameDesktopHint');
  if (style === 'rounded' && !props.capabilities.rounded)
    return t('ui.pinSettings.platformLimited');
  return undefined;
}

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
      <fieldset class="mode-picker frame-picker">
        <legend>
          {{ t('ui.pinSettings.frameStyle') }}
          <HintTip :text="dragHint" />
        </legend>
        <label
          v-for="option in frameOptions"
          :key="option.value"
          :class="{ active: frameStyle === option.value }"
          :title="frameTitle(option.value)"
        >
          <input
            v-model="frameStyle"
            type="radio"
            :value="option.value"
            :disabled="frameDisabled(option.value)"
            @change="emit('commit', ['decorations', 'rounded'])"
          />
          <strong>{{ t(option.key) }}</strong>
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
        <span class="check-with-tip">
          <label
            class="check-row"
            :title="
              !capabilities.transparentBackground ? t('ui.pinSettings.platformLimited') : undefined
            "
            ><input
              v-model="values.transparentBackground"
              type="checkbox"
              :disabled="!capabilities.transparentBackground"
              @change="emit('commit', 'transparentBackground')"
            />{{ t('ui.pinSettings.transparentBackground') }}</label
          >
          <HintTip :text="t('ui.pinSettings.transparentHint')" />
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
