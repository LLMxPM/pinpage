<!-- 功能：统一管理内置与自定义预设，支持副本、编辑、重命名和删除确认。 -->
<script setup lang="ts">
import { ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import AppDialog from '../../components/AppDialog.vue';
import PresetEditor from './PresetEditor.vue';
import { presetLabel } from './presets';
import { text } from '../../i18n/messages';
import type { PresetInput, UiMessage, WindowPreset } from '../../types';

const props = defineProps<{
  presets: WindowPreset[];
  busy: boolean;
  error: UiMessage | null;
  write: (input: PresetInput, presetId?: string) => Promise<boolean>;
  remove: (id: string) => Promise<boolean>;
}>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
const formId = useId();
const editing = ref<WindowPreset | null>(null);
const copying = ref(false);
const deleting = ref<WindowPreset | null>(null);
const dirty = ref(false);
const discarding = ref(false);
const closing = ref(false);

/// 编辑与复制均建立独立草稿，已有模板只有明确保存后才变化。
function edit(preset: WindowPreset, copy = false) {
  editing.value = preset;
  copying.value = copy;
  dirty.value = false;
}

/// 保存失败保持草稿和当前编辑页，成功后返回列表。
async function save(input: PresetInput) {
  if (await props.write(input, copying.value ? undefined : editing.value?.id)) {
    editing.value = null;
    dirty.value = false;
  }
}

/// 删除成功才收起确认区，失败原因继续留在当前弹窗中。
async function remove() {
  if (deleting.value && (await props.remove(deleting.value.id))) deleting.value = null;
}

/// 管理器内返回与关闭共用丢弃确认，不丢失尚未保存的模板参数。
function back() {
  closing.value = false;
  if (props.busy) return;
  if (editing.value) {
    if (dirty.value) discarding.value = true;
    else editing.value = null;
  } else if (deleting.value) deleting.value = null;
  else emit('close');
}
/// 标题栏关闭直接结束管理器，未保存模板则先显示确认。
function close() {
  if (props.busy) return;
  if (editing.value && dirty.value) {
    closing.value = true;
    discarding.value = true;
  } else emit('close');
}

/// 确认放弃后按原入口返回列表或关闭管理器。
function discard() {
  editing.value = null;
  dirty.value = false;
  discarding.value = false;
  if (closing.value) emit('close');
}
</script>

<template>
  <AppDialog
    :title="t(editing ? 'ui.presets.edit' : 'ui.presets.manage')"
    :busy="busy"
    wide
    @close="close"
  >
    <div v-if="error" class="notice error-notice" role="alert">{{ text(error) }}</div>
    <div v-if="discarding">
      <p>{{ t(closing ? 'ui.presets.discardCloseConfirm' : 'ui.presets.discardConfirm') }}</p>
    </div>
    <div v-else-if="deleting">
      <p class="delete-message">
        {{ t('ui.presets.deleteConfirm', { name: presetLabel(deleting) }) }}
      </p>
      <p class="settings-hint">{{ t('ui.presets.deleteHint') }}</p>
    </div>
    <PresetEditor
      v-if="editing"
      v-show="!discarding"
      :key="`${editing.id}:${copying}`"
      :preset="editing"
      :copy="copying"
      :form-id="formId"
      :busy="busy"
      @save="save"
      @dirty="dirty = $event"
    />
    <div v-if="!editing && !deleting && !discarding">
      <p class="settings-hint">{{ t('ui.presets.snapshotHint') }}</p>
      <ul class="preset-library">
        <li v-for="preset in presets" :key="preset.id">
          <div class="preset-summary">
            <strong>{{ presetLabel(preset) }}</strong
            ><span v-if="preset.builtin" class="preset-badge">{{ t('ui.presets.builtin') }}</span>
            <p>
              {{
                t(preset.mode === 'desktop' ? 'ui.pinSettings.desktop' : 'ui.pinSettings.floating')
              }}
              ·
              {{
                preset.fullscreen
                  ? t('ui.presets.fullscreen')
                  : `${preset.width} × ${preset.height}`
              }}
            </p>
          </div>
          <div class="inline-actions">
            <button type="button" class="text-button" :disabled="busy" @click="edit(preset, true)">
              {{ t('ui.presets.copy') }}
            </button>
            <button
              v-if="!preset.builtin"
              type="button"
              class="text-button"
              :disabled="busy"
              @click="edit(preset)"
            >
              {{ t('ui.presets.edit') }}
            </button>
            <button
              v-if="!preset.builtin"
              type="button"
              class="text-button danger-text"
              :disabled="busy"
              @click="deleting = preset"
            >
              {{ t('ui.presets.delete') }}
            </button>
          </div>
        </li>
      </ul>
      <p v-if="!presets.some((preset) => !preset.builtin)" class="settings-hint">
        {{ t('ui.settings.presetEmpty') }}
      </p>
    </div>
    <template #footer>
      <template v-if="discarding">
        <button type="button" class="secondary" @click="discarding = false">
          {{ t('ui.pinSettings.continueEditing') }}
        </button>
        <button type="button" class="primary" @click="discard">
          {{ t('ui.presets.discard') }}
        </button>
      </template>
      <template v-else>
        <button type="button" class="secondary" :disabled="busy" @click="back">
          {{ t(editing || deleting ? 'ui.presets.back' : 'ui.common.close') }}
        </button>
        <button v-if="editing" type="submit" class="primary" :form="formId" :disabled="busy">
          {{ t('ui.presets.save') }}
        </button>
        <button
          v-if="deleting"
          type="button"
          class="primary danger-button"
          :disabled="busy"
          @click="remove"
        >
          {{ t('ui.presets.delete') }}
        </button>
      </template>
    </template>
  </AppDialog>
</template>
