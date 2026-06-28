<script setup lang="ts">
import { onMounted, ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { getLlmConfig, saveLlmConfig, testLlmProvider } from '../../api';
import type { LlmConfigPayload, LlmProviderConfig, LlmModelConfig } from '../../types';
import ProviderEditorDialog from './ProviderEditorDialog.vue';
import ModelEditorDialog from './ModelEditorDialog.vue';
import SettingsBreadcrumb from './SettingsBreadcrumb.vue';
import type { SettingsBreadcrumbItem } from './types';
import InfoTooltip from '../ui/InfoTooltip.vue';
import { showGlobalSuccessToast } from '../../appUiState';

const props = defineProps<{
  breadcrumbItems: SettingsBreadcrumbItem[];
  detailProviderIndex?: number | null;
}>();

const emit = defineEmits<{
  navigateBreadcrumb: [key: string];
  openProviderModels: [index: number];
  clearProviderModels: [];
}>();

const { t } = useI18n();

const config = ref<LlmConfigPayload | null>(null);
const initialConfigSnapshot = ref<string>('');
const isDirty = computed(() => {
  if (!config.value) return false;
  return JSON.stringify(config.value) !== initialConfigSnapshot.value;
});
const isLoading = ref(false);
const isSaving = ref(false);
const statusText = ref('');

const providerDialogRef = ref<InstanceType<typeof ProviderEditorDialog> | null>(null);
const modelDialogRef = ref<InstanceType<typeof ModelEditorDialog> | null>(null);

// We need to keep track of which provider we are editing models for
const currentEditingProviderIndex = ref<number | null>(null);
const currentEditingModelIndex = ref<number | null>(null);
const currentEditingProviderIndexForEdit = ref<number | null>(null);

async function loadData(): Promise<void> {
  isLoading.value = true;
  statusText.value = '';
  try {
    config.value = await getLlmConfig();
    // Initialize context_config with defaults if null
    if (!config.value.context_config) {
      config.value.context_config = {
        compact_trigger_ratio: 0.8,
        reserve_output_tokens: 4096,
        context_window_tokens: 128000,
        compact_summary_max_tokens: 4096,
      };
    }
    initialConfigSnapshot.value = JSON.stringify(config.value);
  } catch (error) {
    console.error(error);
    statusText.value = t('settings.models.loadFailed', 'Failed to load configuration');
  } finally {
    isLoading.value = false;
  }
}

function resetChanges(): void {
  if (!initialConfigSnapshot.value) return;
  config.value = JSON.parse(initialConfigSnapshot.value) as LlmConfigPayload;
}

async function saveAll(): Promise<void> {
  if (!config.value) return;
  isSaving.value = true;
  statusText.value = '';
  try {
    await saveLlmConfig(config.value);
    initialConfigSnapshot.value = JSON.stringify(config.value);
    showGlobalSuccessToast(t('settings.models.saveSuccess', 'Configuration saved successfully!'));
  } catch (error) {
    console.error(error);
    statusText.value = t('settings.models.saveFailed', 'Failed to save configuration');
  } finally {
    isSaving.value = false;
  }
}

// Provider actions
function openAddProvider() {
  currentEditingProviderIndexForEdit.value = null;
  providerDialogRef.value?.openCreate();
}

function openEditProvider(index: number) {
  if (!config.value) return;
  currentEditingProviderIndexForEdit.value = index;
  providerDialogRef.value?.openEdit(config.value.llm_providers[index]);
}

function deleteProvider(index: number) {
  if (!config.value || !confirm('Are you sure you want to delete this provider?')) return;
  config.value.llm_providers.splice(index, 1);
}

function handleProviderSave(provider: LlmProviderConfig) {
  if (!config.value) return;
  if (currentEditingProviderIndexForEdit.value !== null) {
    provider.models = config.value.llm_providers[currentEditingProviderIndexForEdit.value].models;
    config.value.llm_providers[currentEditingProviderIndexForEdit.value] = provider;
  } else {
    config.value.llm_providers.push(provider);
  }
}

// Model actions
function openAddModel(providerIndex: number) {
  currentEditingProviderIndex.value = providerIndex;
  currentEditingModelIndex.value = null;
  modelDialogRef.value?.openCreate();
}

function openEditModel(providerIndex: number, modelIndex: number) {
  if (!config.value) return;
  currentEditingProviderIndex.value = providerIndex;
  currentEditingModelIndex.value = modelIndex;
  modelDialogRef.value?.openEdit(config.value.llm_providers[providerIndex].models[modelIndex]);
}

function deleteModel(providerIndex: number, modelIndex: number) {
  if (!config.value || !confirm('Are you sure you want to delete this model?')) return;
  config.value.llm_providers[providerIndex].models.splice(modelIndex, 1);
}

function handleModelSave(model: LlmModelConfig) {
  if (!config.value || currentEditingProviderIndex.value === null) return;
  const pIndex = currentEditingProviderIndex.value;
  if (currentEditingModelIndex.value !== null) {
    config.value.llm_providers[pIndex].models[currentEditingModelIndex.value] = model;
  } else {
    config.value.llm_providers[pIndex].models.push(model);
  }
}

// Test connectivity
const testingProviderIndex = ref<number | null>(null);
const testingModelIndex = ref<number | null>(null);
async function testModel(providerIndex: number, modelIndex: number) {
  if (!config.value) return;
  testingProviderIndex.value = providerIndex;
  testingModelIndex.value = modelIndex;
  
  const provider = config.value.llm_providers[providerIndex];
  const model = provider.models[modelIndex];
  
  try {
    const res = await testLlmProvider({
      provider: provider,
      model: model.name,
    });
    if (res.status === 'ok') {
      alert(`Test successful! Latency: ${res.detail?.duration_ms}ms\nResponse: ${res.detail?.response_text}`);
    } else {
      alert(`Test failed: ${res.message}\nDetail: ${res.detail?.raw_error}`);
    }
  } catch(e) {
    alert(`Test error: ${e}`);
  } finally {
    testingProviderIndex.value = null;
    testingModelIndex.value = null;
  }
}

const allModelOptions = computed(() => {
  if (!config.value) return [];
  const options: string[] = [];
  config.value.llm_providers.forEach(p => {
    if (p.enable) {
      p.models.forEach(m => {
        options.push(`${m.name}@${p.name}`);
      });
    }
  });
  return options;
});

onMounted(() => {
  void loadData();
});
</script>

<template>
  <section id="models" class="config-section">
    <SettingsBreadcrumb :items="breadcrumbItems" @navigate="emit('navigateBreadcrumb', $event)" />

    <div v-if="statusText" class="section-status-bar">
      <span class="section-status">{{ statusText }}</span>
    </div>

    <p v-if="isLoading" class="models-empty">{{ t('settings.models.loading', 'Loading configuration...') }}</p>

    <div v-else-if="config" class="config-content">
      
      <!-- Default Models Settings -->
      <section class="default-models-section">
        <h4>{{ t('settings.models.defaultModelsTitle', 'Default Models') }}</h4>
        <div class="slots-grid">
          <label class="svc-field">
            <span>
              {{ t('settings.models.primaryModel', 'Primary Model') }}
              <InfoTooltip :text="t('settings.models.primaryModelDesc', 'Default model for general tasks')" />
            </span>
            <select v-model="config.default_models.primary" class="svc-input svc-select">
              <option :value="null">-- Select Model --</option>
              <option v-for="opt in allModelOptions" :key="opt" :value="opt">{{ opt }}</option>
            </select>
          </label>
          <label class="svc-field">
            <span>
              {{ t('settings.models.lightweightModel', 'Lightweight Model') }}
              <InfoTooltip :text="t('settings.models.lightweightModelDesc', 'Faster model for simple tasks')" />
            </span>
            <select v-model="config.default_models.lightweight" class="svc-input svc-select">
              <option :value="null">-- Select Model --</option>
              <option v-for="opt in allModelOptions" :key="opt" :value="opt">{{ opt }}</option>
            </select>
          </label>
          <label class="svc-field">
            <span>
              {{ t('settings.models.visionModel', 'Vision Model') }}
              <InfoTooltip :text="t('settings.models.visionModelDesc', 'Model capable of processing images')" />
            </span>
            <select v-model="config.default_models.vision" class="svc-input svc-select">
              <option :value="null">-- Select Model --</option>
              <option v-for="opt in allModelOptions" :key="opt" :value="opt">{{ opt }}</option>
            </select>
          </label>
        </div>
      </section>

      <!-- Context Config -->
      <section class="context-config-section">
        <h4>
          {{ t('settings.models.contextConfigTitle', 'Default Context Config') }}
          <InfoTooltip :text="t('settings.models.contextConfigDesc', 'Global context window and compaction settings')" />
        </h4>
        <div class="slots-grid">
          <label class="svc-field">
            <span>
              {{ t('settings.models.contextWindowTokens', 'Context Window Tokens') }}
              <InfoTooltip :text="t('settings.models.contextWindowTokensDesc', 'Maximum context window size in tokens')" />
            </span>
            <input v-model.number="config.context_config.context_window_tokens" type="number" class="svc-input" min="0" step="1024" />
          </label>
          <label class="svc-field">
            <span>
              {{ t('settings.models.reserveOutputTokens', 'Reserve Output Tokens') }}
              <InfoTooltip :text="t('settings.models.reserveOutputTokensDesc', 'Reserved tokens for model output')" />
            </span>
            <input v-model.number="config.context_config.reserve_output_tokens" type="number" class="svc-input" min="0" step="256" />
          </label>
          <label class="svc-field">
            <span>
              {{ t('settings.models.compactTriggerRatio', 'Compact Trigger Ratio') }}
              <InfoTooltip :text="t('settings.models.compactTriggerRatioDesc', 'Ratio of context usage to trigger compaction (0-1)')" />
            </span>
            <input v-model.number="config.context_config.compact_trigger_ratio" type="number" class="svc-input" min="0" max="1" step="0.05" />
          </label>
          <label class="svc-field">
            <span>
              {{ t('settings.models.compactSummaryMaxTokens', 'Compact Summary Max Tokens') }}
              <InfoTooltip :text="t('settings.models.compactSummaryMaxTokensDesc', 'Maximum tokens for compaction summary')" />
            </span>
            <input v-model.number="config.context_config.compact_summary_max_tokens" type="number" class="svc-input" min="0" step="256" />
          </label>
        </div>
      </section>

      <!-- Providers View -->
      <section v-if="detailProviderIndex == null" class="providers-section">
        <div class="providers-header">
          <h4>{{ t('settings.models.providersTitle', 'LLM Providers') }}</h4>
          <button type="button" class="secondary-button" @click="openAddProvider">
            {{ t('settings.models.addProvider', 'Add Provider') }}
          </button>
        </div>

        <div class="models-table-wrap">
          <table class="settings-table models-table">
            <thead>
              <tr>
                <th>Provider Name</th>
                <th>Type</th>
                <th>Model Count</th>
                <th>Status</th>
                <th class="actions-th">{{ t('settings.models.table.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(provider, pIndex) in config.llm_providers" :key="pIndex" :class="{'provider-disabled': !provider.enable}">
                <td><strong>{{ provider.name }}</strong></td>
                <td><span class="models-cell-type">{{ provider.type }}</span></td>
                <td>{{ provider.models.length }}</td>
                <td>
                  <span v-if="provider.enable" class="svc-chip">Enabled</span>
                  <span v-else class="svc-chip svc-chip--disabled">Disabled</span>
                </td>
                <td class="models-cell-actions">
                  <button type="button" class="ghost-button" @click="openEditProvider(pIndex)">{{ t('common.edit') }}</button>
                  <button type="button" class="ghost-button" @click="emit('openProviderModels', pIndex)">Models</button>
                  <button type="button" class="ghost-button text-danger" @click="deleteProvider(pIndex)">{{ t('common.delete') }}</button>
                </td>
              </tr>
              <tr v-if="config.llm_providers.length === 0">
                <td colspan="5" class="models-empty">No providers configured yet.</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <!-- Models View -->
      <section v-else-if="detailProviderIndex != null && config.llm_providers[detailProviderIndex]" class="providers-section">
        <div class="providers-header">
          <div style="display: flex; align-items: center; gap: 8px;">
            <button type="button" class="ghost-button" @click="emit('clearProviderModels')" style="padding: 4px 8px;">&larr; Back</button>
            <h4 style="margin: 0;">【{{ config.llm_providers[detailProviderIndex].name }}】 Models</h4>
          </div>
          <button type="button" class="secondary-button" @click="openAddModel(detailProviderIndex)">
            {{ t('settings.models.addModel', 'Add Model') }}
          </button>
        </div>

        <div class="models-table-wrap">
          <table class="settings-table models-table">
            <thead>
              <tr>
                <th>{{ t('settings.models.modelNameLabel', 'Model') }}</th>
                <th>{{ t('settings.models.protocolLabel', 'Protocol') }}</th>
                <th class="actions-th">{{ t('settings.models.table.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(model, mIndex) in config.llm_providers[detailProviderIndex].models" :key="mIndex">
                <td><strong>{{ model.name }}</strong></td>
                <td><span class="models-cell-type">{{ model.protocol }}</span></td>
                <td class="models-cell-actions">
                  <button 
                    type="button" 
                    class="ghost-button" 
                    :disabled="testingProviderIndex === detailProviderIndex && testingModelIndex === mIndex"
                    @click="testModel(detailProviderIndex, mIndex)"
                  >
                    Test
                  </button>
                  <button type="button" class="ghost-button" @click="openEditModel(detailProviderIndex, mIndex)">Edit</button>
                  <button type="button" class="ghost-button text-danger" @click="deleteModel(detailProviderIndex, mIndex)">Del</button>
                </td>
              </tr>
              <tr v-if="config.llm_providers[detailProviderIndex].models.length === 0">
                <td colspan="3" class="models-empty">No models configured for this provider.</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

    </div>

    <div class="section-footer">
      <button v-if="isDirty" type="button" class="secondary-button" @click="resetChanges">
        {{ t('settings.models.resetChanges', 'Reset Changes') }}
      </button>
      <button type="button" class="primary-button" :disabled="isSaving || !config || !isDirty" @click="saveAll">
        {{ isSaving ? t('settings.models.saving', 'Saving...') : t('settings.models.saveAllBtn', 'Save All Changes') }}
      </button>
    </div>

    <ProviderEditorDialog ref="providerDialogRef" @save="handleProviderSave" />
    <ModelEditorDialog ref="modelDialogRef" @save="handleModelSave" />
  </section>
</template>

<style scoped>
.config-section { padding: 12px 0 0; }
.section-status-bar { margin-bottom: 8px; }
.section-status, .models-empty { color: var(--muted); font-size: 0.86rem; }

.section-footer {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 12px;
  padding-top: 16px;
  margin-top: 16px;
  border-top: 1px solid color-mix(in srgb, var(--divider) 76%, transparent);
}

.config-content { display: grid; gap: 24px; margin-top: 10px; }

.default-models-section h4 { margin: 0 0 12px 0; color: var(--text-strong); font-size: 0.95rem; }

.slots-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
}
.svc-field { display: grid; gap: 6px; }
.svc-field > span { color: var(--muted); font-size: 0.76rem; }
.svc-input, .svc-select {
  width: 100%; border: 1px solid var(--panel-border); border-radius: 12px;
  background: var(--panel-bg); color: var(--text-strong); padding: 8px 12px;
  font: inherit; font-size: 0.88rem; box-sizing: border-box;
}
.svc-input[type="number"] {
  -moz-appearance: textfield;
}
.svc-input[type="number"]::-webkit-outer-spin-button,
.svc-input[type="number"]::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.providers-header {
  display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;
}
.providers-header h4 { margin: 0; color: var(--text-strong); font-size: 0.95rem; }

.provider-card {
  background: var(--settings-table-surface);
  border-radius: 16px;
  padding: 16px;
  margin-bottom: 16px;
  border: 1px solid var(--panel-border);
}
.provider-disabled { opacity: 0.7; }

.provider-head {
  display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;
}
.provider-title { display: flex; align-items: center; gap: 8px; }
.provider-title strong { color: var(--text-strong); font-size: 1.05rem; }
.provider-actions { display: flex; gap: 8px; }
.text-danger { color: var(--warn); }

.svc-chip {
  display: inline-flex; align-items: center; min-height: 20px; padding: 0 8px;
  border-radius: 999px; border: 1px solid var(--panel-border); background: var(--panel-bg);
  color: var(--muted); font-size: 0.68rem; white-space: nowrap;
}
.svc-chip--disabled {
  border-color: color-mix(in srgb, var(--warn) 28%, var(--panel-border) 72%);
  background: color-mix(in srgb, var(--warn) 8%, var(--panel-bg) 92%);
  color: var(--warn);
}



.models-table-wrap {
  border-radius: 12px;
  background: var(--panel-bg);
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--panel-border);
}
.settings-table {
  width: 100%; min-width: 0; border-collapse: separate; border-spacing: 0; table-layout: fixed;
}
.settings-table th, .settings-table td {
  padding: 10px 14px; text-align: left; vertical-align: middle;
}
.settings-table thead th {
  border-bottom: 1px solid color-mix(in srgb, var(--divider) 86%, transparent);
  color: var(--text-strong); font-size: 0.8rem; font-weight: 700; white-space: nowrap;
  background: var(--settings-table-head-bg);
}
.settings-table tbody td {
  border-bottom: 1px solid color-mix(in srgb, var(--divider) 76%, transparent);
  color: var(--text-strong); font-size: 0.84rem;
}
.settings-table tbody tr:last-child td { border-bottom: none; }
.settings-table tbody tr:hover td { background: var(--settings-table-row-hover); }

.models-cell-type { color: var(--muted); }
.actions-th { width: 180px; text-align: right; }
.models-cell-actions { 
  display: flex; 
  justify-content: flex-end; 
  gap: 8px; 
  white-space: nowrap; 
}

.add-model-row {
  padding: 8px;
  background: var(--settings-table-head-bg);
  border-top: 1px solid var(--panel-border);
  text-align: center;
}

@media (max-width: 780px) {
  .slots-grid { grid-template-columns: 1fr; }
}
</style>
