<!-- 功能：以单页章节编辑启动恢复、语言与窗口手势，并提供更新和关于入口。 -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, useId, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import AppDialog from './AppDialog.vue';
import AppIcon from './AppIcon.vue';
import AppSelect from './AppSelect.vue';
import UpdateSettings from '../features/updates/UpdateSettings.vue';
import AboutSettings from '../features/updates/AboutSettings.vue';
import { text } from '../i18n/messages';
import { desktopAvailable } from '../services/desktop';
import type { SettingsInput, UiMessage } from '../types';

const props = defineProps<{
  settings: SettingsInput;
  busy: boolean;
  error: UiMessage | null;
  updateAvailable: boolean;
}>();
const emit = defineEmits<{ close: []; save: [input: SettingsInput]; quit: [] }>();
const { t } = useI18n();
const input = reactive<SettingsInput>({ ...props.settings });
const baseline = { ...props.settings };
const formId = useId();
const baseId = useId();
type Section = 'general' | 'controls' | 'about';
const active = ref<Section>('general');
const layout = ref<HTMLElement>();
const sections = computed(
  () =>
    [
      { id: 'general', label: t('ui.settings.navGeneral') },
      { id: 'controls', label: t('ui.settings.navControls') },
      { id: 'about', label: t('ui.settings.navAbout') },
    ] as const,
);
const languageOptions = computed(
  () =>
    [
      { value: 'zh-CN', label: t('ui.settings.languageZh') },
      { value: 'en-US', label: t('ui.settings.languageEn') },
      { value: 'system', label: t('ui.settings.languageSystem') },
    ] as const,
);
const gestureModifierOptions = computed(
  () =>
    [
      { value: 'alt', label: t('ui.settings.gestureAlt') },
      { value: 'ctrl', label: t('ui.settings.gestureCtrl') },
      { value: 'shift', label: t('ui.settings.gestureShift') },
      { value: 'disabled', label: t('ui.settings.gestureDisabled') },
    ] as const,
);
/// 只比较可编辑字段，避免额外配置造成虚假更改。
function normalized(value: SettingsInput): SettingsInput {
  return {
    locale: value.locale,
    gestureModifier: value.gestureModifier,
    restoreSession: value.restoreSession,
  };
}
const current = computed(() => normalized(input));
const dirty = computed(
  () => JSON.stringify(current.value) !== JSON.stringify(normalized(baseline)),
);
const sectionId = (section: Section) => `${baseId}-section-${section}`;
let scrollRoot: HTMLElement | null = null;
let frame = 0;
let lockedUntil = 0;

/// 窄窗口的章节导航吸顶并占据内容高度，定位时需要避让它。
function navOffset() {
  const nav = layout.value?.querySelector('.settings-nav');
  return nav instanceof HTMLElement && getComputedStyle(nav).flexDirection === 'row'
    ? nav.offsetHeight + 8
    : 0;
}

/// 以章节顶部是否越过内容区顶部判断当前节点；滚动到底部时固定高亮最后一章。
function updateActive() {
  if (!scrollRoot || Date.now() < lockedUntil) return;
  const rootTop = scrollRoot.getBoundingClientRect().top;
  const offset = navOffset() + 36;
  let currentSection: Section = 'general';
  for (const section of sections.value) {
    const element = document.getElementById(sectionId(section.id));
    if (element && element.getBoundingClientRect().top - rootTop <= offset) {
      currentSection = section.id;
    }
  }
  if (
    scrollRoot.scrollHeight > scrollRoot.clientHeight + 4 &&
    scrollRoot.scrollTop + scrollRoot.clientHeight >= scrollRoot.scrollHeight - 4
  ) {
    currentSection = 'about';
  }
  active.value = currentSection;
}

/// 连续滚动事件合并到一帧，减少章节位置读取。
function onScroll() {
  if (frame) return;
  frame = requestAnimationFrame(() => {
    frame = 0;
    updateActive();
  });
}

/// 点击章节节点平滑定位；短暂锁定高亮，避免途经章节时节点闪烁。
function goTo(section: Section) {
  if (!scrollRoot) return;
  const element = document.getElementById(sectionId(section));
  if (!element) return;
  active.value = section;
  lockedUntil = Date.now() + 700;
  const top =
    element.getBoundingClientRect().top -
    scrollRoot.getBoundingClientRect().top +
    scrollRoot.scrollTop;
  scrollRoot.scrollTo({ top: Math.max(0, top - navOffset() - 4), behavior: 'smooth' });
}

onMounted(() => {
  scrollRoot = layout.value?.closest('.dialog-body') ?? null;
  scrollRoot?.addEventListener('scroll', onScroll, { passive: true });
  updateActive();
});
onUnmounted(() => {
  scrollRoot?.removeEventListener('scroll', onScroll);
  if (frame) cancelAnimationFrame(frame);
});

/// 保存失败时把错误提示带回可视区，避免长页面底部看不到原因。
watch(
  () => props.error,
  (error) => {
    if (error) scrollRoot?.scrollTo({ top: 0, behavior: 'smooth' });
  },
);

/// 仅在没有改动时阻止重复提交。
function save() {
  if (!dirty.value) return;
  emit('save', { ...current.value });
}
</script>

<template>
  <AppDialog :title="t('ui.settings.title')" :busy="busy" large @close="emit('close')">
    <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
    <form :id="formId" @submit.prevent="save">
      <div ref="layout" class="settings-layout">
        <nav class="settings-nav" :aria-label="t('ui.settings.title')">
          <button
            v-for="section in sections"
            :key="section.id"
            type="button"
            :class="{ active: active === section.id }"
            :aria-current="active === section.id ? 'true' : undefined"
            @click="goTo(section.id)"
          >
            <span>{{ section.label }}</span>
            <span
              v-if="section.id === 'about' && updateAvailable"
              class="nav-badge"
              :title="t('ui.updates.availableBadge')"
            />
          </button>
        </nav>
        <div class="settings-content">
          <section
            :id="sectionId('general')"
            class="settings-section"
            :aria-label="t('ui.settings.navGeneral')"
          >
            <AppSelect
              v-model="input.locale"
              :label="t('ui.settings.language')"
              :options="languageOptions"
              :disabled="busy"
              autofocus
            />
            <label class="setting-toggle"
              ><span
                ><strong>{{ t('ui.settings.restoreSession') }}</strong></span
              ><input
                v-model="input.restoreSession"
                class="switch"
                type="checkbox"
                role="switch"
                :disabled="busy"
            /></label>
          </section>
          <section
            :id="sectionId('controls')"
            class="settings-section"
            :aria-label="t('ui.settings.navControls')"
          >
            <AppSelect
              v-model="input.gestureModifier"
              :label="t('ui.settings.gestureModifier')"
              :options="gestureModifierOptions"
              :disabled="busy"
            />
            <p class="settings-hint">{{ t('ui.settings.gestureHint') }}</p>
          </section>
          <section
            :id="sectionId('about')"
            class="settings-section"
            :aria-label="t('ui.settings.navAbout')"
          >
            <AboutSettings>
              <UpdateSettings :busy="busy" />
            </AboutSettings>
          </section>
        </div>
      </div>
    </form>
    <template #footer>
      <div class="settings-footer">
        <button
          type="button"
          class="text-button settings-quit-button"
          :disabled="busy || !desktopAvailable"
          :title="t('ui.settings.quitHint')"
          @click="emit('quit')"
        >
          <AppIcon name="power" :size="15" />{{ t('ui.settings.quit') }}
        </button>
        <div class="settings-footer-actions">
          <button
            v-if="dirty"
            type="button"
            class="secondary"
            :disabled="busy"
            @click="emit('close')"
          >
            {{ t('ui.common.cancel') }}</button
          ><button v-if="dirty" class="primary" :disabled="busy" type="submit" :form="formId">
            {{ busy ? t('ui.common.working') : t('ui.settings.save') }}</button
          ><button v-else type="button" class="primary" :disabled="busy" @click="emit('close')">
            {{ t('ui.common.close') }}
          </button>
        </div>
      </div>
    </template>
  </AppDialog>
</template>
