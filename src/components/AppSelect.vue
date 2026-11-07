<!-- 功能：提供统一的自定义单选下拉框，支持键盘导航、输入定位和视口内浮层。 -->
<script setup lang="ts" generic="T extends string">
import { computed, nextTick, onMounted, onUnmounted, ref, useId, watch } from 'vue';
import AppIcon from './AppIcon.vue';

const props = defineProps<{
  label: string;
  options: readonly { value: T; label: string }[];
  disabled?: boolean;
  autofocus?: boolean;
}>();
const model = defineModel<T>({ required: true });
const root = ref<HTMLElement>();
const trigger = ref<HTMLButtonElement>();
const popup = ref<HTMLElement>();
const open = ref(false);
const active = ref(0);
const listId = useId();
const position = ref({ top: '0px', left: '0px', width: '0px', maxHeight: '240px' });
const selected = computed(() => props.options.find((option) => option.value === model.value));
let search = '';
let searchTimer: ReturnType<typeof setTimeout> | undefined;

/// 浮层挂到 body，避免被弹窗滚动容器裁切，并根据上下空间选择展开方向。
function place() {
  if (!open.value || !trigger.value) return;
  const rect = trigger.value.getBoundingClientRect();
  const scrollArea = trigger.value.closest('.dialog-body')?.getBoundingClientRect();
  if (scrollArea && (rect.bottom < scrollArea.top || rect.top > scrollArea.bottom)) {
    close();
    return;
  }
  const below = window.innerHeight - rect.bottom - 14;
  const above = rect.top - 14;
  const desired = Math.min(props.options.length * 38 + 12, 240);
  const upward = below < desired && above > below;
  const height = Math.max(38, Math.min(desired, upward ? above : below));
  const width = Math.min(rect.width, window.innerWidth - 16);
  position.value = {
    top: `${upward ? rect.top - height - 6 : rect.bottom + 6}px`,
    left: `${Math.max(8, Math.min(rect.left, window.innerWidth - width - 8))}px`,
    width: `${width}px`,
    maxHeight: `${height}px`,
  };
}

/// 保持键盘活动项可见，焦点始终留在触发器上供弹窗管理。
async function reveal() {
  await nextTick();
  popup.value
    ?.querySelector<HTMLElement>(`[data-index="${active.value}"]`)
    ?.scrollIntoView({ block: 'nearest' });
}

/// 展开时从已选项开始，空列表和禁用状态不产生浮层。
function expand() {
  if (props.disabled || !props.options.length || trigger.value?.matches(':disabled')) return;
  active.value = Math.max(
    0,
    props.options.findIndex((option) => option.value === model.value),
  );
  open.value = true;
  place();
  void reveal();
}

/// 收起不会提交活动项，Escape 和失焦均保留原值。
function close() {
  open.value = false;
  search = '';
  clearTimeout(searchTimer);
}

/// 鼠标和键盘共用选择入口，更新值后保持触发器焦点。
function choose(index: number) {
  const option = props.options[index];
  if (!option || props.disabled || trigger.value?.matches(':disabled')) return;
  model.value = option.value;
  close();
  trigger.value?.focus();
}

/// 支持方向键、首尾定位、确认、取消和前缀输入，Escape 优先关闭下拉而非弹窗。
function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    if (open.value) {
      event.preventDefault();
      event.stopPropagation();
      close();
    }
    return;
  }
  if (event.key === 'Tab') {
    close();
    return;
  }
  if (['Enter', ' '].includes(event.key)) {
    event.preventDefault();
    if (open.value) choose(active.value);
    else expand();
    return;
  }
  if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
    event.preventDefault();
    if (!open.value) {
      expand();
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') return;
    }
    if (event.key === 'Home') active.value = 0;
    else if (event.key === 'End') active.value = props.options.length - 1;
    else
      active.value =
        (active.value + (event.key === 'ArrowDown' ? 1 : -1) + props.options.length) %
        props.options.length;
    void reveal();
    return;
  }
  if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault();
    if (!open.value) expand();
    search += event.key.toLocaleLowerCase();
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      search = '';
    }, 600);
    const index = props.options.findIndex((option) =>
      option.label.toLocaleLowerCase().startsWith(search),
    );
    if (index >= 0) {
      active.value = index;
      void reveal();
    }
  }
}

/// 点击组件以外的区域时收起，浮层内部点击不抢走触发器焦点。
function outside(event: PointerEvent) {
  if (
    event.target instanceof Node &&
    !root.value?.contains(event.target) &&
    !popup.value?.contains(event.target)
  )
    close();
}

watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) close();
  },
);
onMounted(() => {
  document.addEventListener('pointerdown', outside);
  document.addEventListener('scroll', place, true);
  window.addEventListener('resize', place);
});
onUnmounted(() => {
  close();
  document.removeEventListener('pointerdown', outside);
  document.removeEventListener('scroll', place, true);
  window.removeEventListener('resize', place);
});
</script>

<template>
  <div ref="root" class="select-control">
    <span class="select-label" @click="trigger?.focus()">{{ label }}</span>
    <button
      ref="trigger"
      type="button"
      class="select-trigger"
      role="combobox"
      aria-haspopup="listbox"
      :aria-label="label"
      :aria-expanded="open"
      :aria-controls="open ? listId : undefined"
      :aria-activedescendant="open ? `${listId}-${active}` : undefined"
      :disabled="disabled || !options.length"
      :autofocus="autofocus"
      @click="open ? close() : expand()"
      @keydown="onKeydown"
      @blur="close"
    >
      <span :title="selected?.label">{{ selected?.label ?? '—' }}</span
      ><AppIcon name="chevron" :size="16" :class="{ rotated: open }" />
    </button>
    <Teleport to="body">
      <div
        v-if="open"
        :id="listId"
        ref="popup"
        class="select-popup"
        role="listbox"
        :aria-label="label"
        :style="position"
        @mousedown.prevent
      >
        <div
          v-for="(option, index) in options"
          :id="`${listId}-${index}`"
          :key="option.value"
          class="select-option"
          :class="{ active: index === active, selected: option.value === model }"
          role="option"
          :aria-selected="option.value === model"
          :data-index="index"
          :title="option.label"
          @pointermove="active = index"
          @click="choose(index)"
        >
          <span>{{ option.label }}</span
          ><AppIcon v-if="option.value === model" name="check" :size="16" />
        </div>
      </div>
    </Teleport>
  </div>
</template>
