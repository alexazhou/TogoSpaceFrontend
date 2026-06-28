<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LlmContextConfig } from '../../types';
import InfoTooltip from '../ui/InfoTooltip.vue';

const props = defineProps<{
  config: LlmContextConfig;
}>();

const emit = defineEmits<{
  save: [config: LlmContextConfig];
}>();

const { t } = useI18n();

const visible = ref(false);
const form = ref<LlmContextConfig>({
  compact_trigger_ratio: 0.8,
  reserve_output_tokens: 4096,
  context_window_tokens: 128000,
  compact_summary_max_tokens: 4096,
});

function openDialog(): void {
  form.value = { ...props.config };
  visible.value = true;
}

function closeDialog(): void {
  visible.value = false;
}

function handleSave(): void {
  emit('save', { ...form.value });
  closeDialog();
}

function formatNumber(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
  if (n >= 1000) return `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}K`;
  return String(n);
}
</script>

<template>
  <section class="context-config-section">
    <div class="context-config-header">
      <h4>
        {{ t('settings.models.contextConfigTitle', 'Default Context Config') }}
        <InfoTooltip :text="t('settings.models.contextConfigDesc', 'Global context window and compaction settings')" />
      </h4>
      <button type="button" class="ghost-button" @click="openDialog">
        {{ t('common.edit') }}
      </button>
    </div>

    <div class="context-config-tags">
      <span class="context-tag">
        <span class="context-tag-label">Context Window</span>
        <span class="context-tag-value">{{ formatNumber(config.context_window_tokens) }}</span>
      </span>
      <span class="context-tag">
        <span class="context-tag-label">Reserve Output</span>
        <span class="context-tag-value">{{ formatNumber(config.reserve_output_tokens) }}</span>
      </span>
      <span class="context-tag">
        <span class="context-tag-label">Compact Trigger</span>
        <span class="context-tag-value">{{ (config.compact_trigger_ratio * 100).toFixed(0) }}%</span>
      </span>
      <span class="context-tag">
        <span class="context-tag-label">Summary Max</span>
        <span class="context-tag-value">{{ formatNumber(config.compact_summary_max_tokens) }}</span>
      </span>
    </div>

    <!-- Edit Dialog -->
    <Teleport to="body">
      <div v-if="visible" class="editor-overlay" @click.self="closeDialog">
        <section class="editor-dialog panel scrollbar-thin">
          <header class="editor-head">
            <div class="editor-head-copy">
              <p class="editor-eyebrow">Context Config</p>
              <h3>{{ t('settings.models.contextConfigTitle', 'Default Context Config') }}</h3>
            </div>
            <div class="editor-head-actions">
              <button type="button" class="ghost-button editor-close" @click="closeDialog">×</button>
            </div>
          </header>

          <div class="editor-form">
            <label class="svc-field">
              <span>
                {{ t('settings.models.contextWindowTokens', 'Context Window Tokens') }}
                <InfoTooltip :text="t('settings.models.contextWindowTokensDesc', 'Maximum context window size in tokens')" />
              </span>
              <input v-model.number="form.context_window_tokens" type="number" class="svc-input" min="0" step="1024" />
            </label>

            <label class="svc-field">
              <span>
                {{ t('settings.models.reserveOutputTokens', 'Reserve Output Tokens') }}
                <InfoTooltip :text="t('settings.models.reserveOutputTokensDesc', 'Reserved tokens for model output')" />
              </span>
              <input v-model.number="form.reserve_output_tokens" type="number" class="svc-input" min="0" step="256" />
            </label>

            <label class="svc-field">
              <span>
                {{ t('settings.models.compactTriggerRatio', 'Compact Trigger Ratio') }}
                <InfoTooltip :text="t('settings.models.compactTriggerRatioDesc', 'Ratio of context usage to trigger compaction (0-1)')" />
              </span>
              <input v-model.number="form.compact_trigger_ratio" type="number" class="svc-input" min="0" max="1" step="0.05" />
            </label>

            <label class="svc-field">
              <span>
                {{ t('settings.models.compactSummaryMaxTokens', 'Compact Summary Max Tokens') }}
                <InfoTooltip :text="t('settings.models.compactSummaryMaxTokensDesc', 'Maximum tokens for compaction summary')" />
              </span>
              <input v-model.number="form.compact_summary_max_tokens" type="number" class="svc-input" min="0" step="256" />
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
.context-config-section {
  padding: 0;
}

.context-config-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.context-config-header h4 {
  margin: 0;
  color: var(--text-strong);
  font-size: 0.95rem;
}

.context-config-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.context-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 8px;
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  font-size: 0.84rem;
}

.context-tag-label {
  color: var(--muted);
}

.context-tag-value {
  color: var(--text-strong);
  font-weight: 600;
}

/* Dialog styles */
.editor-overlay { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 20px; background: rgba(6, 10, 16, 0.56); backdrop-filter: blur(10px); }
.editor-dialog { width: min(480px, calc(100vw - 40px)); max-height: calc(100vh - 40px); padding: 18px; display: grid; gap: 14px; overflow: auto; background: var(--panel-bg); border-radius: 12px; }
.editor-head, .editor-actions { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
.editor-head-copy { min-width: 0; }
.editor-close { min-width: 32px; height: 32px; padding: 0; font-size: 1rem; }
.editor-eyebrow { margin: 0; color: var(--accent); text-transform: uppercase; letter-spacing: 0.14em; font-size: 0.68rem; }
.editor-head h3 { margin: 0; color: var(--text-strong); }
.editor-form { display: grid; gap: 12px; }
.svc-field { display: grid; gap: 6px; }
.svc-field > span { color: var(--muted); font-size: 0.76rem; }
.svc-input {
  width: 100%; border: 1px solid var(--panel-border); border-radius: 12px;
  background: var(--panel-bg); color: var(--text-strong); padding: 8px 12px;
  font: inherit; font-size: 0.88rem; box-sizing: border-box;
}
.svc-input[type="number"] { -moz-appearance: textfield; }
.svc-input[type="number"]::-webkit-outer-spin-button,
.svc-input[type="number"]::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
.editor-actions-trailing { display: flex; gap: 8px; justify-content: flex-end; }
</style>
