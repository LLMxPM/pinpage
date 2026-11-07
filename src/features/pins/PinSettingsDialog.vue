<!-- 功能：编辑页面来源、显示模式、窗口几何与独立锁定，失败时保留输入。 -->
<script setup lang="ts">
import { computed, reactive, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import AppDialog from '../../components/AppDialog.vue';
import AppSelect from '../../components/AppSelect.vue';
import AppIcon from '../../components/AppIcon.vue';
import { text } from '../../i18n/messages';
import { usePinSettingsForm } from './usePinSettingsForm';
import { usePinSourceDraft } from './usePinSourceDraft';
import type { PinConfig, PinSettingsInput, UiMessage } from '../../types';

const props = defineProps<{ pin: PinConfig; busy: boolean; error: UiMessage | null }>();
const emit = defineEmits<{
  close: [];
  save: [input: PinSettingsInput];
  error: [message: UiMessage];
}>();
const { t } = useI18n();
const formId = useId();
const input = usePinSettingsForm(() => props.pin);
// 包成响应式对象后，模板可以直接读写草稿里的各个响应式字段，无需逐个 .value。
const draft = reactive(
  usePinSourceDraft(() => props.pin, { onError: (reason) => emit('error', reason) }),
);
const isUrlSource = computed(() => props.pin.source.kind === 'url');
/// 当前来源的展示文本单独取值，避免模板里再做联合类型收窄。
const currentUrl = computed(() => (props.pin.source.kind === 'url' ? props.pin.source.url : ''));
const currentEntry = computed(() =>
  props.pin.source.kind === 'bundled' ? props.pin.source.entry : '',
);
const working = computed(() => props.busy || draft.choosing);
const borderless = computed(() => input.mode === 'desktop' || !input.decorations);
const showBorder = computed({
  get: () => input.mode === 'floating' && input.decorations,
  set: (value: boolean) => {
    input.decorations = value;
  },
});
const showOnTop = computed({
  get: () => input.mode === 'floating' && input.alwaysOnTop,
  set: (value: boolean) => {
    input.alwaysOnTop = value;
  },
});

/// 提交完整字段；来源未改动时不带 source，后端因此不会重新加载页面或复制副本。
function save() {
  const changed = draft.input;
  emit('save', changed ? { ...input, source: changed } : { ...input });
}
</script>

<template>
  <AppDialog
    :title="t('ui.pinSettings.title')"
    :description="pin.name"
    :busy="working"
    wide
    @close="emit('close')"
  >
    <form :id="formId" @submit.prevent="save">
      <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
      <fieldset class="form-fields" :disabled="working">
        <label class="field-label"
          >{{ t('ui.pinSettings.name')
          }}<input v-model="input.name" required maxlength="80" autofocus
        /></label>
        <div class="form-section">
          <h3>{{ t('ui.pinSettings.source') }}</h3>
          <label v-if="isUrlSource" class="field-label"
            >{{ t('ui.pinSettings.sourceUrl')
            }}<input
              v-model="draft.address"
              :placeholder="t('ui.form.urlPlaceholder')"
              :title="currentUrl"
              autocomplete="url"
              spellcheck="false"
          /></label>
          <div v-else class="import-selection">
            <p class="source-current">
              <AppIcon name="window" :size="16" />{{
                t('ui.card.importedSource', { entry: currentEntry })
              }}
            </p>
            <div class="import-actions">
              <button type="button" class="file-picker" @click="draft.choose('directory')">
                <AppIcon name="folder" /><strong>{{ t('ui.form.pickFolder') }}</strong>
              </button>
              <button type="button" class="file-picker" @click="draft.choose('file')">
                <AppIcon name="window" /><strong>{{ t('ui.form.pickFile') }}</strong>
              </button>
              <button type="button" class="file-picker" @click="draft.choose('package')">
                <AppIcon name="download" /><strong>{{ t('ui.form.pickPackage') }}</strong>
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
                :disabled="working"
              />
            </div>
          </div>
          <p class="source-hint">
            {{
              t(isUrlSource ? 'ui.pinSettings.sourceUrlHint' : 'ui.pinSettings.sourceImportHint')
            }}
          </p>
          <p v-if="draft.dirty" class="source-dirty">
            <AppIcon name="refresh" :size="15" />{{ t('ui.pinSettings.sourceDirty') }}
          </p>
        </div>
        <fieldset class="mode-picker">
          <legend>{{ t('ui.pinSettings.modeLegend') }}</legend>
          <label :class="{ active: input.mode === 'floating' }"
            ><input v-model="input.mode" type="radio" value="floating" /><span
              ><strong>{{ t('ui.pinSettings.floating') }}</strong></span
            ></label
          >
          <label :class="{ active: input.mode === 'desktop' }"
            ><input v-model="input.mode" type="radio" value="desktop" /><span
              ><strong>{{ t('ui.pinSettings.desktop') }}</strong></span
            ></label
          >
        </fieldset>
        <div class="pin-setting-checks">
          <label class="check-row"
            ><input v-model="showOnTop" type="checkbox" :disabled="input.mode === 'desktop'" />{{
              t('ui.pinSettings.top')
            }}</label
          >
          <label class="check-row"
            ><input v-model="showBorder" type="checkbox" :disabled="input.mode === 'desktop'" />{{
              t('ui.pinSettings.border')
            }}</label
          >
          <label class="check-row"
            ><input v-model="input.rounded" type="checkbox" :disabled="!borderless" />{{
              t('ui.pinSettings.rounded')
            }}</label
          >
        </div>
        <div class="form-section">
          <h3>{{ t('ui.pinSettings.geometry') }}</h3>
          <div class="geometry-fields">
            <label class="field-label"
              >{{ t('ui.pinSettings.width')
              }}<input v-model.number="input.width" type="number" min="160" max="3840" required
            /></label>
            <label class="field-label"
              >{{ t('ui.pinSettings.height')
              }}<input v-model.number="input.height" type="number" min="100" max="2160" required
            /></label>
            <label class="field-label"
              >{{ t('ui.pinSettings.positionX')
              }}<input v-model.number="input.x" type="number" required
            /></label>
            <label class="field-label"
              >{{ t('ui.pinSettings.positionY')
              }}<input v-model.number="input.y" type="number" required
            /></label>
          </div>
        </div>
        <div class="lock-settings">
          <label class="setting-toggle"
            ><span
              ><strong>{{ t('ui.pinSettings.lockPosition') }}</strong></span
            ><input v-model="input.positionLocked" class="switch" type="checkbox" role="switch"
          /></label>
          <label class="setting-toggle"
            ><span
              ><strong>{{ t('ui.pinSettings.lockSize') }}</strong></span
            ><input v-model="input.sizeLocked" class="switch" type="checkbox" role="switch"
          /></label>
        </div>
      </fieldset>
    </form>
    <template #footer>
      <button type="button" class="secondary" :disabled="working" @click="emit('close')">
        {{ t('ui.common.cancel') }}</button
      ><button class="primary" :disabled="working" type="submit" :form="formId">
        {{ working ? t('ui.pinSettings.busy') : t('ui.pinSettings.save') }}
      </button>
    </template>
  </AppDialog>
</template>
