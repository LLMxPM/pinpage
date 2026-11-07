<!-- 功能：收集网页或本地 HTML 来源，创建具有独立窗口设置的悬浮页。 -->
<script setup lang="ts">
import { ref } from 'vue';
import AppIcon from '../../components/AppIcon.vue';
import { desktopAvailable, pickSource } from '../../services/desktop';
import { displayPath } from '../../services/format';
import type { CreatePin, LocalChoice } from '../../types';

defineProps<{ busy: boolean }>();
const emit = defineEmits<{ create: [input: CreatePin]; error: [message: string] }>();
const mode = ref<'url' | 'local'>('url');
const address = ref('');
const name = ref('');
const local = ref<LocalChoice | null>(null);
const choosing = ref(false);
const customRoot = ref(false);
const top = ref(true);
const borderless = ref(false);
const windowMode = ref<'floating' | 'desktop'>('floating');

/// 原生选择器返回授权票据；取消时保留原选择，错误显示在工作台。
async function choose() {
  if (!desktopAvailable) {
    emit('error', '请选择桌面应用中的本地文件入口。');
    return;
  }
  choosing.value = true;
  try {
    const selected = await pickSource(customRoot.value);
    if (selected) local.value = selected;
  } catch (error) {
    emit('error', String(error));
  } finally {
    choosing.value = false;
  }
}

/// 只提交来源票据或网址，原生层完成最终解析和权限校验。
function submit() {
  if (mode.value === 'local' && !local.value) {
    emit('error', '请先选择一个 HTML 文件。');
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
        : { kind: 'local', ticket: local.value!.ticket },
  });
}
</script>

<template>
  <section class="new-pin panel" aria-labelledby="new-title">
    <div class="section-heading">
      <div class="heading-icon"><AppIcon name="plus" /></div>
      <div>
        <h2 id="new-title">固定一个新页面</h2>
        <p>一个网址，或一个本地 HTML 文件。</p>
      </div>
    </div>
    <form @submit.prevent="submit">
      <div class="source-tabs" role="group" aria-label="页面来源">
        <button type="button" :class="{ active: mode === 'url' }" @click="mode = 'url'">
          <AppIcon name="link" :size="16" />网页地址
        </button>
        <button type="button" :class="{ active: mode === 'local' }" @click="mode = 'local'">
          <AppIcon name="folder" :size="16" />本地文件
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
      <div v-else class="file-selection">
        <button type="button" class="file-picker" :disabled="choosing || busy" @click="choose">
          <AppIcon name="folder" /><span>{{
            local ? displayPath(local.entry) : '选择 HTML 文件'
          }}</span
          ><small>{{ choosing ? '选择中…' : '浏览文件' }}</small>
        </button>
        <label class="check-row subtle"
          ><input v-model="customRoot" type="checkbox" />选择时另外指定资源根目录</label
        >
        <p v-if="local" class="root-hint" :title="displayPath(local.root)">
          资源目录：{{ displayPath(local.root) }}
        </p>
      </div>
      <div class="creation-row">
        <input
          v-model="name"
          class="name-input"
          aria-label="页面名称"
          maxlength="80"
          placeholder="给页面起个名字（可选）"
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
