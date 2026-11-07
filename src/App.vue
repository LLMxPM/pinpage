<!-- 功能：以页面管理为中心组织工作台、示例库、新建流程和统一的设置与删除弹窗。 -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from 'vue-i18n';
import AppIcon from './components/AppIcon.vue';
import AppDialog from './components/AppDialog.vue';
import SettingsDialog from './components/SettingsDialog.vue';
import NewPinForm from './features/launcher/NewPinForm.vue';
import ExampleGallery from './features/launcher/ExampleGallery.vue';
import PinCard from './features/pins/PinCard.vue';
import PinSettingsDialog from './features/pins/PinSettingsDialog.vue';
import { useWorkspace } from './features/pins/useWorkspace';
import { text } from './i18n/messages';
import { desktopAvailable } from './services/desktop';
import type { CreatePin, PinConfig } from './types';

const { t } = useI18n();
const {
  snapshot,
  pins,
  busy,
  pending,
  loading,
  ready,
  error,
  toast,
  notices,
  settingsOpen,
  editingPinId,
  editingPin,
  savePin,
  openCount,
  create,
  createExample,
  exportPin,
  action,
  saveSettings,
  quit,
} = useWorkspace();
const creatingOpen = ref(false);
const launchMode = ref<'url' | 'import'>('url');
const deletingPin = ref<PinConfig | null>(null);
// 管理窗口使用自绘标题栏，拖动和窗口按钮直接调用原生窗口能力。
const appWindow = desktopAvailable ? getCurrentWindow() : null;
const maximized = ref(false);
let unwatchResize: (() => void) | null = null;
const modalOpen = computed(
  () => creatingOpen.value || settingsOpen.value || !!editingPin.value || !!deletingPin.value,
);

/// 顶栏空白处按下左键拖动窗口，双击在最大化与还原之间切换；按钮和链接保持自身行为。
function dragWindow(event: MouseEvent) {
  if (!appWindow || event.button !== 0) return;
  if (event.target instanceof Element && event.target.closest('button, a, input, select, summary'))
    return;
  event.preventDefault();
  if (event.detail === 2) void appWindow.toggleMaximize();
  else void appWindow.startDragging();
}

/// 关闭按钮销毁管理 WebView，与托盘一致；页面窗口不受影响。
async function windowControl(operation: 'minimize' | 'maximize' | 'close') {
  if (!appWindow) return;
  if (operation === 'minimize') await appWindow.minimize();
  else if (operation === 'maximize') await appWindow.toggleMaximize();
  else await appWindow.close();
}

onMounted(async () => {
  if (!appWindow) return;
  maximized.value = await appWindow.isMaximized();
  unwatchResize = await appWindow.onResized(async () => {
    maximized.value = await appWindow.isMaximized();
  });
});
onUnmounted(() => unwatchResize?.());

/// 新任务清除旧错误，避免把上一项操作失败带入当前表单。
function openCreate(mode: 'url' | 'import' = 'url') {
  error.value = null;
  launchMode.value = mode;
  creatingOpen.value = true;
}

/// 创建成功才收起弹窗，失败时保留来源、名称和窗口选项。
async function createAndClose(input: CreatePin) {
  if (await create(input)) creatingOpen.value = false;
}

/// 所有页面删除先进入明确的确认弹窗，其余操作交给原生层。
function cardAction(id: string, operation: string) {
  error.value = null;
  if (operation === 'delete') deletingPin.value = pins.value.find((pin) => pin.id === id) ?? null;
  else void action(id, operation);
}

/// 仅在原生删除成功后关闭确认框，删除失败的原因留在框内。
async function confirmDelete() {
  if (deletingPin.value && (await action(deletingPin.value.id, 'delete'))) deletingPin.value = null;
}

/// 页面设置从卡片进入，共享相同的错误与关闭语义。
function editPin(id: string) {
  error.value = null;
  editingPinId.value = id;
}

/// 应用设置只有一个入口，打开时清除其他任务遗留的错误。
function openSettings() {
  error.value = null;
  settingsOpen.value = true;
}
</script>

<template>
  <div v-if="!ready" class="boot-shell" :aria-label="t('ui.boot.ariaLabel')" role="status">
    <AppIcon name="pin" :size="30" /><span>PinPage</span>
  </div>
  <div v-else class="app-shell">
    <header class="topbar" @mousedown="dragWindow">
      <a class="brand" href="#workspace" :aria-label="t('ui.brand.ariaLabel')"
        ><span class="brand-mark"><AppIcon name="pin" :size="16" /></span><span>PinPage</span></a
      >
      <span v-if="!desktopAvailable" class="preview-badge">{{ t('ui.desktop.preview') }}</span>
      <div class="topbar-actions">
        <button class="secondary compact" @click="openSettings">
          <AppIcon name="settings" :size="16" />{{ t('ui.sidebar.settings') }}</button
        ><button class="primary compact" :disabled="busy" @click="openCreate()">
          <AppIcon name="plus" :size="17" />{{ t('ui.workspace.add') }}
        </button>
      </div>
      <div v-if="desktopAvailable" class="window-controls" role="group">
        <button
          class="window-button"
          :aria-label="t('ui.topbar.minimize')"
          :title="t('ui.topbar.minimize')"
          @click="windowControl('minimize')"
        >
          <AppIcon name="minimize" :size="15" />
        </button>
        <button
          class="window-button"
          :aria-label="t(maximized ? 'ui.topbar.restore' : 'ui.topbar.maximize')"
          :title="t(maximized ? 'ui.topbar.restore' : 'ui.topbar.maximize')"
          @click="windowControl('maximize')"
        >
          <AppIcon :name="maximized ? 'restore' : 'maximize'" :size="14" />
        </button>
        <button
          class="window-button close"
          :aria-label="t('ui.topbar.close')"
          :title="t('ui.topbar.close')"
          @click="windowControl('close')"
        >
          <AppIcon name="close" :size="15" />
        </button>
      </div>
    </header>
    <main class="workspace-scroll">
      <div id="workspace" class="workspace">
        <div class="workspace-heading">
          <div>
            <h1>
              {{ t('ui.pins.title') }}<span class="count-badge">{{ pins.length }}</span>
            </h1>
          </div>
          <span class="workspace-status"
            ><i class="running-dot" :class="{ idle: !openCount }" />{{
              t('ui.pins.running', { count: openCount })
            }}</span
          >
        </div>
        <div v-if="error && !modalOpen" class="notice error-notice" role="alert">
          <AppIcon name="info" :size="18" /><span>{{ text(error) }}</span
          ><button
            class="icon-button"
            :aria-label="t('ui.notice.closeError')"
            @click="error = null"
          >
            <AppIcon name="close" :size="16" />
          </button>
        </div>
        <div
          v-for="(entry, index) in notices"
          :key="index"
          class="notice warning-notice"
          role="status"
        >
          <AppIcon name="info" :size="17" /><span>{{ text(entry) }}</span>
        </div>
        <section class="pins-section" :aria-label="t('ui.pins.title')">
          <div v-if="loading" class="empty-state panel" role="status">
            <AppIcon class="spinning" name="refresh" :size="28" />
            <h2>{{ t('ui.pins.loading') }}</h2>
          </div>
          <div v-else-if="!pins.length" class="empty-state panel">
            <div class="empty-icon">
              <AppIcon name="window" :size="30" /><span><AppIcon name="plus" :size="13" /></span>
            </div>
            <div>
              <h2>{{ t('ui.pins.emptyTitle') }}</h2>
              <div class="empty-actions">
                <button class="primary" :disabled="busy" @click="openCreate()">
                  <AppIcon name="link" :size="16" />{{ t('ui.workspace.addUrl') }}</button
                ><button class="secondary" :disabled="busy" @click="openCreate('import')">
                  <AppIcon name="folder" :size="16" />{{ t('ui.workspace.import') }}
                </button>
              </div>
            </div>
          </div>
          <div v-else class="pin-grid">
            <PinCard
              v-for="pin in pins"
              :key="pin.id"
              :pin="pin"
              :runtime="snapshot.runtime[pin.id]"
              :busy="busy"
              :working="!!pending?.startsWith(pin.id + ':')"
              @action="cardAction"
              @settings="editPin"
              @export="exportPin"
            />
          </div>
        </section>
        <ExampleGallery :busy="busy" :pending="pending" @create="createExample" />
        <footer class="workspace-footer">
          <button class="text-button" :disabled="busy || !desktopAvailable" @click="quit">
            <AppIcon name="power" :size="14" />{{ t('ui.sidebar.quit') }}
          </button>
        </footer>
      </div>
    </main>
    <div v-if="toast" class="toast notice success-notice" role="status">
      <AppIcon name="check" :size="18" /><span>{{ text(toast) }}</span
      ><button class="icon-button" :aria-label="t('ui.notice.closeToast')" @click="toast = null">
        <AppIcon name="close" :size="16" />
      </button>
    </div>
    <NewPinForm
      v-if="creatingOpen"
      :busy="busy"
      :error="error"
      :initial-mode="launchMode"
      @create="createAndClose"
      @error="error = $event"
      @close="creatingOpen = false"
    />
    <SettingsDialog
      v-if="settingsOpen"
      :settings="snapshot.settings"
      :busy="busy"
      :error="error"
      @close="settingsOpen = false"
      @save="saveSettings"
    />
    <PinSettingsDialog
      v-if="editingPin"
      :key="editingPin.id"
      :pin="editingPin"
      :busy="busy"
      :error="error"
      @close="editingPinId = null"
      @save="savePin"
    />
    <AppDialog
      v-if="deletingPin"
      :title="t('ui.card.deleteTitle')"
      :busy="busy"
      @close="deletingPin = null"
    >
      <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
      <p class="delete-message">{{ t('ui.card.deleteConfirm', { name: deletingPin.name }) }}</p>
      <p class="delete-description">
        {{
          t(
            deletingPin.source.kind === 'bundled'
              ? 'ui.card.deleteImportedHint'
              : 'ui.card.deleteHint',
          )
        }}
      </p>
      <template #footer>
        <button class="secondary" :disabled="busy" autofocus @click="deletingPin = null">
          {{ t('ui.common.cancel') }}</button
        ><button class="danger" :disabled="busy" @click="confirmDelete">
          {{ busy ? t('ui.common.working') : t('ui.card.deleteTitle') }}
        </button>
      </template>
    </AppDialog>
  </div>
</template>
