<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { DefaultModelSlots, LlmProviderConfig } from '../../types';
import InfoTooltip from '../ui/InfoTooltip.vue';

const props = defineProps<{
  defaultModels: DefaultModelSlots;
  providers: LlmProviderConfig[];
}>();

const emit = defineEmits<{
  save: [models: DefaultModelSlots];
}>();

const { t } = useI18n();

const visible = ref(false);
const form = ref<DefaultModelSlots>({
  primary: '',
  lightweight: '',
  vision: '',
});

// 每个槽位当前选中的 provider name
const selectedProviders = ref<Record<string, string>>({
  primary: '',
  lightweight: '',
  vision: '',
});

const enabledProviders = computed(() =>
  props.providers.filter(p => p.enable && p.models.length > 0)
);

function getProviderName(modelValue: string | null): string {
  if (!modelValue) return '';
  const atIndex = modelValue.indexOf('@');
  return atIndex === -1 ? '' : modelValue.substring(atIndex + 1);
}

function getModelName(modelValue: string | null): string {
  if (!modelValue) return '';
  const atIndex = modelValue.indexOf('@');
  return atIndex === -1 ? modelValue : modelValue.substring(0, atIndex);
}

function getModelsForProvider(providerName: string): string[] {
  const provider = enabledProviders.value.find(p => p.name === providerName);
  if (!provider) return [];
  return provider.models.map(m => m.name);
}

function openDialog(): void {
  form.value = { ...props.defaultModels };
  selectedProviders.value = {
    primary: getProviderName(props.defaultModels.primary),
    lightweight: getProviderName(props.defaultModels.lightweight),
    vision: getProviderName(props.defaultModels.vision),
  };
  visible.value = true;
}

function closeDialog(): void {
  visible.value = false;
}

function handleProviderChange(slot: 'primary' | 'lightweight' | 'vision'): void {
  // 切换 provider 时，清空对应的 model 选择
  const models = getModelsForProvider(selectedProviders.value[slot]);
  if (models.length === 1) {
    form.value[slot] = `${models[0]}@${selectedProviders.value[slot]}`;
  } else {
    form.value[slot] = '';
  }
}

function handleModelChange(slot: 'primary' | 'lightweight' | 'vision'): void {
  // model select 的值直接就是完整的 "modelName@providerName"
  // 不需要额外处理
}

function handleSave(): void {
  emit('save', { ...form.value });
  closeDialog();
}

function resolveModelDisplay(value: string | null): string {
  if (!value) return t('common.notConfigured', '未配置');
  const atIndex = value.indexOf('@');
  if (atIndex === -1) return value;
  const modelName = value.substring(0, atIndex);
  const providerName = value.substring(atIndex + 1);
  return `${modelName} (${providerName})`;
}
</script>

<template>
  <section class="default-models-section">
    <div class="default-models-header">
      <h4>
        {{ t('settings.models.defaultModelsTitle', 'Default Models') }}
        <InfoTooltip :text="t('settings.models.defaultModelsDesc', 'Global default model assignments for different task types')" />
      </h4>
      <button type="button" class="ghost-button" @click="openDialog">
        {{ t('common.edit') }}
      </button>
    </div>

    <div class="default-models-tags">
      <span class="model-tag">
        <span class="model-tag-label">{{ t('settings.models.primaryModel', 'Primary Model') }}</span>
        <span class="model-tag-value">{{ resolveModelDisplay(defaultModels.primary) }}</span>
      </span>
      <span class="model-tag">
        <span class="model-tag-label">{{ t('settings.models.lightweightModel', 'Lightweight Model') }}</span>
        <span class="model-tag-value">{{ resolveModelDisplay(defaultModels.lightweight) }}</span>
      </span>
      <span class="model-tag">
        <span class="model-tag-label">{{ t('settings.models.visionModel', 'Vision Model') }}</span>
        <span class="model-tag-value">{{ resolveModelDisplay(defaultModels.vision) }}</span>
      </span>
    </div>

    <!-- Edit Dialog -->
    <Teleport to="body">
      <div v-if="visible" class="editor-overlay" @click.self="closeDialog">
        <section class="editor-dialog panel scrollbar-thin">
          <header class="editor-head">
            <div class="editor-head-copy">
              <p class="editor-eyebrow">Default Models</p>
              <h3>{{ t('settings.models.defaultModelsTitle', 'Default Models') }}</h3>
            </div>
            <div class="editor-head-actions">
              <button type="button" class="ghost-button editor-close" @click="closeDialog">×</button>
            </div>
          </header>

          <div class="editor-form">
            <!-- 主模型 -->
            <label class="svc-field">
              <span>
                {{ t('settings.models.primaryModel', 'Primary Model') }}
                <InfoTooltip :text="t('settings.models.primaryModelDesc', 'Default model for general tasks')" />
              </span>
              <div class="model-cascader">
                <div class="model-cascader-col">
                  <span class="model-cascader-label">{{ t('settings.models.providerLabel', 'Provider') }}</span>
                  <select
                    v-model="selectedProviders.primary"
                    class="svc-input svc-select model-cascader-select"
                    @change="handleProviderChange('primary')"
                  >
                    <option value="">{{ t('common.notConfigured', '未配置') }}</option>
                    <option v-for="p in enabledProviders" :key="p.name" :value="p.name">{{ p.name }}</option>
                  </select>
                </div>
                <div class="model-cascader-col">
                  <span class="model-cascader-label">{{ t('settings.models.modelLabel', 'Model') }}</span>
                  <select
                    v-model="form.primary"
                    class="svc-input svc-select model-cascader-select"
                    :disabled="!selectedProviders.primary"
                    @change="handleModelChange('primary')"
                  >
                    <option value="">{{ t('common.notConfigured', '未配置') }}</option>
                    <option v-for="m in getModelsForProvider(selectedProviders.primary)" :key="m" :value="`${m}@${selectedProviders.primary}`">
                      {{ m }}
                    </option>
                  </select>
                </div>
              </div>
            </label>

            <!-- 轻量模型 -->
            <label class="svc-field">
              <span>
                {{ t('settings.models.lightweightModel', 'Lightweight Model') }}
                <InfoTooltip :text="t('settings.models.lightweightModelDesc', 'Faster model for simple tasks')" />
              </span>
              <div class="model-cascader">
                <div class="model-cascader-col">
                  <span class="model-cascader-label">{{ t('settings.models.providerLabel', 'Provider') }}</span>
                  <select
                    v-model="selectedProviders.lightweight"
                    class="svc-input svc-select model-cascader-select"
                    @change="handleProviderChange('lightweight')"
                  >
                    <option value="">{{ t('common.notConfigured', '未配置') }}</option>
                    <option v-for="p in enabledProviders" :key="p.name" :value="p.name">{{ p.name }}</option>
                  </select>
                </div>
                <div class="model-cascader-col">
                  <span class="model-cascader-label">{{ t('settings.models.modelLabel', 'Model') }}</span>
                  <select
                    v-model="form.lightweight"
                    class="svc-input svc-select model-cascader-select"
                    :disabled="!selectedProviders.lightweight"
                    @change="handleModelChange('lightweight')"
                  >
                    <option value="">{{ t('common.notConfigured', '未配置') }}</option>
                    <option v-for="m in getModelsForProvider(selectedProviders.lightweight)" :key="m" :value="`${m}@${selectedProviders.lightweight}`">
                      {{ m }}
                    </option>
                  </select>
                </div>
              </div>
            </label>

            <!-- 视觉模型 -->
            <label class="svc-field">
              <span>
                {{ t('settings.models.visionModel', 'Vision Model') }}
                <InfoTooltip :text="t('settings.models.visionModelDesc', 'Model capable of processing images')" />
              </span>
              <div class="model-cascader">
                <div class="model-cascader-col">
                  <span class="model-cascader-label">{{ t('settings.models.providerLabel', 'Provider') }}</span>
                  <select
                    v-model="selectedProviders.vision"
                    class="svc-input svc-select model-cascader-select"
                    @change="handleProviderChange('vision')"
                  >
                    <option value="">{{ t('common.notConfigured', '未配置') }}</option>
                    <option v-for="p in enabledProviders" :key="p.name" :value="p.name">{{ p.name }}</option>
                  </select>
                </div>
                <div class="model-cascader-col">
                  <span class="model-cascader-label">{{ t('settings.models.modelLabel', 'Model') }}</span>
                  <select
                    v-model="form.vision"
                    class="svc-input svc-select model-cascader-select"
                    :disabled="!selectedProviders.vision"
                    @change="handleModelChange('vision')"
                  >
                    <option value="">{{ t('common.notConfigured', '未配置') }}</option>
                    <option v-for="m in getModelsForProvider(selectedProviders.vision)" :key="m" :value="`${m}@${selectedProviders.vision}`">
                      {{ m }}
                    </option>
                  </select>
                </div>
              </div>
            </label>
          </div>

          <footer class="editor-actions">
            <div class="editor-actions-leading"></div>
            <div class="editor-actions-trailing">
              <button type="button" class="secondary-button" @click="closeDialog">{{ t('common.cancel') }}</button>
              <button type="button" class="secondary-button" @click="handleSave">{{ t('common.confirm') }}</button>
            </div>
          </footer>
        </section>
      </div>
    </Teleport>
  </section>
</template>

<style scoped>
.default-models-section {
  padding: 0;
}

.default-models-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.default-models-header h4 {
  margin: 0;
  color: var(--text-strong);
  font-size: 0.95rem;
}

.default-models-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.model-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 8px;
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  font-size: 0.84rem;
}

.model-tag-label {
  color: var(--muted);
}

.model-tag-value {
  color: var(--text-strong);
  font-weight: 600;
}

/* Dialog styles */
.editor-overlay { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 20px; background: rgba(6, 10, 16, 0.56); backdrop-filter: blur(10px); }
.editor-dialog { width: min(560px, calc(100vw - 40px)); max-height: calc(100vh - 40px); padding: 18px; display: grid; gap: 14px; overflow: auto; background: var(--panel-bg); border-radius: 12px; }
.editor-head, .editor-actions { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
.editor-head-copy { min-width: 0; }
.editor-close { min-width: 32px; height: 32px; padding: 0; font-size: 1rem; }
.editor-eyebrow { margin: 0; color: var(--accent); text-transform: uppercase; letter-spacing: 0.14em; font-size: 0.68rem; }
.editor-head h3 { margin: 0; color: var(--text-strong); }
.editor-form { display: grid; gap: 14px; }
.svc-field { display: grid; gap: 6px; }
.svc-field > span { color: var(--muted); font-size: 0.76rem; }
.svc-input {
  width: 100%; border: 1px solid var(--panel-border); border-radius: 12px;
  background: var(--panel-bg); color: var(--text-strong); padding: 8px 12px;
  font: inherit; font-size: 0.88rem; box-sizing: border-box;
}
.svc-select { appearance: none; cursor: pointer; }
.editor-actions-trailing { display: flex; gap: 8px; justify-content: flex-end; }

/* 两级选择器 */
.model-cascader {
  display: grid;
  grid-template-columns: 1fr 1.4fr;
  gap: 8px;
}

.model-cascader-col {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.model-cascader-label {
  color: var(--muted);
  font-size: 0.68rem;
  letter-spacing: 0.02em;
}

.model-cascader-select {
  min-width: 0;
}

.model-cascader-select:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
