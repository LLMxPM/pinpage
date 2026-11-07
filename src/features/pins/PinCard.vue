<!-- 功能：呈现单个悬浮窗的来源、状态和原生操作入口。 -->
<script setup lang="ts">
import { computed } from 'vue';
import AppIcon from '../../components/AppIcon.vue';
import type { PinConfig, PinRuntime } from '../../types';

const props = defineProps<{ pin: PinConfig; runtime?: PinRuntime; busy: boolean }>();
const emit = defineEmits<{
  action: [id: string, operation: string];
  settings: [id: string];
  export: [id: string];
}>();
const sourceText = computed(() =>
  props.pin.source.kind === 'url' ? props.pin.source.url : `已导入 · ${props.pin.source.entry}`,
);
const status = computed(() =>
  props.runtime?.error
    ? '需要关注'
    : !props.runtime?.open
      ? props.pin.enabled
        ? '尚未打开'
        : '已关闭'
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
        <AppIcon :name="pin.source.kind === 'url' ? 'window' : 'folder'" :size="23" />
      </div>
      <span
        class="status-pill"
        :class="{ warning: runtime?.error, muted: !runtime?.open || !runtime?.visible }"
        ><i></i>{{ status }}</span
      ><button
        class="icon-button close-pin"
        :disabled="busy"
        :aria-label="`删除页面 ${pin.name}`"
        title="删除页面记录"
        @click="action('delete')"
      >
        <AppIcon name="close" :size="17" />
      </button>
    </div>
    <h3>{{ pin.name }}</h3>
    <p class="source-address" :title="sourceText">{{ sourceText }}</p>
    <p v-if="runtime?.error" class="pin-error" role="status">{{ runtime.error }}</p>
    <div class="pin-meta">
      <span>{{ Math.round(pin.width) }} × {{ Math.round(pin.height) }}</span
      ><span>{{
        pin.mode === 'desktop' ? '桌面模式' : pin.decorations ? '标准边框' : '无边框'
      }}</span
      ><span v-if="pin.mode === 'floating' && pin.alwaysOnTop" class="top-label"
        ><AppIcon name="pin" :size="12" />已置顶</span
      >
      <span v-if="pin.positionLocked">位置已锁定</span><span v-if="pin.sizeLocked">大小已锁定</span>
      <span v-if="(pin.mode === 'desktop' || !pin.decorations) && pin.rounded">圆角</span>
    </div>
    <div class="pin-actions">
      <label class="pin-enable"
        ><input
          type="checkbox"
          role="switch"
          :aria-label="`${runtime?.open ? '关闭' : '开启'}悬浮页 ${pin.name}`"
          :checked="!!runtime?.open"
          :disabled="busy"
          @change="action(runtime?.open ? 'close' : 'open')"
        /><span>{{ runtime?.open ? '已开启' : '开启页面' }}</span></label
      >
      <div class="action-group">
        <button
          class="icon-button"
          :disabled="busy"
          title="页面设置"
          aria-label="页面设置"
          @click="emit('settings', pin.id)"
        >
          <AppIcon name="settings" :size="16" />
        </button>
        <button
          class="icon-button"
          :disabled="busy || !runtime?.open"
          title="显示页面"
          aria-label="显示页面"
          @click="action('focus')"
        >
          <AppIcon name="expand" :size="16" />
        </button>
        <button
          class="icon-button"
          :class="{ selected: pin.mode === 'floating' && pin.alwaysOnTop }"
          :disabled="busy || !runtime?.open || pin.mode === 'desktop'"
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
          :disabled="busy || !runtime?.open || pin.source.kind !== 'bundled'"
          :title="
            pin.source.kind === 'bundled'
              ? '导出页面包（含页面数据）'
              : '网址页面没有可导出的站点文件'
          "
          aria-label="导出页面包"
          @click="emit('export', pin.id)"
        >
          <AppIcon name="download" :size="16" /></button
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
