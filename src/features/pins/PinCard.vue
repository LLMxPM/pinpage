<!-- 功能：呈现单个悬浮窗的来源、状态和原生操作入口。 -->
<script setup lang="ts">
import { computed } from 'vue';
import AppIcon from '../../components/AppIcon.vue';
import { displayPath } from '../../services/format';
import type { PinConfig, PinRuntime } from '../../types';

const props = defineProps<{ pin: PinConfig; runtime?: PinRuntime; busy: boolean }>();
const emit = defineEmits<{ action: [id: string, operation: string] }>();
const sourceText = computed(() =>
  props.pin.source.kind === 'url'
    ? props.pin.source.url
    : props.pin.source.kind === 'local'
      ? displayPath(props.pin.source.entry)
      : '内置示例 · 按截止时间计时',
);
const status = computed(() =>
  props.runtime?.error
    ? '需要关注'
    : !props.runtime?.open
      ? '尚未打开'
      : props.runtime.loading
        ? '加载中'
        : props.runtime.visible
          ? '运行中'
          : '已隐藏',
);

/// 向工作台提交窗口 ID 与操作，前端不自行修改原生状态。
function action(operation: string) {
  emit('action', props.pin.id, operation);
}
</script>

<template>
  <article class="pin-card panel">
    <div class="pin-card-header">
      <div class="source-icon" :class="pin.source.kind">
        <AppIcon
          :name="
            pin.source.kind === 'example'
              ? 'clock'
              : pin.source.kind === 'local'
                ? 'folder'
                : 'window'
          "
          :size="23"
        />
      </div>
      <span
        class="status-pill"
        :class="{ warning: runtime?.error, muted: !runtime?.open || !runtime?.visible }"
        ><i></i>{{ status }}</span
      ><button
        class="icon-button close-pin"
        :disabled="busy"
        :aria-label="`关闭并删除 ${pin.name}`"
        title="关闭并删除"
        @click="action('close')"
      >
        <AppIcon name="close" :size="17" />
      </button>
    </div>
    <h3>{{ pin.name }}</h3>
    <p class="source-address" :title="sourceText">{{ sourceText }}</p>
    <p v-if="runtime?.error" class="pin-error" role="status">{{ runtime.error }}</p>
    <div class="pin-meta">
      <span>{{ Math.round(pin.width) }} × {{ Math.round(pin.height) }}</span
      ><span>{{ pin.decorations ? '标准边框' : '无边框' }}</span
      ><span v-if="pin.alwaysOnTop" class="top-label"><AppIcon name="pin" :size="12" />已置顶</span>
    </div>
    <div class="pin-actions">
      <button :disabled="busy" @click="action(runtime?.open ? 'focus' : 'reopen')">
        <AppIcon name="expand" :size="15" />{{ runtime?.open ? '显示窗口' : '重新打开' }}
      </button>
      <div class="action-group">
        <button
          class="icon-button"
          :class="{ selected: pin.alwaysOnTop }"
          :disabled="busy || !runtime?.open"
          :title="pin.alwaysOnTop ? '取消置顶' : '设为置顶'"
          :aria-label="pin.alwaysOnTop ? '取消置顶' : '设为置顶'"
          @click="action('top')"
        >
          <AppIcon name="pin" :size="16" /></button
        ><button
          class="icon-button"
          :disabled="busy || !runtime?.open"
          title="切换边框"
          aria-label="切换边框"
          @click="action('border')"
        >
          <AppIcon name="window" :size="16" /></button
        ><button
          class="icon-button"
          :disabled="busy || !runtime?.open"
          title="刷新页面"
          aria-label="刷新页面"
          @click="action('reload')"
        >
          <AppIcon name="refresh" :size="16" /></button
        ><button
          class="icon-button"
          :disabled="busy || !runtime?.open"
          title="显示 / 隐藏"
          aria-label="显示或隐藏窗口"
          @click="action('visibility')"
        >
          <AppIcon name="eye" :size="16" />
        </button>
      </div>
    </div>
  </article>
</template>
