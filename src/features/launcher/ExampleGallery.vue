<!-- 功能：以统一的示例卡片展示四种本地工具，并反馈具体示例的创建状态。 -->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import AppIcon from '../../components/AppIcon.vue';
import { examples, type ExampleId } from './examples';

defineProps<{ busy: boolean; pending: string | null }>();
const emit = defineEmits<{ create: [id: ExampleId] }>();
const { t } = useI18n();
</script>

<template>
  <section class="examples-section" aria-labelledby="examples-title">
    <div class="section-heading">
      <div>
        <h2 id="examples-title">{{ t('ui.examples.title') }}</h2>
      </div>
    </div>
    <div class="example-grid">
      <article v-for="example in examples" :key="example.id" class="example-tile panel">
        <div class="example-preview" :class="example.id" aria-hidden="true">
          <template v-if="example.id === 'timer'"
            ><strong>25<span>:</span>00</strong>
            <div class="preview-progress"><i /></div
          ></template>
          <template v-else-if="example.id === 'notes'"
            ><AppIcon name="note" :size="18" />
            <div class="preview-lines"><i /><i /><i /></div
          ></template>
          <template v-else-if="example.id === 'checklist'"
            ><span
              ><i class="checked"><AppIcon name="check" :size="10" /></i><b /></span
            ><span><i /><b /></span><span><i /><b /></span
          ></template>
          <template v-else><strong>09<span>:</span>41</strong></template>
        </div>
        <div class="example-details">
          <h3>{{ t(example.title) }}</h3>
          <button
            class="secondary compact"
            :disabled="busy"
            :aria-label="t('ui.examples.addAria', { name: t(example.title) })"
            @click="emit('create', example.id)"
          >
            <AppIcon
              :name="pending === `example:${example.id}` ? 'refresh' : 'plus'"
              :size="15"
              :class="{ spinning: pending === `example:${example.id}` }"
            />{{
              pending === `example:${example.id}` ? t('ui.common.working') : t('ui.examples.add')
            }}
          </button>
        </div>
      </article>
    </div>
  </section>
</template>
