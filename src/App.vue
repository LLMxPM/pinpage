<!-- 功能：构建 PinPage 工作台，协调语言呈现、新建页面、窗口卡片与设置。 -->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import AppIcon from './components/AppIcon.vue';
import SettingsDialog from './components/SettingsDialog.vue';
import NewPinForm from './features/launcher/NewPinForm.vue';
import PinCard from './features/pins/PinCard.vue';
import PinSettingsDialog from './features/pins/PinSettingsDialog.vue';
import { useWorkspace } from './features/pins/useWorkspace';
import { text } from './i18n/messages';
import { desktopAvailable } from './services/desktop';

const { t } = useI18n();
const {
  snapshot,
  pins,
  busy,
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
  topCount,
  create,
  createExample,
  exportPin,
  action,
  saveSettings,
  quit,
} = useWorkspace();
</script>

<template>
  <div v-if="!ready" class="boot-shell" :aria-label="t('ui.boot.ariaLabel')" role="status">
    <AppIcon name="pin" :size="30" /><span>PinPage</span>
  </div>
  <div v-else class="app-shell">
    <aside class="sidebar">
      <a class="brand" href="#" :aria-label="t('ui.brand.ariaLabel')"
        ><span class="brand-mark"><AppIcon name="pin" :size="23" /></span
        ><span>PinPage<span class="brand-dot">.</span></span></a
      >
      <div class="sidebar-section">{{ t('ui.sidebar.space') }}</div>
      <a href="#workspace" class="nav-item active"
        ><AppIcon name="window" :size="19" />{{ t('ui.sidebar.workspace')
        }}<span>{{ pins.length }}</span></a
      ><button class="nav-item" @click="settingsOpen = true">
        <AppIcon name="settings" :size="19" />{{ t('ui.sidebar.settings') }}
      </button>
      <div class="sidebar-bottom">
        <div class="sidebar-tip">
          <AppIcon name="pin" :size="18" />
          <p>{{ t('ui.sidebar.tipTop') }}<br />{{ t('ui.sidebar.tipBottom') }}</p>
          <small>{{ t('ui.sidebar.tipHint') }}</small>
        </div>
        <div class="version"><span class="connection-dot"></span>Windows · v0.1.0</div>
        <button class="quit-button" :disabled="busy || !desktopAvailable" @click="quit">
          <AppIcon name="power" :size="14" />{{ t('ui.sidebar.quit') }}
        </button>
      </div>
    </aside>
    <main id="workspace" class="workspace">
      <header class="topbar">
        <span
          >{{ t('ui.topbar.workspace') }} <span class="breadcrumb-separator">/</span>
          {{ t('ui.sidebar.workspace') }}</span
        ><button
          class="icon-button"
          :aria-label="t('ui.topbar.openSettings')"
          :title="t('ui.topbar.openSettings')"
          @click="settingsOpen = true"
        >
          <AppIcon name="settings" :size="19" />
        </button>
      </header>
      <div class="workspace-content">
        <section class="intro">
          <div>
            <div class="eyebrow"><span></span>{{ t('ui.hero.tagline') }}</div>
            <h1>
              {{ t('ui.hero.title') }}<span>{{ t('ui.hero.mark') }}</span>
            </h1>
            <p>{{ t('ui.hero.subtitle') }}</p>
          </div>
          <span class="desktop-badge"
            ><AppIcon name="window" :size="15" />{{
              desktopAvailable ? t('ui.desktop.connected') : t('ui.desktop.preview')
            }}</span
          >
        </section>
        <div v-if="!desktopAvailable" class="notice">
          <AppIcon name="info" :size="17" /><span
            >{{ t('ui.desktop.previewLead') }} <code>{{ t('ui.desktop.previewCommand') }}</code>
            {{ t('ui.desktop.previewTail') }}</span
          >
        </div>
        <div v-if="error" class="notice error-notice" role="alert">
          <AppIcon name="info" :size="18" /><span>{{ text(error) }}</span
          ><button
            class="icon-button"
            :aria-label="t('ui.notice.closeError')"
            @click="error = null"
          >
            <AppIcon name="close" :size="16" />
          </button>
        </div>
        <div v-if="toast" class="notice success-notice" role="status">
          <AppIcon name="check" :size="18" /><span>{{ text(toast) }}</span
          ><button
            class="icon-button"
            :aria-label="t('ui.notice.closeToast')"
            @click="toast = null"
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
        <NewPinForm :busy="busy" @create="create" @error="error = $event" />
        <section class="pins-section" aria-labelledby="pins-title">
          <div class="pins-heading">
            <div>
              <h2 id="pins-title">
                {{ t('ui.pins.title') }} <span>{{ pins.length.toString().padStart(2, '0') }}</span>
              </h2>
              <p>{{ t('ui.pins.subtitle') }}</p>
            </div>
            <div class="window-stats">
              <span
                ><i class="running-dot"></i>{{ t('ui.pins.running', { count: openCount }) }}</span
              ><span
                ><AppIcon name="pin" :size="13" />{{
                  t('ui.pins.pinned', { count: topCount })
                }}</span
              >
            </div>
          </div>
          <div v-if="loading" class="empty-state panel" role="status">
            <AppIcon name="refresh" :size="28" />
            <h3>{{ t('ui.pins.loading') }}</h3>
          </div>
          <div v-else-if="!pins.length" class="empty-state panel">
            <div class="empty-illustration">
              <div class="mini-window mini-back"></div>
              <div class="mini-window mini-front">
                <span></span><AppIcon name="pin" :size="30" />
              </div>
            </div>
            <h3>{{ t('ui.pins.emptyTitle') }}</h3>
            <p>{{ t('ui.pins.emptyLead') }}<br />{{ t('ui.pins.emptyTail') }}</p>
            <button class="secondary" :disabled="busy" @click="createExample">
              <AppIcon name="clock" :size="16" />{{ t('ui.pins.tryTimer')
              }}<AppIcon name="arrow" :size="15" />
            </button>
          </div>
          <div v-else class="pin-grid">
            <PinCard
              v-for="pin in pins"
              :key="pin.id"
              :pin="pin"
              :runtime="snapshot.runtime[pin.id]"
              :busy="busy"
              @action="action"
              @settings="editingPinId = $event"
              @export="exportPin"
            /><button class="example-card" :disabled="busy" @click="createExample">
              <AppIcon name="plus" :size="23" /><strong>{{ t('ui.pins.exampleTitle') }}</strong
              ><span>{{ t('ui.pins.exampleHint') }}</span>
            </button>
          </div>
        </section>
        <footer class="workspace-footer">
          <span><AppIcon name="info" :size="14" />{{ t('ui.footer.dragHint') }}</span
          ><span>{{ t('ui.footer.motto') }}</span>
        </footer>
      </div>
    </main>
    <SettingsDialog
      v-if="settingsOpen"
      :settings="snapshot.settings"
      :busy="busy"
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
  </div>
</template>
