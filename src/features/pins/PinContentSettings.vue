<!-- 功能：独立编辑页面名称和来源，保存成功后清空草稿，不重新应用窗口几何。 -->
<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import AppSelect from '../../components/AppSelect.vue';
import AppIcon from '../../components/AppIcon.vue';
import { usePinSourceDraft } from './usePinSourceDraft';
import { message } from '../../i18n/messages';
import type { PinConfig, PinContentInput, UiMessage } from '../../types';

const props = defineProps<{
  pin: PinConfig;
  formId: string;
  busy: boolean;
  save: (input: PinContentInput) => Promise<boolean | undefined>;
}>();
const emit = defineEmits<{
  error: [error: UiMessage];
  dirty: [value: boolean];
  pending: [value: boolean];
}>();
const { t } = useI18n();
const name = ref(props.pin.name);
const draft = reactive(
  usePinSourceDraft(() => props.pin, { onError: (error) => emit('error', error) }),
);
const dirty = computed(() => name.value.trim() !== props.pin.name || draft.dirty);
const isUrl = computed(() => props.pin.source.kind === 'url');
const currentEntry = computed(() =>
  props.pin.source.kind === 'bundled' ? props.pin.source.entry : '',
);
watch(dirty, (value) => emit('dirty', value), { immediate: true });
watch(
  () => draft.choosing,
  (value) => emit('pending', value),
);

/// 保存后复位已提交来源，失败时保留名称、地址与导入票据供重试。
async function save() {
  if (!dirty.value) return;
  if (isUrl.value && !draft.address.trim()) {
    emit('error', message('native.errors.invalidUrl'));
    return;
  }
  const source = draft.input;
  if (await props.save({ name: name.value, ...(source ? { source } : {}) })) reset();
}

/// 显式放弃内容草稿，不影响窗口即时设置。
function reset() {
  name.value = props.pin.name;
  draft.reset();
}
</script>

<template>
  <form :id="formId" class="pin-settings-form" @submit.prevent="save">
    <fieldset class="form-fields" :disabled="busy || draft.choosing">
      <label class="field-label"
        >{{ t('ui.pinSettings.name') }}<input v-model="name" required maxlength="80"
      /></label>
      <div class="form-section">
        <h3>{{ t('ui.pinSettings.source') }}</h3>
        <label v-if="isUrl" class="field-label"
          >{{ t('ui.pinSettings.sourceUrl')
          }}<input
            v-model="draft.address"
            :placeholder="t('ui.form.urlPlaceholder')"
            autocomplete="url"
            spellcheck="false"
            required
        /></label>
        <div v-else class="import-selection">
          <p class="source-current">
            <AppIcon name="window" :size="16" />{{
              t('ui.card.importedSource', { entry: currentEntry })
            }}
          </p>
          <div class="source-picks">
            <button type="button" class="source-pick" @click="draft.choose('directory')">
              <AppIcon name="folder" :size="16" />{{ t('ui.form.pickFolder') }}
            </button>
            <button type="button" class="source-pick" @click="draft.choose('file')">
              <AppIcon name="window" :size="16" />{{ t('ui.form.pickFile') }}
            </button>
            <button type="button" class="source-pick" @click="draft.choose('package')">
              <AppIcon name="download" :size="16" />{{ t('ui.form.pickPackage') }}
            </button>
          </div>
          <p v-if="draft.choosing" class="import-progress" role="status">
            {{ t('ui.common.working') }}
          </p>
          <div v-if="draft.preview" class="import-summary">
            <p><AppIcon name="check" :size="16" />{{ draft.preview.label }}</p>
            <AppSelect
              v-if="draft.preview.candidates.length > 1"
              v-model="draft.entry"
              :label="t('ui.form.entry')"
              :options="draft.entryOptions"
              :disabled="busy"
            />
          </div>
        </div>
        <p class="source-hint">
          {{ t(isUrl ? 'ui.pinSettings.sourceUrlHint' : 'ui.pinSettings.sourceImportHint') }}
        </p>
        <p v-if="draft.dirty" class="source-dirty">
          <AppIcon name="refresh" :size="15" />{{ t('ui.pinSettings.sourceDirty') }}
        </p>
      </div>
      <button v-if="dirty" type="button" class="text-button" @click="reset">
        {{ t('ui.pinSettings.discardContent') }}
      </button>
    </fieldset>
  </form>
</template>
