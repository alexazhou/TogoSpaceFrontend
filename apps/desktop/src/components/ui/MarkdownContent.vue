<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { showGlobalSuccessToast } from '../../appUiState';
import { isTauriRuntime, openPathInFinder } from '../../desktop';
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
const LOCAL_PATH_PATTERN = /(^|[\s([{"'`])((?:\/|[A-Za-z]:[\\/])(?:[^\n\r<>])+?)(?=$|[\s)\]}",'`，。；：！？、])/gm;

function codeBlockLanguage(pre: HTMLElement): string {
  return pre.dataset.language?.trim() || 'text';
}

function syncWrapButtonState(button: HTMLButtonElement, wrapped: boolean): void {
  button.setAttribute('aria-pressed', wrapped ? 'true' : 'false');
  button.setAttribute('aria-label', wrapped ? t('common.disableCodeWrap') : t('common.enableCodeWrap'));
  button.title = wrapped ? t('common.disableCodeWrap') : t('common.enableCodeWrap');
}

function hasAbsoluteLocalPathPrefix(value: string): boolean {
  const candidate = value.trim();
  return candidate.startsWith('/') || /^[A-Za-z]:[\\/]/.test(candidate);
}

function normalizeLocalPathCandidate(value: string): string | null {
  let candidate = value.trim();
  if (!candidate || candidate.includes('://') || !hasAbsoluteLocalPathPrefix(candidate)) {
    return null;
  }

  candidate = candidate.replace(/[,\uFF0C;\uFF1B:\uFF1A!\uFF01?\uFF1F\u3001)\]}\"'`]+$/g, '');
  if (candidate.endsWith('.')) {
    const lastSegment = candidate.split(/[\\/]/).pop() ?? '';
    if (!/\.[A-Za-z0-9]{1,16}$/.test(lastSegment)) {
      candidate = candidate.replace(/\.+$/g, '');
    }
  }

  candidate = candidate.trim();
  return candidate && hasAbsoluteLocalPathPrefix(candidate) ? candidate : null;
}

function isAbsoluteLocalPath(value: string): boolean {
  return normalizeLocalPathCandidate(value) !== null;
}

function resolveFolderPath(path: string): string {
  const trimmedPath = path.trim().replace(/[\\/]+$/, '');
  const segments = trimmedPath.split(/[\\/]/);
  const lastSegment = segments[segments.length - 1] ?? '';
  const looksLikeFile = lastSegment.startsWith('.') || /\.[A-Za-z0-9]{1,16}$/.test(lastSegment);

  if (!looksLikeFile) {
    return trimmedPath;
  }

  const lastSeparatorIndex = Math.max(trimmedPath.lastIndexOf('/'), trimmedPath.lastIndexOf('\\'));
  if (lastSeparatorIndex <= 0) {
    return trimmedPath;
  }
  return trimmedPath.slice(0, lastSeparatorIndex);
}

type LocalPathMatch = {
  start: number;
  end: number;
  path: string;
};

function findLocalPathMatches(content: string): LocalPathMatch[] {
  const matches: LocalPathMatch[] = [];
  LOCAL_PATH_PATTERN.lastIndex = 0;
  let match = LOCAL_PATH_PATTERN.exec(content);

  while (match) {
    const leading = match[1] ?? '';
    const rawPath = match[2] ?? '';
    const normalizedPath = normalizeLocalPathCandidate(rawPath);

    if (normalizedPath) {
      const start = match.index + leading.length;
      const end = start + normalizedPath.length;
      matches.push({ start, end, path: normalizedPath });
    }

    match = LOCAL_PATH_PATTERN.exec(content);
  }

  return matches;
}

function createLocalPathChip(path: string): HTMLElement {
  const finderTarget = resolveFolderPath(path);
  const chip = document.createElement('span');
  chip.className = 'markdown-local-path';
  chip.dataset.localPath = finderTarget;
  chip.dataset.copyPath = path;

  const label = document.createElement('span');
  label.className = 'markdown-local-path__label';
  label.textContent = path;
  label.title = path;

  const actions = document.createElement('span');
  actions.className = 'markdown-local-path__actions';

  const finderButton = document.createElement('button');
  finderButton.type = 'button';
  finderButton.className = 'markdown-local-path__action';
  finderButton.dataset.action = 'finder';
  finderButton.setAttribute('aria-label', t('chat.openInFinder'));
  finderButton.title = t('chat.openInFinder');
  finderButton.innerHTML = '<i class="fa-solid fa-folder-open" aria-hidden="true"></i>';

  const copyButton = document.createElement('button');
  copyButton.type = 'button';
  copyButton.className = 'markdown-local-path__action';
  copyButton.dataset.action = 'copy';
  copyButton.setAttribute('aria-label', t('chat.copyPath'));
  copyButton.title = t('chat.copyPath');
  copyButton.innerHTML = '<i class="fa-solid fa-copy" aria-hidden="true"></i>';

  actions.appendChild(finderButton);
  actions.appendChild(copyButton);
  chip.appendChild(label);
  chip.appendChild(actions);
  return chip;
}

function enhanceLocalPathTextNode(textNode: Text): boolean {
  const content = textNode.textContent ?? '';
  if (!content.trim()) {
    return false;
  }

  const matches = findLocalPathMatches(content);
  if (!matches.length) {
    const trimmed = normalizeLocalPathCandidate(content);
    if (!trimmed || !isAbsoluteLocalPath(trimmed)) {
      return false;
    }

    const leadingWhitespace = content.match(/^\s*/)?.[0] ?? '';
    const trailingWhitespace = content.match(/\s*$/)?.[0] ?? '';
    const fragment = document.createDocumentFragment();

    if (leadingWhitespace) {
      fragment.appendChild(document.createTextNode(leadingWhitespace));
    }
    fragment.appendChild(createLocalPathChip(trimmed));
    if (trailingWhitespace) {
      fragment.appendChild(document.createTextNode(trailingWhitespace));
    }

    textNode.parentNode?.replaceChild(fragment, textNode);
    return true;
  }

  const fragment = document.createDocumentFragment();
  let cursor = 0;

  matches.forEach((match) => {
    if (match.start > cursor) {
      fragment.appendChild(document.createTextNode(content.slice(cursor, match.start)));
    }
    fragment.appendChild(createLocalPathChip(match.path));
    cursor = match.end;
  });

  if (cursor < content.length) {
    fragment.appendChild(document.createTextNode(content.slice(cursor)));
  }

  textNode.parentNode?.replaceChild(fragment, textNode);
  return true;
}

function enhanceLocalPaths(): void {
  const root = rootRef.value;
  if (!root || typeof document === 'undefined') {
    return;
  }

  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const textNodes: Text[] = [];
  let currentNode = walker.nextNode();

  while (currentNode) {
    if (
      currentNode instanceof Text
      && currentNode.parentElement
      && !currentNode.parentElement.closest('a, code, pre, button, .markdown-local-path, .markdown-local-path__action')
    ) {
      textNodes.push(currentNode);
    }
    currentNode = walker.nextNode();
  }

  textNodes.forEach((textNode) => {
    enhanceLocalPathTextNode(textNode);
  });
}

function enhanceInlineCodeLocalPaths(): void {
  const root = rootRef.value;
  if (!root) {
    return;
  }

  root.querySelectorAll('code').forEach((codeElement) => {
    const code = codeElement as HTMLElement;
    if (
      code.closest('pre')
      || code.closest('.markdown-local-path')
      || code.childElementCount > 0
    ) {
      return;
    }

    const normalizedPath = normalizeLocalPathCandidate(code.textContent ?? '');
    if (!normalizedPath) {
      return;
    }

    code.replaceWith(createLocalPathChip(normalizedPath));
  });
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

    const actions = document.createElement('div');
    actions.className = 'markdown-code-actions';

    const wrapButton = document.createElement('button');
    wrapButton.type = 'button';
    wrapButton.className = 'markdown-code-action markdown-code-wrap';
    syncWrapButtonState(wrapButton, false);
    wrapButton.innerHTML = '<i class="fa-solid fa-align-left" aria-hidden="true"></i>';

    const copyButton = document.createElement('button');
    copyButton.type = 'button';
    copyButton.className = 'markdown-code-action markdown-code-copy';
    copyButton.setAttribute('aria-label', t('common.copy'));
    copyButton.title = t('common.copy');
    copyButton.innerHTML = '<i class="fa-solid fa-copy" aria-hidden="true"></i>';

    actions.appendChild(wrapButton);
    actions.appendChild(copyButton);

    toolbar.appendChild(language);
    toolbar.appendChild(actions);

    pre.parentNode?.insertBefore(wrapper, pre);
    wrapper.appendChild(toolbar);
    wrapper.appendChild(pre);
  });
}

async function syncCodeBlocks(): Promise<void> {
  await nextTick();
  enhanceCodeBlocks();
  enhanceLocalPaths();
  enhanceInlineCodeLocalPaths();
}

async function handleRootClick(event: MouseEvent): Promise<void> {
  const target = event.target;
  const targetElement = target instanceof HTMLElement
    ? target
    : target instanceof Node
      ? target.parentElement
      : null;
  if (!(targetElement instanceof HTMLElement)) {
    return;
  }

  const localPathAction = targetElement.closest('.markdown-local-path__action');
  if (localPathAction instanceof HTMLButtonElement) {
    const chip = localPathAction.closest('.markdown-local-path');
    if (!(chip instanceof HTMLElement)) {
      return;
    }

    const action = localPathAction.dataset.action;
    const localPath = chip.dataset.localPath?.trim();
    const copyPath = chip.dataset.copyPath?.trim() ?? localPath ?? '';
    if (!action) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    if (action === 'finder') {
      if (!isTauriRuntime() || !localPath) {
        return;
      }

      try {
        await openPathInFinder(localPath);
      } catch (error) {
        console.error(error);
      }
      return;
    }

    if (action === 'copy' && copyPath) {
      try {
        await navigator.clipboard.writeText(copyPath);
        showGlobalSuccessToast(t('common.copied'));
      } catch (error) {
        console.error(error);
      }
      return;
    }
  }

  const wrapButton = targetElement.closest('.markdown-code-wrap');
  if (wrapButton instanceof HTMLButtonElement) {
    const wrapper = wrapButton.closest('.markdown-code-block');
    if (!(wrapper instanceof HTMLElement)) {
      return;
    }
    const wrapped = wrapper.classList.toggle('markdown-code-block--wrapped');
    syncWrapButtonState(wrapButton, wrapped);
    return;
  }

  const button = targetElement.closest('.markdown-code-copy');
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
  --markdown-code-bg: var(--markdown-code-bg-default, var(--surface-panel));
  --markdown-code-shadow: none;
  --markdown-code-text: var(--text-primary);
  --markdown-inline-code-bg: var(--markdown-inline-code-bg-default, var(--surface-elevated));
  --markdown-link-color: var(--markdown-link-color-default, color-mix(in srgb, var(--state-info) 88%, white 12%));
  --markdown-link-hover-color: var(--markdown-link-hover-color-default, color-mix(in srgb, var(--state-info) 76%, white 24%));
}

:global(:root[data-theme='dark']) .markdown-content {
  --markdown-code-bg: var(--markdown-code-bg-default, var(--surface-panel-muted));
  --markdown-code-shadow: none;
  --markdown-code-text: var(--text-primary);
  --markdown-inline-code-bg: var(--markdown-inline-code-bg-default, var(--surface-panel-muted));
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
  padding: 0.12rem 0.6rem 0.5rem;
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
  padding: 0.16em 0.35em;
  border-radius: 7px;
  background: color-mix(in srgb, var(--surface-panel-muted) 88%, var(--surface-panel) 12%);
  border: 1px solid color-mix(in srgb, var(--border-default) 72%, transparent);
  color: var(--text-primary);
}

.markdown-content :deep(pre code) {
  display: block;
  color: var(--markdown-code-text);
  line-height: 1.45;
  white-space: pre;
}

.markdown-content :deep(.markdown-code-block--wrapped pre) {
  overflow-x: hidden;
}

.markdown-content :deep(.markdown-code-block--wrapped pre code) {
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.markdown-content :deep(a) {
  color: var(--markdown-link-color);
  text-decoration: underline;
  text-underline-offset: 0.14em;
  word-break: break-word;
}

.markdown-content :deep(.markdown-local-path) {
  display: inline-flex;
  align-items: center;
  gap: 0.34rem;
  max-width: 100%;
  margin: 0 0.1rem;
  padding: 0.2rem 0.3rem 0.2rem 0.5rem;
  border: 1px solid color-mix(in srgb, var(--border-default) 68%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface-panel-muted) 88%, var(--surface-panel) 12%);
  vertical-align: middle;
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 12%, transparent);
}

.markdown-content :deep(.markdown-local-path__label) {
  min-width: 0;
  color: var(--text-primary);
  font-size: 0.84em;
  line-height: 1.4;
  overflow-wrap: anywhere;
}

.markdown-content :deep(.markdown-local-path__actions) {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  flex: 0 0 auto;
}

.markdown-content :deep(.markdown-local-path__action) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.28rem;
  height: 1.28rem;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface-panel) 28%, transparent);
  color: var(--text-secondary);
  font-size: 0.66rem;
  line-height: 1;
  cursor: pointer;
  transition:
    border-color 140ms ease,
    background 140ms ease,
    color 140ms ease,
    transform 140ms ease;
}

.markdown-content :deep(.markdown-local-path__action:hover) {
  border-color: color-mix(in srgb, var(--interactive-focus-border) 40%, transparent);
  background: color-mix(in srgb, var(--interactive-selected) 20%, var(--surface-panel) 80%);
  color: var(--text-primary);
  transform: translateY(-1px);
}

.markdown-content :deep(.markdown-local-path__action i) {
  pointer-events: none;
}

.markdown-content :deep(a:hover) {
  color: var(--markdown-link-hover-color);
}

.markdown-content :deep(table) {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.95em;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  padding: 0.48rem 0.62rem;
  border: 1px solid var(--gray-600);
  text-align: left;
  vertical-align: top;
}

.markdown-content :deep(th) {
  color: var(--text-primary);
  background: var(--markdown-table-header-bg-default, var(--surface-panel-muted));
}

.markdown-content :deep(hr) {
  border: 0;
  border-top: 1px solid var(--gray-600);
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
  color: var(--markdown-code-toolbar-text);
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

.markdown-content :deep(.markdown-code-actions) {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 4px;
}

.markdown-content :deep(.markdown-code-action) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--markdown-code-toolbar-text);
  font-size: 0.65rem;
  line-height: 1;
  cursor: pointer;
  transition:
    background 140ms ease,
    color 140ms ease;
}

.markdown-content :deep(.markdown-code-action:hover),
.markdown-content :deep(.markdown-code-wrap[aria-pressed='true']) {
  color: var(--text-primary);
}

.markdown-content :deep(.markdown-code-action i) {
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
