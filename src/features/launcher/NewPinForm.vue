<!-- 功能：在统一弹窗中添加网址或导入页面，按需展开窗口选项，保留失败时的输入。 -->
<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import AppDialog from '../../components/AppDialog.vue';
import AppSelect from '../../components/AppSelect.vue';
import AppIcon from '../../components/AppIcon.vue';
import { message, text, toUiMessage } from '../../i18n/messages';
import { desktopAvailable, importPage } from '../../services/desktop';
import type { CreatePin, ImportPreview, UiMessage } from '../../types';

const props = defineProps<{
  busy: boolean;
  error: UiMessage | null;
  initialMode: 'url' | 'import';
}>();
const emit = defineEmits<{ create: [input: CreatePin]; error: [message: UiMessage]; close: [] }>();
const { t } = useI18n();
const formId = useId();
const mode = ref(props.initialMode);
const address = ref('');
const name = ref('');
const preview = ref<ImportPreview | null>(null);
const entry = ref('');
const choosing = ref(false);
const nameAuto = ref(true);
const top = ref(true);
const borderless = ref(false);
const windowMode = ref<'floating' | 'desktop'>('floating');
const modeOptions = computed(
  () =>
    [
      { value: 'floating', label: t('ui.form.modeFloating') },
      { value: 'desktop', label: t('ui.form.modeDesktop') },
    ] as const,
);
const entryOptions = computed(
  () => preview.value?.candidates.map((value) => ({ value, label: value })) ?? [],
);
const displayTop = computed({
  get: () => windowMode.value === 'floating' && top.value,
  set: (value: boolean) => {
    top.value = value;
  },
});
const displayBorderless = computed({
  get: () => windowMode.value === 'desktop' || borderless.value,
  set: (value: boolean) => {
    borderless.value = value;
  },
});

/// 原生选择器取消时保留上次结果，用户主动命名后不再覆盖名称。
async function choose(importMode: 'directory' | 'file' | 'package') {
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
    if (nameAuto.value || !name.value) {
      name.value =
        selected.mode === 'directory'
          ? selected.label
          : selected.label.replace(/\.(html|htm)$/i, '');
      nameAuto.value = true;
    }
  } catch (reason) {
    emit('error', toUiMessage(reason));
  } finally {
    choosing.value = false;
  }
}

/// 仅提交来源票据和外观偏好，路径、网址与窗口权限由原生层最终校验。
function submit() {
  if (props.busy || choosing.value) return;
  if (mode.value === 'import' && !preview.value) return;
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
  <AppDialog :title="t('ui.form.title')" :busy="busy || choosing" wide @close="emit('close')">
    <form :id="formId" @submit.prevent="submit">
      <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
      <fieldset class="form-fields" :disabled="busy || choosing">
        <div class="source-tabs" role="group" :aria-label="t('ui.form.sourceGroup')">
          <button
            type="button"
            :class="{ active: mode === 'url' }"
            :aria-pressed="mode === 'url'"
            @click="mode = 'url'"
          >
            <AppIcon name="link" :size="17" />{{ t('ui.form.tabUrl') }}
          </button>
          <button
            type="button"
            :class="{ active: mode === 'import' }"
            :aria-pressed="mode === 'import'"
            @click="mode = 'import'"
          >
            <AppIcon name="folder" :size="17" />{{ t('ui.form.tabImport') }}
          </button>
        </div>
        <label v-if="mode === 'url'" class="field-label"
          >{{ t('ui.form.urlAria')
          }}<input
            v-model="address"
            :placeholder="t('ui.form.urlPlaceholder')"
            required
            autocomplete="url"
            spellcheck="false"
            autofocus
        /></label>
        <div v-else class="import-selection">
          <div class="import-actions">
            <button type="button" class="file-picker" @click="choose('directory')">
              <AppIcon name="folder" /><strong>{{ t('ui.form.pickFolder') }}</strong>
            </button>
            <button type="button" class="file-picker" @click="choose('file')">
              <AppIcon name="window" /><strong>{{ t('ui.form.pickFile') }}</strong>
            </button>
            <button type="button" class="file-picker" @click="choose('package')">
              <AppIcon name="download" /><strong>{{ t('ui.form.pickPackage') }}</strong>
            </button>
          </div>
          <p v-if="choosing" class="import-progress" role="status">{{ t('ui.common.working') }}</p>
          <div v-if="preview" class="import-summary">
            <p><AppIcon name="check" :size="16" />{{ preview.label }}</p>
            <AppSelect
              v-if="preview.candidates.length > 1"
              v-model="entry"
              :label="t('ui.form.entry')"
              :options="entryOptions"
              :disabled="busy || choosing"
            />
          </div>
        </div>
        <label class="field-label"
          >{{ t('ui.form.nameAria') }}<span class="optional">{{ t('ui.form.optional') }}</span
          ><input
            v-model="name"
            maxlength="80"
            :placeholder="t('ui.form.namePlaceholder')"
            @input="nameAuto = false"
        /></label>
        <details class="advanced-options">
          <summary>{{ t('ui.form.windowOptions') }}<AppIcon name="chevron" :size="16" /></summary>
          <AppSelect
            v-model="windowMode"
            :label="t('ui.form.mode')"
            :options="modeOptions"
            :disabled="busy || choosing"
          />
          <div class="creation-options">
            <label class="check-row"
              ><input v-model="displayTop" type="checkbox" :disabled="windowMode === 'desktop'" />{{
                t('ui.form.top')
              }}</label
            ><label class="check-row"
              ><input
                v-model="displayBorderless"
                type="checkbox"
                :disabled="windowMode === 'desktop'"
              />{{ t('ui.form.borderless') }}</label
            >
          </div>
        </details>
      </fieldset>
    </form>
    <template #footer>
      <button type="button" class="secondary" :disabled="busy || choosing" @click="emit('close')">
        {{ t('ui.common.cancel') }}</button
      ><button
        class="primary"
        :disabled="busy || choosing || (mode === 'import' && !preview)"
        type="submit"
        :form="formId"
      >
        <AppIcon name="plus" :size="17" />{{ busy ? t('ui.form.busy') : t('ui.form.submit') }}
      </button>
    </template>
  </AppDialog>
</template>
