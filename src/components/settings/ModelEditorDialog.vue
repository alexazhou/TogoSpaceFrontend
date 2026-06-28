<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LlmModelConfig } from '../../types';

type EditorMode = 'create' | 'edit';

const emit = defineEmits<{
  save: [model: LlmModelConfig];
}>();

const { t } = useI18n();

const visible = ref(false);
const mode = ref<EditorMode>('create');

const form = ref({
  name: '',
  protocol: 'openai',
  context_window_tokens: 131072,
  reserve_output_tokens: 16384,
  compact_trigger_ratio: 0.85,
  compact_summary_max_tokens: 2048,
  extra_headers: '',
  provider_params: '',
});

const isCreating = computed(() => mode.value === 'create');
const dialogTitle = computed(() => (
  isCreating.value ? t('settings.models.newModelTitle', 'New Model') : (form.value.name || t('settings.models.detailFallback'))
));
const dialogEyebrow = computed(() => (isCreating.value ? 'New Model' : 'Model Detail'));
const advancedOpen = ref(false);

const canSave = computed(() => {
  return form.value.name.trim().length > 0 && form.value.protocol.trim().length > 0;
});

function closeDialog(): void {
  visible.value = false;
}

function openCreate(): void {
  mode.value = 'create';
  form.value = {
    name: '',
    protocol: 'openai',
    context_window_tokens: 131072,
    reserve_output_tokens: 16384,
    compact_trigger_ratio: 0.85,
    compact_summary_max_tokens: 2048,
    extra_headers: '',
    provider_params: '',
  };
  advancedOpen.value = false;
  visible.value = true;
}

function openEdit(model: LlmModelConfig): void {
  mode.value = 'edit';
  form.value = {
    name: model.name,
    protocol: model.protocol || 'openai',
    context_window_tokens: model.context_config?.context_window_tokens ?? 131072,
    reserve_output_tokens: model.context_config?.reserve_output_tokens ?? 16384,
    compact_trigger_ratio: model.context_config?.compact_trigger_ratio ?? 0.85,
    compact_summary_max_tokens: model.context_config?.compact_summary_max_tokens ?? 2048,
    extra_headers: serializeHeaders(model.extra_headers),
    provider_params: serializeProviderParams(model.provider_params),
  };
  advancedOpen.value = false;
  visible.value = true;
}

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

function handleSave(): void {
  if (!canSave.value) return;

  try {
    const model: LlmModelConfig = {
      name: form.value.name.trim(),
      protocol: form.value.protocol.trim(),
      context_config: {
        context_window_tokens: form.value.context_window_tokens,
        reserve_output_tokens: form.value.reserve_output_tokens,
        compact_trigger_ratio: form.value.compact_trigger_ratio,
        compact_summary_max_tokens: form.value.compact_summary_max_tokens,
      },
      extra_headers: parseHeaders(form.value.extra_headers),
      provider_params: parseProviderParams(form.value.provider_params),
    };
    emit('save', model);
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
          <div class="editor-head-copy">
            <p class="editor-eyebrow">{{ dialogEyebrow }}</p>
            <h3>{{ dialogTitle }}</h3>
          </div>
          <div class="editor-head-actions">
            <button type="button" class="ghost-button editor-close" @click="closeDialog">×</button>
          </div>
        </header>

        <div class="svc-form-grid">
          <label class="svc-field">
            <span>{{ t('settings.models.modelNameLabel', 'Model Name') }}</span>
            <input v-model="form.name" type="text" class="svc-input" :readonly="!isCreating" :class="{ 'svc-input--readonly': !isCreating }" placeholder="e.g. gpt-4o" />
          </label>

          <label class="svc-field">
            <span>Protocol</span>
            <select v-model="form.protocol" class="svc-input svc-select">
              <option value="openai">OpenAI</option>
              <option value="anthropic">Anthropic</option>
            </select>
          </label>
        </div>

        <section class="advanced-card">
          <button type="button" class="advanced-toggle" :aria-expanded="advancedOpen" @click="advancedOpen = !advancedOpen">
            <div>
              <p class="editor-eyebrow">Advanced</p>
              <strong>{{ t('settings.models.advanced') }}</strong>
            </div>
            <span class="advanced-toggle__state">{{ advancedOpen ? t('common.collapse') : t('common.expand') }}</span>
          </button>

          <div v-if="advancedOpen" class="advanced-grid">
            <label class="svc-field">
              <span>{{ t('settings.models.contextWindow') }}</span>
              <input v-model.number="form.context_window_tokens" type="number" class="svc-input" min="1024" />
            </label>

            <label class="svc-field">
              <span>{{ t('settings.models.outputReserved') }}</span>
              <input v-model.number="form.reserve_output_tokens" type="number" class="svc-input" min="256" />
            </label>

            <label class="svc-field">
              <span>{{ t('settings.models.compactRatio') }}</span>
              <input v-model.number="form.compact_trigger_ratio" type="number" class="svc-input" min="0" max="1" step="0.01" />
            </label>

            <label class="svc-field">
              <span>{{ t('settings.models.summaryMax') }}</span>
              <input v-model.number="form.compact_summary_max_tokens" type="number" class="svc-input" min="256" />
            </label>

            <label class="svc-field svc-field--wide">
              <span>{{ t('settings.models.extraHeaders') }}</span>
              <textarea v-model="form.extra_headers" class="svc-textarea" rows="3" placeholder="X-Custom-Header: value"></textarea>
            </label>

            <label class="svc-field svc-field--wide">
              <span>{{ t('settings.models.providerParams') }}</span>
              <textarea v-model="form.provider_params" class="svc-textarea svc-textarea--code" rows="4" placeholder="{}"></textarea>
            </label>
          </div>
        </section>

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
.editor-overlay { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 20px; background: rgba(6, 10, 16, 0.56); backdrop-filter: blur(10px); }
.editor-dialog { width: min(600px, calc(100vw - 40px)); max-height: calc(100vh - 40px); padding: 18px; display: grid; gap: 14px; overflow: auto; background: var(--panel-bg); border-radius: 12px; }
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
.svc-textarea { width: 100%; border: 1px solid var(--panel-border); border-radius: 12px; background: var(--panel-bg); color: var(--text-strong); padding: 10px 12px; font: inherit; font-size: 0.88rem; box-sizing: border-box; resize: vertical; }
.svc-textarea--code { font-family: monospace; font-size: 0.8rem; }
.advanced-card { border: 1px solid var(--panel-border); border-radius: 12px; padding: 12px; }
.advanced-toggle { display: flex; width: 100%; justify-content: space-between; align-items: center; background: transparent; border: none; padding: 0; cursor: pointer; text-align: left; }
.advanced-toggle strong { color: var(--text-strong); font-size: 0.9rem; }
.advanced-toggle__state { color: var(--muted); font-size: 0.8rem; }
.advanced-grid { margin-top: 14px; padding-top: 14px; border-top: 1px solid var(--panel-border); display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.editor-actions-trailing { display: flex; gap: 8px; justify-content: flex-end; }
</style>
