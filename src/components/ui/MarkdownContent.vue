<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { showGlobalSuccessToast } from '../../appUiState';
import { renderMarkdown } from '../../utils/markdown';

const props = withDefaults(defineProps<{
  content: string;
  inline?: boolean;
}>(), {
  inline: false,
});

const { t } = useI18n();
const rootRef = ref<HTMLElement | null>(null);
const renderedContent = computed(() => renderMarkdown(props.content, { inline: props.inline }));

function codeBlockLanguage(pre: HTMLElement): string {
  return pre.dataset.language?.trim() || 'text';
}

function enhanceCodeBlocks(): void {
  const root = rootRef.value;
  if (!root) {
    return;
  }

  root.querySelectorAll('pre').forEach((preElement) => {
    const pre = preElement as HTMLElement;
    if (pre.parentElement?.classList.contains('markdown-code-block')) {
      return;
    }

    const wrapper = document.createElement('div');
    wrapper.className = 'markdown-code-block';

    const toolbar = document.createElement('div');
    toolbar.className = 'markdown-code-toolbar';

    const language = document.createElement('span');
    language.className = 'markdown-code-language';
    language.textContent = codeBlockLanguage(pre);

    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'markdown-code-copy';
    button.setAttribute('aria-label', t('common.copy'));
    button.innerHTML = '<i class="fa-solid fa-copy" aria-hidden="true"></i>';

    toolbar.appendChild(language);
    toolbar.appendChild(button);

    pre.parentNode?.insertBefore(wrapper, pre);
    wrapper.appendChild(toolbar);
    wrapper.appendChild(pre);
  });
}

async function syncCodeBlocks(): Promise<void> {
  await nextTick();
  enhanceCodeBlocks();
}

async function handleRootClick(event: MouseEvent): Promise<void> {
  const target = event.target;
  if (!(target instanceof HTMLElement)) {
    return;
  }

  const button = target.closest('.markdown-code-copy');
  if (!(button instanceof HTMLButtonElement)) {
    return;
  }

  const wrapper = button.closest('.markdown-code-block');
  const pre = wrapper?.querySelector('pre');
  const codeText = pre?.textContent?.trimEnd() ?? '';
  if (!codeText) {
    return;
  }

  try {
    await navigator.clipboard.writeText(codeText);
    showGlobalSuccessToast(t('common.copied'));
  } catch (error) {
    console.error(error);
  }
}

watch(renderedContent, () => {
  syncCodeBlocks().catch(console.error);
});

onMounted(() => {
  syncCodeBlocks().catch(console.error);
});
</script>

<template>
  <div
    ref="rootRef"
    class="markdown-content"
    :class="{ 'markdown-content--inline': inline }"
    v-html="renderedContent"
    @click="handleRootClick"
  ></div>
</template>

<style scoped>
.markdown-content {
  color: inherit;
  font: inherit;
  --markdown-code-bg: var(--surface-panel);
  --markdown-code-shadow: none;
  --markdown-code-text: var(--text-primary);
  --markdown-inline-code-bg: var(--surface-elevated);
}

:global(:root[data-theme='dark']) .markdown-content {
  --markdown-code-bg: var(--surface-panel-muted);
  --markdown-code-shadow: none;
  --markdown-code-text: var(--text-primary);
  --markdown-inline-code-bg: var(--surface-panel-muted);
}

.markdown-content :deep(*) {
  box-sizing: border-box;
}

.markdown-content :deep(p),
.markdown-content :deep(blockquote),
.markdown-content :deep(ul),
.markdown-content :deep(ol),
.markdown-content :deep(pre),
.markdown-content :deep(table),
.markdown-content :deep(hr) {
  margin: 0;
}

.markdown-content :deep(* + p),
.markdown-content :deep(* + blockquote),
.markdown-content :deep(* + ul),
.markdown-content :deep(* + ol),
.markdown-content :deep(* + .markdown-code-block),
.markdown-content :deep(* + table),
.markdown-content :deep(* + hr) {
  margin-top: 0.7em;
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4),
.markdown-content :deep(h5),
.markdown-content :deep(h6) {
  margin: 0.9em 0 0;
  color: inherit;
  font-size: 1em;
  font-weight: 700;
  line-height: 1.35;
}

.markdown-content :deep(h1:first-child),
.markdown-content :deep(h2:first-child),
.markdown-content :deep(h3:first-child),
.markdown-content :deep(h4:first-child),
.markdown-content :deep(h5:first-child),
.markdown-content :deep(h6:first-child) {
  margin-top: 0;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  padding-left: 1.45rem;
}

.markdown-content :deep(li + li) {
  margin-top: 0.22rem;
}

.markdown-content :deep(li > p) {
  margin: 0;
}

.markdown-content :deep(blockquote) {
  padding: 0.75em 0.95em;
  border-left: 3px solid var(--border-strong);
  border-radius: 0 10px 10px 0;
  background: color-mix(in srgb, var(--surface-panel-muted) 74%, transparent);
  color: var(--text-secondary);
}

.markdown-content :deep(pre) {
  overflow-x: auto;
  padding: 0 0.6rem 0.5rem;
  border-radius: 0 0 8px 8px;
  background: transparent;
  border: none;
  box-shadow: var(--markdown-code-shadow);
  scrollbar-width: thin;
  scrollbar-color: var(--markdown-code-scrollbar-thumb) var(--markdown-code-scrollbar-track);
}

.markdown-content :deep(pre::-webkit-scrollbar) {
  width: 10px;
  height: 10px;
}

.markdown-content :deep(pre::-webkit-scrollbar-track) {
  background: var(--markdown-code-scrollbar-track);
  border-radius: 999px;
}

.markdown-content :deep(pre::-webkit-scrollbar-thumb) {
  min-width: 56px;
  border: 2px solid transparent;
  border-radius: 999px;
  background: var(--markdown-code-scrollbar-thumb);
  background-clip: padding-box;
}

.markdown-content :deep(pre::-webkit-scrollbar-thumb:hover) {
  background: var(--markdown-code-scrollbar-thumb-hover);
  background-clip: padding-box;
}

.markdown-content :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  font-size: 0.92em;
}

.markdown-content :deep(:not(pre) > code) {
  padding: 0.12em 0.38em;
  border-radius: 6px;
  background: var(--markdown-inline-code-bg);
  color: color-mix(in srgb, var(--text-primary) 92%, white 8%);
}

.markdown-content :deep(pre code) {
  display: block;
  color: var(--markdown-code-text);
  line-height: 1.45;
  white-space: pre;
}

.markdown-content :deep(a) {
  color: color-mix(in srgb, var(--state-info) 88%, white 12%);
  text-decoration: underline;
  text-underline-offset: 0.14em;
  word-break: break-word;
}

.markdown-content :deep(a:hover) {
  color: color-mix(in srgb, var(--state-info) 76%, white 24%);
}

.markdown-content :deep(table) {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.95em;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  padding: 0.48rem 0.62rem;
  border: 1px solid color-mix(in srgb, var(--border-default) 36%, transparent);
  text-align: left;
  vertical-align: top;
}

.markdown-content :deep(th) {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--surface-panel-muted) 72%, transparent);
}

.markdown-content :deep(hr) {
  border: 0;
  border-top: 1px solid color-mix(in srgb, var(--border-default) 44%, transparent);
}

.markdown-content :deep(input[type='checkbox']) {
  margin: 0 0.42rem 0 0;
  accent-color: var(--state-success);
  pointer-events: none;
}

.markdown-content :deep(.task-list-item) {
  list-style: none;
}

.markdown-content :deep(.task-list-item-checkbox) {
  vertical-align: middle;
}

.markdown-content :deep(.hljs) {
  color: var(--markdown-code-text);
}

.markdown-content :deep(.markdown-code-block) {
  overflow: hidden;
  border-radius: 8px;
  background: var(--markdown-code-bg);
}

.markdown-content :deep(.markdown-code-toolbar) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 24px;
  padding: 0.22rem 0.55rem 0;
  color: var(--text-secondary);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  font-size: 0.72rem;
  line-height: 1.35;
}

.markdown-content :deep(.markdown-code-language) {
  overflow: hidden;
  min-width: 0;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.markdown-content :deep(.markdown-code-copy) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.65rem;
  line-height: 1;
  cursor: pointer;
  transition:
    background 140ms ease,
    color 140ms ease;
}

.markdown-content :deep(.markdown-code-copy:hover) {
  color: var(--text-primary);
}

.markdown-content :deep(.markdown-code-copy i) {
  pointer-events: none;
}

.markdown-content :deep(.hljs-comment),
.markdown-content :deep(.hljs-quote) {
  color: var(--text-secondary);
}

.markdown-content :deep(.hljs-keyword),
.markdown-content :deep(.hljs-selector-tag),
.markdown-content :deep(.hljs-literal),
.markdown-content :deep(.hljs-section),
.markdown-content :deep(.hljs-link) {
  color: #ffb86c;
}

.markdown-content :deep(.hljs-string),
.markdown-content :deep(.hljs-title),
.markdown-content :deep(.hljs-name),
.markdown-content :deep(.hljs-attribute),
.markdown-content :deep(.hljs-symbol),
.markdown-content :deep(.hljs-bullet),
.markdown-content :deep(.hljs-addition) {
  color: #8dd6a5;
}

.markdown-content :deep(.hljs-number),
.markdown-content :deep(.hljs-meta),
.markdown-content :deep(.hljs-built_in),
.markdown-content :deep(.hljs-builtin-name),
.markdown-content :deep(.hljs-type),
.markdown-content :deep(.hljs-params) {
  color: #7dbff2;
}

.markdown-content :deep(.hljs-deletion) {
  color: #ef8a84;
}

.markdown-content--inline :deep(* + p),
.markdown-content--inline :deep(* + blockquote),
.markdown-content--inline :deep(* + ul),
.markdown-content--inline :deep(* + ol),
.markdown-content--inline :deep(* + pre),
.markdown-content--inline :deep(* + table),
.markdown-content--inline :deep(* + hr) {
  margin-top: 0.5em;
}
</style>
