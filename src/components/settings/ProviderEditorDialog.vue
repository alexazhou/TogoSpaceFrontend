<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LlmProviderConfig } from '../../types';
import { getProviderPresets } from '../../api';
import ToggleSwitch from '../ui/ToggleSwitch.vue';

type EditorMode = 'create' | 'edit';

const providerPresets = ref<Record<string, { label: string; [key: string]: string }>>({});
const SERVICE_TYPES = computed(() =>
  Object.entries(providerPresets.value).map(([value, v]) => ({ value, label: v.label }))
);
const URL_PROTOCOL_TYPES = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
];

const emit = defineEmits<{
  save: [provider: LlmProviderConfig];
}>();

const { t } = useI18n();

const visible = ref(false);
const mode = ref<EditorMode>('create');
const apiKeyVisible = ref(false);

const urlsForm = ref<{ type: string; url: string }[]>([]);

onMounted(async () => {
  await loadPresets();
});

async function loadPresets() {
  try {
    providerPresets.value = await getProviderPresets();
  } catch (e) {
    console.error('Failed to load provider presets:', e);
  }
}

const form = ref({
  name: '',
  enable: true,
  type: 'other',
  api_key: '',
  extra_headers: '',
  provider_params: '',
});

function addUrl() {
  const usedTypes = urlsForm.value.map(u => u.type);
  const availableType = URL_PROTOCOL_TYPES.find(t => !usedTypes.includes(t.value));
  if (availableType) {
    urlsForm.value.push({ type: availableType.value, url: '' });
  }
}

function removeUrl(index: number) {
  urlsForm.value.splice(index, 1);
}

function handleTypeChange() {
  const preset = providerPresets.value[form.value.type];
  if (preset) {
    urlsForm.value = Object.entries(preset)
      .filter(([k]) => k !== 'label' && URL_PROTOCOL_TYPES.some(p => p.value === k))
      .map(([type, url]) => ({ type, url }));
  }
}

const isCreating = computed(() => mode.value === 'create');
const dialogTitle = computed(() => (
  isCreating.value ? t('settings.models.newProviderTitle', 'New Provider') : (form.value.name || t('settings.models.detailFallback'))
));
const dialogEyebrow = computed(() => (isCreating.value ? 'New Provider' : 'Provider Detail'));

const canSave = computed(() => {
  return form.value.name.trim().length > 0 && form.value.api_key.trim().length > 0;
});

function serializeHeaders(headers: Record<string, string> | undefined | null): string {
  if (!headers) return '';
  return Object.entries(headers)
    .filter(([k, v]) => k && v)
    .map(([k, v]) => `${k}: ${v}`)
    .join('\n');
}

function parseHeaders(text: string): Record<string, string> {
  const headers: Record<string, string> = {};
  for (const line of text.split('\n')) {
    const dividerIndex = line.indexOf(':');
    if (dividerIndex <= 0) continue;
    const key = line.slice(0, dividerIndex).trim();
    const value = line.slice(dividerIndex + 1).trim();
    if (key && value) {
      headers[key] = value;
    }
  }
  return headers;
}

function serializeProviderParams(params: Record<string, unknown> | undefined | null): string {
  if (!params || Object.keys(params).length === 0) return '';
  return JSON.stringify(params, null, 2);
}

function parseProviderParams(text: string): Record<string, unknown> {
  const trimmed = text.trim();
  if (!trimmed) return {};
  try {
    const parsed = JSON.parse(trimmed);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') {
      throw new Error();
    }
    return parsed;
  } catch {
    throw new Error(t('settings.models.providerParamsInvalid'));
  }
}

function closeDialog(): void {
  visible.value = false;
}

function openCreate(): void {
  mode.value = 'create';
  urlsForm.value = [{ type: 'openai', url: '' }];
  form.value = {
    name: '',
    enable: true,
    type: 'other',
    api_key: '',
    extra_headers: '',
    provider_params: '',
  };
  apiKeyVisible.value = false;
  visible.value = true;
  void loadPresets();
}

function openEdit(provider: LlmProviderConfig): void {
  mode.value = 'edit';

  urlsForm.value = [];
  if (provider.urls) {
    for (const [k, v] of Object.entries(provider.urls)) {
      if (v) urlsForm.value.push({ type: k, url: v });
    }
  }

  form.value = {
    name: provider.name,
    enable: provider.enable ?? true,
    type: provider.type || 'other',
    api_key: provider.api_key || '',
    extra_headers: serializeHeaders(provider.extra_headers),
    provider_params: serializeProviderParams(provider.provider_params),
  };
  apiKeyVisible.value = false;
  visible.value = true;
  void loadPresets();
}

function handleSave(): void {
  if (!canSave.value) return;

  try {
    const urlsToSave: Record<string, string> = {};
    urlsForm.value.forEach(item => {
      if (item.url.trim()) urlsToSave[item.type] = item.url.trim();
    });

    const provider: LlmProviderConfig = {
      name: form.value.name.trim(),
      enable: form.value.enable,
      type: form.value.type as any,
      api_key: form.value.api_key.trim(),
      urls: urlsToSave,
      extra_headers: parseHeaders(form.value.extra_headers),
      provider_params: parseProviderParams(form.value.provider_params),
      models: [], // to be merged by parent
    };
    emit('save', provider);
    closeDialog();
  } catch (error) {
    alert(error instanceof Error ? error.message : 'Invalid parameters');
  }
}

defineExpose({ openCreate, openEdit });
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="editor-overlay" @click.self="closeDialog">
      <section class="editor-dialog panel scrollbar-thin">
        <header class="editor-head">
          <div class="editor-head-copy" style="display: flex; align-items: center; gap: 16px;">
            <div>
              <p class="editor-eyebrow">{{ dialogEyebrow }}</p>
              <h3>{{ dialogTitle }}</h3>
            </div>
            <div style="margin-top: 14px;">
              <ToggleSwitch variant="inline" :checked="form.enable" :label="form.enable ? t('settings.models.enabled') : t('settings.models.disabled')" @toggle="form.enable = $event" />
            </div>
          </div>
          <div class="editor-head-actions">
            <button type="button" class="ghost-button editor-close" @click="closeDialog">×</button>
          </div>
        </header>

        <div class="svc-form-grid">
          <label class="svc-field">
            <span>{{ t('settings.models.nameLabel') }}</span>
            <input v-model="form.name" type="text" class="svc-input" :readonly="!isCreating" :class="{ 'svc-input--readonly': !isCreating }" placeholder="e.g. OpenAI" />
          </label>

          <label class="svc-field">
            <span>{{ t('settings.models.typeLabel') }}</span>
            <select v-model="form.type" class="svc-input svc-select" @change="handleTypeChange">
              <option v-for="st in SERVICE_TYPES" :key="st.value" :value="st.value">{{ st.label }}</option>
            </select>
          </label>

          <div class="svc-field svc-field--wide">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;">
              <span>Base URLs</span>
            </div>
            
            <div v-for="(item, index) in urlsForm" :key="index" style="display: flex; gap: 8px; margin-bottom: 8px; align-items: center;">
              <select v-model="item.type" class="svc-input svc-select" style="width: 140px; flex-shrink: 0;">
                <option v-for="type in URL_PROTOCOL_TYPES" :key="type.value" :value="type.value" :disabled="urlsForm.some((u, i) => u.type === type.value && i !== index)">
                  {{ type.label }}
                </option>
              </select>
              <input v-model="item.url" type="text" class="svc-input svc-input--flex" placeholder="https://..." />
              <button type="button" class="ghost-button" @click="removeUrl(index)" style="padding: 0 8px; color: var(--error);">
                ×
              </button>
            </div>
            <button type="button" class="secondary-button" @click="addUrl" :disabled="urlsForm.length >= URL_PROTOCOL_TYPES.length" style="width: 100%; border-style: dashed; justify-content: center;">
              + Add URL
            </button>
          </div>

          <label class="svc-field svc-field--wide">
            <span>API Key</span>
            <div class="svc-input-group">
              <input v-model="form.api_key" :type="apiKeyVisible ? 'text' : 'password'" class="svc-input svc-input--flex" placeholder="sk-..." />
              <button type="button" class="ghost-button" @click="apiKeyVisible = !apiKeyVisible">
                {{ apiKeyVisible ? t('settings.models.apiKeyHide') : t('settings.models.apiKeyShow') }}
              </button>
            </div>
          </label>
        </div>

        <footer class="editor-actions">
          <div class="editor-actions-leading"></div>
          <div class="editor-actions-trailing">
            <button type="button" class="secondary-button" @click="closeDialog">{{ t('common.cancel') }}</button>
            <button type="button" class="secondary-button" :disabled="!canSave" @click="handleSave">{{ t('common.confirm') }}</button>
          </div>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
/* Inherit from existing styles */
.editor-overlay { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 20px; background: rgba(6, 10, 16, 0.56); backdrop-filter: blur(10px); }
.editor-dialog { width: min(760px, calc(100vw - 40px)); max-height: calc(100vh - 40px); padding: 18px; display: grid; gap: 14px; overflow: auto; background: var(--panel-bg); border-radius: 12px; }
.editor-head, .editor-actions { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
.editor-head-copy { min-width: 0; }
.editor-close { min-width: 32px; height: 32px; padding: 0; font-size: 1rem; }
.editor-eyebrow { margin: 0; color: var(--accent); text-transform: uppercase; letter-spacing: 0.14em; font-size: 0.68rem; }
.editor-head h3 { margin: 0; color: var(--text-strong); }
.svc-form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.svc-field { display: grid; gap: 6px; }
.svc-field--wide { grid-column: 1 / -1; }
.svc-field > span { color: var(--muted); font-size: 0.76rem; }
.svc-input, .svc-select { height: 40px; width: 100%; border: 1px solid var(--panel-border); border-radius: 12px; background: var(--panel-bg); color: var(--text-strong); padding: 0 12px; font: inherit; font-size: 0.88rem; box-sizing: border-box; }
.svc-input-group { display: flex; gap: 8px; }
.svc-input--flex { flex: 1; min-width: 0; }
.editor-actions-trailing { display: flex; gap: 8px; justify-content: flex-end; }
</style>
