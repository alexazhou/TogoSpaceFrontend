<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import SettingsBreadcrumb from './SettingsBreadcrumb.vue';
import type { SettingsBreadcrumbItem } from './types';

defineProps<{
  breadcrumbItems: SettingsBreadcrumbItem[];
  isBackingUp: boolean;
}>();

const emit = defineEmits<{
  backupDatabase: [];
  navigateBreadcrumb: [key: string];
}>();

const { t } = useI18n();
</script>

<template>
  <section id="maintenance" class="config-section maintenance-section">
    <SettingsBreadcrumb :items="breadcrumbItems" @navigate="emit('navigateBreadcrumb', $event)" />

    <section class="maintenance-panel panel">
      <div class="maintenance-head">
        <div class="maintenance-title-group">
          <h3>{{ t('settings.maintenance.title') }}</h3>
        </div>
        <button
          type="button"
          class="secondary-button"
          :disabled="isBackingUp"
          @click="emit('backupDatabase')"
        >
          {{ isBackingUp ? t('settings.maintenance.backingUp') : t('settings.maintenance.backupButton') }}
        </button>
      </div>

      <p class="maintenance-description">{{ t('settings.maintenance.description') }}</p>
      <p class="maintenance-note">{{ t('settings.maintenance.backupLocation') }}</p>
    </section>
  </section>
</template>

<style scoped>
.maintenance-section {
  display: grid;
  gap: 12px;
}

.maintenance-panel {
  display: grid;
  gap: 10px;
  padding: 16px;
  border-radius: 16px;
}

.maintenance-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.maintenance-title-group {
  display: grid;
  gap: 4px;
}

.maintenance-title-group h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 1rem;
}

.maintenance-description,
.maintenance-note {
  margin: 0;
  color: var(--text-secondary);
  font-size: 0.84rem;
  line-height: 1.5;
}

.maintenance-note {
  color: var(--text-tertiary);
}
</style>
