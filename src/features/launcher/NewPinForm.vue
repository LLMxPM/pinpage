<!-- 功能：收集网页地址或导入式本地网页，创建具有独立窗口设置的悬浮页。 -->
<script setup lang="ts">
import { ref } from 'vue';
import AppIcon from '../../components/AppIcon.vue';
import { desktopAvailable, importPage } from '../../services/desktop';
import { formatBytes } from '../../services/format';
import type { CreatePin, ImportPreview } from '../../types';

defineProps<{ busy: boolean }>();
const emit = defineEmits<{ create: [input: CreatePin]; error: [message: string] }>();
const mode = ref<'url' | 'import'>('url');
const address = ref('');
const name = ref('');
const preview = ref<ImportPreview | null>(null);
const entry = ref('');
const choosing = ref(false);
/// 名称仍由来源自动推导时，切换导入来源后跟随更新；用户改过就不再覆盖。
const nameAuto = ref(true);
const top = ref(true);
const borderless = ref(false);
const windowMode = ref<'floating' | 'desktop'>('floating');

/// 打开原生选择器扫描来源；内置示例直接返回预览，取消时保留上次结果。
async function choose(importMode: ImportPreview['mode']) {
  if (!desktopAvailable) {
    emit('error', '请选择桌面应用中的导入入口。');
    return;
  }
  choosing.value = true;
  try {
    const selected = await importPage(importMode);
    if (!selected) return;
    preview.value = selected;
    entry.value = selected.candidates[0] ?? '';
    // 用户未命名时按来源给出默认名称，改过名称后不再覆盖。
    if (nameAuto.value || !name.value) {
      name.value =
        selected.mode === 'directory'
          ? selected.label
          : selected.label.replace(/\.(html|htm)$/i, '');
      nameAuto.value = true;
    }
  } catch (error) {
    emit('error', String(error));
  } finally {
    choosing.value = false;
  }
}

/// 只提交网址或导入票据，原生层完成最终校验与文件复制。
function submit() {
  if (mode.value === 'import' && !preview.value) {
    emit('error', '请先导入一个目录或 HTML 文件。');
    return;
  }
  emit('create', {
    name: name.value,
    alwaysOnTop: top.value,
    decorations: !borderless.value,
    mode: windowMode.value,
    source:
      mode.value === 'url'
        ? { kind: 'url', address: address.value }
        : { kind: 'import', ticket: preview.value!.ticket, entry: entry.value },
  });
}
</script>

<template>
  <section class="new-pin panel" aria-labelledby="new-title">
    <div class="section-heading">
      <div class="heading-icon"><AppIcon name="plus" /></div>
      <div>
        <h2 id="new-title">固定一个新页面</h2>
        <p>一个网址，或导入一个本地网页。</p>
      </div>
    </div>
    <form @submit.prevent="submit">
      <div class="source-tabs" role="group" aria-label="页面来源">
        <button type="button" :class="{ active: mode === 'url' }" @click="mode = 'url'">
          <AppIcon name="link" :size="16" />网页地址
        </button>
        <button type="button" :class="{ active: mode === 'import' }" @click="mode = 'import'">
          <AppIcon name="folder" :size="16" />导入页面
        </button>
      </div>
      <div v-if="mode === 'url'" class="input-wrap">
        <AppIcon name="link" :size="18" /><input
          v-model="address"
          aria-label="网页地址"
          placeholder="https:// 或 localhost:3000"
          required
          autocomplete="url"
          spellcheck="false"
        />
      </div>
      <div v-else class="import-selection">
        <div class="import-actions">
          <button
            type="button"
            class="file-picker"
            :disabled="choosing || busy"
            @click="choose('directory')"
          >
            <AppIcon name="folder" /><span>导入目录</span><small>整个目录</small>
          </button>
          <button
            type="button"
            class="file-picker"
            :disabled="choosing || busy"
            @click="choose('file')"
          >
            <AppIcon name="window" /><span>导入单文件</span><small>仅入口</small>
          </button>
          <button
            type="button"
            class="file-picker"
            :disabled="choosing || busy"
            @click="choose('package')"
          >
            <AppIcon name="download" /><span>导入页面包</span><small>.pinpage</small>
          </button>
          <button
            type="button"
            class="file-picker"
            :disabled="choosing || busy"
            @click="choose('example')"
          >
            <AppIcon name="clock" /><span>导入示例</span><small>带数据</small>
          </button>
        </div>
        <template v-if="preview">
          <p class="import-summary">
            {{ preview.label }} · {{ preview.files }} 个文件 · {{ formatBytes(preview.bytes) }}
          </p>
          <label v-if="preview.candidates.length > 1" class="check-row import-entry">
            入口
            <select v-model="entry" aria-label="导入入口">
              <option v-for="item in preview.candidates" :key="item" :value="item">
                {{ item }}
              </option>
            </select>
          </label>
          <p v-else class="root-hint">入口：{{ entry }}</p>
        </template>
        <p v-else class="root-hint">
          导入目录会复制整个目录，单文件只复制入口；页面包可直接恢复站点与页面数据。
        </p>
      </div>
      <div class="creation-row">
        <input
          v-model="name"
          class="name-input"
          aria-label="页面名称"
          maxlength="80"
          placeholder="给页面起个名字（可选）"
          @input="nameAuto = false"
        /><button class="primary" :disabled="busy || choosing" type="submit">
          <AppIcon name="pin" :size="16" />{{ busy ? '正在处理…' : '创建悬浮窗' }}
        </button>
      </div>
      <div class="creation-options">
        <label class="check-row"
          >模式<select v-model="windowMode" aria-label="新页面显示模式">
            <option value="floating">浮动窗口</option>
            <option value="desktop">固定在桌面</option>
          </select></label
        >
        <label class="check-row"
          ><input
            v-model="top"
            type="checkbox"
            :disabled="windowMode === 'desktop'"
          />始终置顶</label
        ><label class="check-row"><input v-model="borderless" type="checkbox" />无边框模式</label
        ><span>每个页面独立运行</span>
      </div>
    </form>
  </section>
</template>
