<!-- 功能：收集网页地址或导入式本地网页，创建具有独立窗口设置的悬浮页。 -->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { ref } from 'vue';
import AppIcon from '../../components/AppIcon.vue';
import { message, toUiMessage } from '../../i18n/messages';
import { desktopAvailable, importPage } from '../../services/desktop';
import { formatBytes } from '../../services/format';
import type { CreatePin, ImportPreview, UiMessage } from '../../types';

defineProps<{ busy: boolean }>();
const emit = defineEmits<{ create: [input: CreatePin]; error: [message: UiMessage] }>();
const { t } = useI18n();
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
    emit('error', message('ui.form.needDesktop'));
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
    emit('error', toUiMessage(error));
  } finally {
    choosing.value = false;
  }
}

/// 只提交网址或导入票据，原生层完成最终校验与文件复制。
function submit() {
  if (mode.value === 'import' && !preview.value) {
    emit('error', message('ui.form.needImport'));
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
        <h2 id="new-title">{{ t('ui.form.title') }}</h2>
        <p>{{ t('ui.form.subtitle') }}</p>
      </div>
    </div>
    <form @submit.prevent="submit">
      <div class="source-tabs" role="group" :aria-label="t('ui.form.sourceGroup')">
        <button type="button" :class="{ active: mode === 'url' }" @click="mode = 'url'">
          <AppIcon name="link" :size="16" />{{ t('ui.form.tabUrl') }}
        </button>
        <button type="button" :class="{ active: mode === 'import' }" @click="mode = 'import'">
          <AppIcon name="folder" :size="16" />{{ t('ui.form.tabImport') }}
        </button>
      </div>
      <div v-if="mode === 'url'" class="input-wrap">
        <AppIcon name="link" :size="18" /><input
          v-model="address"
          :aria-label="t('ui.form.urlAria')"
          :placeholder="t('ui.form.urlPlaceholder')"
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
            <AppIcon name="folder" /><span>{{ t('ui.form.pickFolder') }}</span
            ><small>{{ t('ui.form.pickFolderHint') }}</small>
          </button>
          <button
            type="button"
            class="file-picker"
            :disabled="choosing || busy"
            @click="choose('file')"
          >
            <AppIcon name="window" /><span>{{ t('ui.form.pickFile') }}</span
            ><small>{{ t('ui.form.pickFileHint') }}</small>
          </button>
          <button
            type="button"
            class="file-picker"
            :disabled="choosing || busy"
            @click="choose('package')"
          >
            <AppIcon name="download" /><span>{{ t('ui.form.pickPackage') }}</span
            ><small>{{ t('ui.form.pickPackageHint') }}</small>
          </button>
          <button
            type="button"
            class="file-picker"
            :disabled="choosing || busy"
            @click="choose('example')"
          >
            <AppIcon name="clock" /><span>{{ t('ui.form.pickExample') }}</span
            ><small>{{ t('ui.form.pickExampleHint') }}</small>
          </button>
        </div>
        <template v-if="preview">
          <p class="import-summary">
            {{
              t('ui.form.importedSummary', {
                label: preview.label,
                count: preview.files,
                bytes: formatBytes(preview.bytes),
              })
            }}
          </p>
          <label v-if="preview.candidates.length > 1" class="check-row import-entry">
            {{ t('ui.form.entry') }}
            <select v-model="entry" :aria-label="t('ui.form.entryAria')">
              <option v-for="item in preview.candidates" :key="item" :value="item">
                {{ item }}
              </option>
            </select>
          </label>
          <p v-else class="root-hint">{{ t('ui.form.entryValue', { entry }) }}</p>
        </template>
        <p v-else class="root-hint">{{ t('ui.form.importHint') }}</p>
      </div>
      <div class="creation-row">
        <input
          v-model="name"
          class="name-input"
          :aria-label="t('ui.form.nameAria')"
          maxlength="80"
          :placeholder="t('ui.form.namePlaceholder')"
          @input="nameAuto = false"
        /><button class="primary" :disabled="busy || choosing" type="submit">
          <AppIcon name="pin" :size="16" />{{ busy ? t('ui.form.busy') : t('ui.form.submit') }}
        </button>
      </div>
      <div class="creation-options">
        <label class="check-row"
          >{{ t('ui.form.mode')
          }}<select v-model="windowMode" :aria-label="t('ui.form.modeAria')">
            <option value="floating">{{ t('ui.form.modeFloating') }}</option>
            <option value="desktop">{{ t('ui.form.modeDesktop') }}</option>
          </select></label
        >
        <label class="check-row"
          ><input v-model="top" type="checkbox" :disabled="windowMode === 'desktop'" />{{
            t('ui.form.top')
          }}</label
        ><label class="check-row"
          ><input v-model="borderless" type="checkbox" />{{ t('ui.form.borderless') }}</label
        ><span>{{ t('ui.form.footnote') }}</span>
      </div>
    </form>
  </section>
</template>
