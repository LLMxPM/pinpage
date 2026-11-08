<!-- 功能：展示版本信息，并为更新设置留出内容插槽。 -->
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import AppIcon from '../../components/AppIcon.vue';
import { useUpdates } from './useUpdates';

const { t } = useI18n();
const { snapshot } = useUpdates();
</script>

<template>
  <div class="about-settings">
    <div class="about-identity">
      <span class="about-mark"><AppIcon name="pin" :size="18" /></span>
      <div>
        <p class="about-name">PinPage</p>
        <p>{{ t('ui.updates.currentVersion', { version: snapshot.currentVersion }) }}</p>
      </div>
    </div>
    <slot />
  </div>
</template>
