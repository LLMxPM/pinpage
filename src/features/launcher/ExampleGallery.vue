<!-- 功能：展示合并后的七种示例、用途与使用条件，以静态预览反馈创建状态。 -->
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
          <template v-else-if="example.id === 'calendar'"
            ><small>11 月</small><strong>15</strong>
            <div class="preview-dots"><i /><i /><i /><i /><i /><i /><i /></div
          ></template>
          <template v-else-if="example.id === 'kanban'"
            ><div class="preview-board">
              <span><i /><i /></span><span><i /></span><span><i /><i /></span></div
          ></template>
          <template v-else-if="example.id === 'stocks'"
            ><strong>+1.28<span>%</span></strong
            ><svg class="preview-spark" viewBox="0 0 70 20" aria-hidden="true">
              <polyline
                points="0,18 10,12 20,15 30,9 40,11 50,4 60,7 70,2"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
              /></svg
          ></template>
          <template v-else-if="example.id === 'folder'"
            ><span class="preview-folder-icon"></span>
            <div class="preview-files"><i /><i /></div
          ></template>
          <template v-else-if="example.id === 'systemMonitor'"
            ><strong>37<span>%</span></strong>
            <svg class="preview-spark" viewBox="0 0 70 20" aria-hidden="true">
              <polyline
                points="0,15 10,12 20,14 30,7 40,10 50,4 60,9 70,6"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
              /></svg
          ></template>
        </div>
        <div class="example-details">
          <h3>{{ t(example.title) }}</h3>
          <p class="example-description">{{ t(`ui.examples.descriptions.${example.id}`) }}</p>
          <div class="example-actions">
            <span class="example-badge" :class="example.badge">{{
              t(`ui.examples.badges.${example.badge}`)
            }}</span>
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
        </div>
      </article>
    </div>
  </section>
</template>
