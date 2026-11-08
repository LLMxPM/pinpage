<!-- 功能：信息图标在悬停或聚焦时显示说明气泡，按视口边界校正位置。 -->
<script setup lang="ts">
import { nextTick, onUnmounted, ref } from 'vue';
import AppIcon from './AppIcon.vue';

const props = defineProps<{ text: string }>();
const trigger = ref<HTMLButtonElement>();
const bubble = ref<HTMLElement>();
const open = ref(false);
const position = ref({ top: 0, left: 0, arrow: 0, below: false });
let listening = false;
// 悬停与键盘聚焦分别记录，任一方仍激活时保持气泡可见。
let hovered = false;
let focused = false;

/// 气泡优先显示在上方，空间不足时翻到下方，并夹在视口左右边界内。
function place() {
  const anchor = trigger.value?.getBoundingClientRect();
  const box = bubble.value?.getBoundingClientRect();
  if (!anchor || !box) return;
  const margin = 8;
  const left = Math.min(
    Math.max(margin, anchor.left + anchor.width / 2 - box.width / 2),
    window.innerWidth - box.width - margin,
  );
  const above = anchor.top - box.height - 8;
  const arrow = Math.min(
    Math.max(14, anchor.left + anchor.width / 2 - left),
    Math.max(14, box.width - 14),
  );
  position.value = {
    top: above >= margin ? above : anchor.bottom + 8,
    left,
    arrow,
    below: above < margin,
  };
}

/// 滚动时收起，避免固定定位的气泡与图标分离。
function hide() {
  open.value = false;
  if (listening) {
    window.removeEventListener('scroll', hide, true);
    window.removeEventListener('resize', place);
    listening = false;
  }
}

/// 先渲染气泡再测量宽度，悬停和键盘聚焦共用同一套定位。
async function show() {
  if (open.value) return;
  open.value = true;
  await nextTick();
  place();
  if (!listening) {
    window.addEventListener('scroll', hide, true);
    window.addEventListener('resize', place);
    listening = true;
  }
}

function enter() {
  hovered = true;
  void show();
}
function leave() {
  hovered = false;
  if (!focused) hide();
}
function focus() {
  focused = true;
  void show();
}
function blur() {
  focused = false;
  if (!hovered) hide();
}

onUnmounted(hide);
</script>

<template>
  <button
    ref="trigger"
    type="button"
    class="hint-tip"
    :aria-label="text"
    @mouseenter="enter"
    @mouseleave="leave"
    @focus="focus"
    @blur="blur"
    @keydown.escape.stop.prevent="hide"
  >
    <AppIcon name="info" :size="14" />
  </button>
  <Teleport to="body">
    <div
      v-if="open"
      ref="bubble"
      class="hint-tip-bubble"
      role="tooltip"
      aria-hidden="true"
      :class="{ below: position.below }"
      :style="{
        top: `${position.top}px`,
        left: `${position.left}px`,
        '--tip-arrow': `${position.arrow}px`,
      }"
    >
      {{ props.text }}
    </div>
  </Teleport>
</template>

<style scoped>
.hint-tip {
  display: inline-flex;
  align-items: center;
  margin: -3px;
  padding: 3px;
  border: 0;
  border-radius: 50%;
  background: none;
  color: #8d99a8;
  cursor: help;
}
.hint-tip:hover,
.hint-tip:focus-visible {
  color: var(--accent);
}
.hint-tip:focus-visible {
  outline: 2px solid #e2a58e;
  outline-offset: 2px;
}
.hint-tip-bubble {
  position: fixed;
  z-index: 60;
  max-width: min(280px, calc(100vw - 16px));
  padding: 8px 11px;
  border-radius: 8px;
  background: #2f3a49;
  color: #fff;
  font-size: 12px;
  line-height: 1.7;
  text-align: left;
  box-shadow: 0 8px 24px #1b25323d;
  pointer-events: none;
}
.hint-tip-bubble::after {
  content: '';
  position: absolute;
  left: var(--tip-arrow, 50%);
  bottom: -4px;
  width: 9px;
  height: 9px;
  border-radius: 1px;
  background: #2f3a49;
  transform: translateX(-50%) rotate(45deg);
}
.hint-tip-bubble.below::after {
  top: -4px;
  bottom: auto;
}
</style>
