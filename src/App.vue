<!-- 功能：构建 PinPage 工作台，协调新建页面、窗口卡片与设置。 -->
<script setup lang="ts">
import AppIcon from './components/AppIcon.vue';
import SettingsDialog from './components/SettingsDialog.vue';
import NewPinForm from './features/launcher/NewPinForm.vue';
import PinCard from './features/pins/PinCard.vue';
import PinSettingsDialog from './features/pins/PinSettingsDialog.vue';
import { useWorkspace } from './features/pins/useWorkspace';
import { desktopAvailable } from './services/desktop';

const {
  snapshot,
  pins,
  busy,
  loading,
  error,
  settingsOpen,
  editingPinId,
  editingPin,
  savePin,
  openCount,
  topCount,
  create,
  action,
  saveSettings,
  quit,
} = useWorkspace();

/// 创建内嵌计时样例，用于快速体验和验证模块及 JSON 资源加载。
function createExample() {
  void create({
    name: '专注计时器',
    source: { kind: 'example' },
    alwaysOnTop: true,
    decorations: true,
  });
}
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <a class="brand" href="#" aria-label="PinPage 工作台"
        ><span class="brand-mark"><AppIcon name="pin" :size="23" /></span
        ><span>PinPage<span class="brand-dot">.</span></span></a
      >
      <div class="sidebar-section">我的空间</div>
      <a href="#workspace" class="nav-item active"
        ><AppIcon name="window" :size="19" />悬浮工作台<span>{{ pins.length }}</span></a
      ><button class="nav-item" @click="settingsOpen = true">
        <AppIcon name="settings" :size="19" />偏好设置
      </button>
      <div class="sidebar-bottom">
        <div class="sidebar-tip">
          <AppIcon name="pin" :size="18" />
          <p>重要的事，<br />始终留在眼前。</p>
          <small>计时器 / 看板 / 小工具</small>
        </div>
        <div class="version"><span class="connection-dot"></span>Windows · v0.1.0</div>
        <button class="quit-button" :disabled="busy || !desktopAvailable" @click="quit">
          <AppIcon name="power" :size="14" />退出并保存会话
        </button>
      </div>
    </aside>
    <main id="workspace" class="workspace">
      <header class="topbar">
        <span>工作空间 <span class="breadcrumb-separator">/</span> 悬浮工作台</span
        ><button
          class="icon-button"
          aria-label="打开设置"
          title="打开设置"
          @click="settingsOpen = true"
        >
          <AppIcon name="settings" :size="19" />
        </button>
      </header>
      <div class="workspace-content">
        <section class="intro">
          <div>
            <div class="eyebrow"><span></span>YOUR LITTLE DESKTOP COMPANIONS</div>
            <h1>让重要的页面，留在眼前<span>。</span></h1>
            <p>把计时器、看板和小工具，变成随手可用的桌面窗口。</p>
          </div>
          <span class="desktop-badge"
            ><AppIcon name="window" :size="15" />{{
              desktopAvailable ? '桌面已连接' : '界面预览'
            }}</span
          >
        </section>
        <div v-if="!desktopAvailable" class="notice">
          <AppIcon name="info" :size="17" /><span
            >当前为浏览器预览。运行 <code>pnpm tauri dev</code> 后即可创建原生悬浮窗。</span
          >
        </div>
        <div v-if="error" class="notice error-notice" role="alert">
          <AppIcon name="info" :size="18" /><span>{{ error }}</span
          ><button class="icon-button" aria-label="关闭错误提示" @click="error = ''">
            <AppIcon name="close" :size="16" />
          </button>
        </div>
        <div
          v-for="notice in snapshot.notices"
          :key="notice"
          class="notice warning-notice"
          role="status"
        >
          <AppIcon name="info" :size="17" /><span>{{ notice }}</span>
        </div>
        <NewPinForm :busy="busy" @create="create" @error="error = $event" />
        <section class="pins-section" aria-labelledby="pins-title">
          <div class="pins-heading">
            <div>
              <h2 id="pins-title">
                我的悬浮页 <span>{{ pins.length.toString().padStart(2, '0') }}</span>
              </h2>
              <p>各有各的位置，随时回到你的视线。</p>
            </div>
            <div class="window-stats">
              <span><i class="running-dot"></i>{{ openCount }} 个运行中</span
              ><span><AppIcon name="pin" :size="13" />{{ topCount }} 个已置顶</span>
            </div>
          </div>
          <div v-if="loading" class="empty-state panel" role="status">
            <AppIcon name="refresh" :size="28" />
            <h3>正在连接桌面窗口…</h3>
          </div>
          <div v-else-if="!pins.length" class="empty-state panel">
            <div class="empty-illustration">
              <div class="mini-window mini-back"></div>
              <div class="mini-window mini-front">
                <span></span><AppIcon name="pin" :size="30" />
              </div>
            </div>
            <h3>你的第一个悬浮页，从这里开始</h3>
            <p>在上方添加网址或本地文件，<br />也可以先打开一个专注计时器试试。</p>
            <button class="secondary" :disabled="busy" @click="createExample">
              <AppIcon name="clock" :size="16" />试试计时器<AppIcon name="arrow" :size="15" />
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
            /><button class="example-card" :disabled="busy" @click="createExample">
              <AppIcon name="plus" :size="23" /><strong>添加专注计时器</strong
              ><span>一个开箱即用的小示例</span>
            </button>
          </div>
        </section>
        <footer class="workspace-footer">
          <span
            ><AppIcon name="info" :size="14" />无边框窗口可用 Alt + 左键拖动；托盘可恢复边框。</span
          ><span>少一点切换，多一点专注。</span>
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
