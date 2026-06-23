<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { getAgentAvatarUrl } from '../../avatar';
import { showGlobalSuccessToast } from '../../appUiState';
import { appendDesktopDebugEvent, isTauriRuntime, listenDesktopDragDropEvent } from '../../desktop';
import { displayName, i18nText } from '../../utils';
import type { MessageInfo, RoomMemberProfile, RoomState } from '../../types';
import { useAgentStatus } from '../../realtime/selectors';
import MessageStream from './MessageStream.vue';

const COMPOSER_DRAG_ZONE_HEIGHT_PX = 8;
const COMPOSER_RATIO_STORAGE_KEY = 'chat-composer-height-ratio';

const props = defineProps<{
  currentRoom: RoomState | null;
  memberProfiles: RoomMemberProfile[];
  messages: MessageInfo[];
  hasMoreHistory: boolean;
  loadingOlderMessages: boolean;
  errorMessage: string;
  reloadingMessages: boolean;
  draft: string;
  composerNotice: string;
  escalatingMessageIds?: number[];
}>();

const emit = defineEmits<{
  updateDraft: [value: string];
  submit: [];
  clickWorkingAgent: [agentId: number];
  clickAgent: [agentId: number];
  loadOlderMessages: [];
  escalateMessage: [messageId: number];
  openRoomSettings: [];
}>();

const { t } = useI18n();

const hasBanner = computed(() => Boolean(props.errorMessage || props.reloadingMessages));
const hasComposer = computed(() => Boolean(props.currentRoom && !props.composerNotice));
const membersOpen = ref(false);
const isDraftComposing = ref(false);
const fileDragActive = ref(false);
const composerDraft = ref(props.draft);
const currentMembers = computed(() => props.memberProfiles);
const composerDividerDragging = ref(false);
const composerHeightRatio = ref(0.28);

const chatRef = useTemplateRef('chatRef');
const chatHeadRef = useTemplateRef('chatHeadRef');
const chatBannerRef = useTemplateRef('chatBannerRef');
const messageViewportRef = useTemplateRef('messageViewportRef');

const chatHeight = ref(0);
const chatHeadHeight = ref(0);
const chatBannerHeight = ref(0);

let layoutResizeObserver: ResizeObserver | null = null;
let stopComposerResize: (() => void) | null = null;
let pendingComposerBottomAnchorSync = false;
let pendingComposerBottomAnchorDistance: number | null = null;
let removeDesktopDragDropListener: (() => void) | null = null;

// #region debug-point shared:reporter
function reportDebugEvent(
  hypothesisId: 'A' | 'B' | 'C' | 'D' | 'E',
  msg: string,
  data: Record<string, unknown>,
  runId = 'pre-fix',
): void {
  if (!isTauriRuntime()) {
    return;
  }

  void appendDesktopDebugEvent({
      sessionId: 'drag-path-missing',
      runId,
      hypothesisId,
      location: 'src/components/chat/ChatPanel.vue',
      msg: `[DEBUG] ${msg}`,
      data,
      ts: Date.now(),
  }).catch(() => {});
}
// #endregion

const isScheduling = computed(() => props.currentRoom?.state === 'scheduling');
const currentTurnAgentId = computed(() => props.currentRoom?.current_turn_agent_id ?? null);
const currentTurnAgent = computed<RoomMemberProfile | null>(() => {
  const agentId = currentTurnAgentId.value;
  if (agentId === null) {
    return null;
  }
  return props.memberProfiles.find((member) => member.id === agentId) ?? null;
});
const currentSpeaker = computed(() => {
  const agent = currentTurnAgent.value;
  return agent ? displayName(agent) : null;
});

const turnAgentStatus = useAgentStatus(currentTurnAgentId);
const workingAgent = computed<RoomMemberProfile | null>(() => {
  if (
    props.currentRoom?.need_scheduling
    && isScheduling.value
    && currentTurnAgent.value
    && turnAgentStatus.value === 'active'
  ) {
    return currentTurnAgent.value;
  }
  return null;
});

const isDeptRoom = computed(() => props.currentRoom?.tags?.includes('DEPT') ?? false);

const composerMetrics = computed(() => {
  if (!hasComposer.value) {
    return null;
  }

  const reservedHeight = chatHeadHeight.value
    + (hasBanner.value ? chatBannerHeight.value : 0);
  const availableHeight = chatHeight.value - reservedHeight;
  if (availableHeight <= 0) {
    return null;
  }

  const minComposerHeight = Math.min(220, Math.max(100, Math.round(availableHeight * 0.18)));
  const minMessageHeight = Math.min(260, Math.max(120, Math.round(availableHeight * 0.24)));
  const maxComposerHeight = Math.max(minComposerHeight, availableHeight - minMessageHeight);
  const composerHeight = Math.round(
    Math.min(
      maxComposerHeight,
      Math.max(minComposerHeight, availableHeight * composerHeightRatio.value),
    ),
  );

  return {
    availableHeight,
    minComposerHeight,
    maxComposerHeight,
    composerHeight,
  };
});

const chatLayoutStyle = computed(() => {
  if (hasComposer.value) {
    const composerHeight = composerMetrics.value?.composerHeight ?? 200;
    return {
      gridTemplateRows: hasBanner.value
        ? `auto auto minmax(0, 1fr) ${composerHeight}px`
        : `auto minmax(0, 1fr) ${composerHeight}px`,
    };
  }

  if (props.composerNotice) {
    return {
      gridTemplateRows: hasBanner.value
        ? 'auto auto minmax(0, 1fr) auto'
        : 'auto minmax(0, 1fr) auto',
    };
  }

  return {
    gridTemplateRows: hasBanner.value
      ? 'auto auto minmax(0, 1fr)'
      : 'auto minmax(0, 1fr)',
  };
});

const composerStyle = computed(() => (
  hasComposer.value
    ? {
      height: `${composerMetrics.value?.composerHeight ?? 200}px`,
      '--composer-drag-zone-height': `${COMPOSER_DRAG_ZONE_HEIGHT_PX}px`,
    }
    : {}
));

function persistComposerHeightRatio(): void {
  try {
    localStorage.setItem(COMPOSER_RATIO_STORAGE_KEY, String(composerHeightRatio.value));
  } catch {
    // ignore localStorage failures
  }
}

function restoreComposerHeightRatio(): void {
  try {
    const raw = localStorage.getItem(COMPOSER_RATIO_STORAGE_KEY);
    if (!raw) {
      return;
    }

    const parsed = Number(raw);
    if (Number.isFinite(parsed) && parsed >= 0.12 && parsed <= 0.72) {
      composerHeightRatio.value = parsed;
    }
  } catch {
    // ignore localStorage failures
  }
}

function refreshLayoutMetrics(): void {
  chatHeight.value = chatRef.value?.clientHeight ?? 0;
  chatHeadHeight.value = chatHeadRef.value?.clientHeight ?? 0;
  chatBannerHeight.value = chatBannerRef.value?.clientHeight ?? 0;
}

function bindLayoutResizeObserver(): void {
  layoutResizeObserver?.disconnect();

  if (typeof ResizeObserver === 'undefined') {
    return;
  }

  layoutResizeObserver = new ResizeObserver(() => {
    refreshLayoutMetrics();
    if (composerDividerDragging.value && pendingComposerBottomAnchorDistance !== null) {
      preserveMessageBottomAnchor(pendingComposerBottomAnchorDistance);
    }
  });

  if (chatRef.value) {
    layoutResizeObserver.observe(chatRef.value);
  }
  if (chatHeadRef.value) {
    layoutResizeObserver.observe(chatHeadRef.value);
  }
  if (chatBannerRef.value) {
    layoutResizeObserver.observe(chatBannerRef.value);
  }
  if (messageViewportRef.value) {
    layoutResizeObserver.observe(messageViewportRef.value);
  }
}

function resetComposerDragState(): void {
  composerDividerDragging.value = false;
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}

function getMessageStreamElement(): HTMLElement | null {
  const stream = chatRef.value?.querySelector('.message-stream');
  return stream instanceof HTMLElement ? stream : null;
}

function preserveMessageBottomAnchor(distanceToBottom: number): void {
  const stream = getMessageStreamElement();
  if (!stream) {
    return;
  }

  const nextScrollTop = stream.scrollHeight - stream.clientHeight - distanceToBottom;
  stream.scrollTop = Math.max(0, nextScrollTop);
}

function scheduleComposerBottomAnchorSync(): void {
  if (pendingComposerBottomAnchorSync) {
    return;
  }

  pendingComposerBottomAnchorSync = true;
  void nextTick(() => {
    pendingComposerBottomAnchorSync = false;
    if (pendingComposerBottomAnchorDistance !== null) {
      preserveMessageBottomAnchor(pendingComposerBottomAnchorDistance);
    }
  });
}

function startComposerResize(event: PointerEvent): void {
  const metrics = composerMetrics.value;
  if (!metrics) {
    return;
  }

  event.preventDefault();

  const startHeight = metrics.composerHeight;
  const startY = event.clientY;

  composerDividerDragging.value = true;
  document.body.style.cursor = 'row-resize';
  document.body.style.userSelect = 'none';

  const stopResize = (): void => {
    resetComposerDragState();
    pendingComposerBottomAnchorDistance = null;
    window.removeEventListener('pointermove', handlePointerMove);
    window.removeEventListener('pointerup', stopResize);
    window.removeEventListener('pointercancel', stopResize);
    stopComposerResize = null;
  };

  const handlePointerMove = (moveEvent: PointerEvent): void => {
    const stream = getMessageStreamElement();
    const distanceToBottom = stream
      ? stream.scrollHeight - stream.scrollTop - stream.clientHeight
      : 0;
    const nextComposerHeight = Math.min(
      metrics.maxComposerHeight,
      Math.max(metrics.minComposerHeight, startHeight - (moveEvent.clientY - startY)),
    );
    pendingComposerBottomAnchorDistance = distanceToBottom;
    composerHeightRatio.value = nextComposerHeight / metrics.availableHeight;
    persistComposerHeightRatio();
    scheduleComposerBottomAnchorSync();
  };

  window.addEventListener('pointermove', handlePointerMove);
  window.addEventListener('pointerup', stopResize, { once: true });
  window.addEventListener('pointercancel', stopResize, { once: true });
  stopComposerResize = stopResize;
}

watch(
  () => props.currentRoom?.room_id ?? null,
  () => {
    membersOpen.value = false;
  },
);

watch(
  () => props.draft,
  (value) => {
    // #region debug-point C:prop-draft-watch
    reportDebugEvent('C', 'props.draft watcher fired', {
      incomingDraftLength: value.length,
      composerDraftLength: composerDraft.value.length,
      incomingPreview: value.slice(0, 200),
    });
    // #endregion
    if (value !== composerDraft.value) {
      composerDraft.value = value;
    }
  },
  { immediate: true },
);

watch(fileDragActive, (value) => {
  // #region debug-point D:file-drag-active-watch
  reportDebugEvent('D', 'fileDragActive changed', {
    fileDragActive: value,
  });
  // #endregion
});

watch(
  () => [hasBanner.value, hasComposer.value, props.composerNotice] as const,
  async () => {
    await nextTick();
    refreshLayoutMetrics();
    bindLayoutResizeObserver();
  },
);

function toggleMembers(): void {
  if (!props.currentRoom) {
    return;
  }
  membersOpen.value = !membersOpen.value;
}

function closeMembers(): void {
  membersOpen.value = false;
}

function openRoomSettings(): void {
  if (!props.currentRoom) {
    return;
  }
  emit('openRoomSettings');
}

function handleComposerSubmit(): void {
  if (isDraftComposing.value) {
    return;
  }
  emit('submit');
}

function updateComposerDraft(value: string): void {
  // #region debug-point C:update-composer-draft
  reportDebugEvent('C', 'updateComposerDraft called', {
    previousDraftLength: composerDraft.value.length,
    nextDraftLength: value.length,
    preview: value.slice(0, 200),
  });
  // #endregion
  composerDraft.value = value;
  emit('updateDraft', value);
}

function handleEnterKey(e: KeyboardEvent): void {
  if (isDraftComposing.value || e.isComposing || e.keyCode === 229) {
    return;
  }
  e.preventDefault();
  emit('submit');
}

function isFileDrag(dataTransfer: DataTransfer | null): boolean {
  if (!dataTransfer) {
    return false;
  }

  const types = Array.from(dataTransfer.types ?? []);
  return types.includes('Files') || dataTransfer.files.length > 0;
}

function extractDroppedFilePaths(dataTransfer: DataTransfer | null): string[] {
  if (!dataTransfer) {
    return [];
  }

  const files = Array.from(dataTransfer.files ?? []) as Array<File & { path?: string }>;
  const directPaths = files
    .map((file) => file.path?.trim())
    .filter((value): value is string => Boolean(value));

  const uriListPaths = (dataTransfer.getData('text/uri-list') ?? '')
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith('#'))
    .map((line) => {
      if (!line.startsWith('file://')) {
        return '';
      }

      try {
        const decoded = decodeURI(new URL(line).pathname);
        return decoded.trim();
      } catch {
        return '';
      }
    })
    .filter((value): value is string => Boolean(value));

  const textPaths = (dataTransfer.getData('text/plain') ?? '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith('/') || /^[A-Za-z]:[\\/]/.test(line));

  return Array.from(new Set([...directPaths, ...uriListPaths, ...textPaths]));
}

function extractDroppedFileNames(dataTransfer: DataTransfer | null): string[] {
  if (!dataTransfer) {
    return [];
  }

  const names = Array.from(dataTransfer.files ?? [])
    .map((file) => file.name?.trim())
    .filter((value): value is string => Boolean(value));

  return Array.from(new Set(names));
}

function appendDroppedEntries(entries: string[]): void {
  if (!entries.length) {
    return;
  }

  const currentDraft = composerDraft.value;
  const separator = currentDraft && !currentDraft.endsWith('\n') ? '\n' : '';
  const prefix = currentDraft ? `${currentDraft}${separator}` : '';
  updateComposerDraft(`${prefix}${entries.join('\n')}`);
}

function handleComposerDragEnter(event: DragEvent): void {
  if (!isFileDrag(event.dataTransfer)) {
    return;
  }

  // #region debug-point A:drag-enter
  reportDebugEvent('A', 'dragenter accepted by composer', {
    types: Array.from(event.dataTransfer?.types ?? []),
    fileCount: event.dataTransfer?.files.length ?? 0,
  });
  // #endregion
  event.preventDefault();
  fileDragActive.value = true;
}

function handleComposerDragOver(event: DragEvent): void {
  if (!isFileDrag(event.dataTransfer)) {
    return;
  }

  // #region debug-point A:drag-over
  reportDebugEvent('A', 'dragover accepted by composer', {
    types: Array.from(event.dataTransfer?.types ?? []),
    fileCount: event.dataTransfer?.files.length ?? 0,
  });
  // #endregion
  event.preventDefault();
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy';
  }
  fileDragActive.value = true;
}

function handleComposerDragLeave(event: DragEvent): void {
  const nextTarget = event.relatedTarget;
  if (
    nextTarget instanceof Node
    && chatRef.value?.contains(nextTarget)
  ) {
    return;
  }

  // #region debug-point D:drag-leave
  reportDebugEvent('D', 'dragleave resets overlay state', {
    hasRelatedTarget: nextTarget instanceof Node,
    overlayWasActive: fileDragActive.value,
  });
  // #endregion
  fileDragActive.value = false;
}

function handleComposerDrop(event: DragEvent): void {
  if (!isFileDrag(event.dataTransfer)) {
    return;
  }

  const plainText = event.dataTransfer?.getData('text/plain') ?? '';
  const uriList = event.dataTransfer?.getData('text/uri-list') ?? '';
  const itemSummaries = Array.from(event.dataTransfer?.items ?? []).map((item) => ({
    kind: item.kind,
    type: item.type,
  }));
  const fileSummaries = Array.from(event.dataTransfer?.files ?? []).map((file) => {
    const fileWithPath = file as File & { path?: string };
    return {
      name: file.name,
      type: file.type,
      size: file.size,
      webkitRelativePath: file.webkitRelativePath,
      path: fileWithPath.path ?? null,
    };
  });

  // #region debug-point A:drop-received
  reportDebugEvent('A', 'drop received by composer', {
    types: Array.from(event.dataTransfer?.types ?? []),
    fileCount: event.dataTransfer?.files.length ?? 0,
    overlayWasActive: fileDragActive.value,
  });
  // #endregion
  event.preventDefault();
  fileDragActive.value = false;
  const extractedPaths = extractDroppedFilePaths(event.dataTransfer);
  const extractedNames = extractDroppedFileNames(event.dataTransfer);
  // #region debug-point B:drop-paths
  reportDebugEvent('B', 'paths extracted from drop payload', {
    fileCount: event.dataTransfer?.files.length ?? 0,
    extractedPathCount: extractedPaths.length,
    extractedPaths,
    extractedNameCount: extractedNames.length,
    extractedNames,
    plainText,
    uriList,
    itemSummaries,
    fileSummaries,
  });
  // #endregion

  if (extractedPaths.length > 0) {
    appendDroppedEntries(extractedPaths);
    reportDebugEvent('B', 'drop appended absolute paths', {
      appendedCount: extractedPaths.length,
      appendedEntries: extractedPaths,
    }, 'post-fix');
    return;
  }

  if (extractedNames.length > 0) {
    appendDroppedEntries(extractedNames);
    showGlobalSuccessToast(t('chat.fileDropNameFallback'));
    reportDebugEvent('B', 'drop fell back to file names', {
      appendedCount: extractedNames.length,
      appendedEntries: extractedNames,
    }, 'post-fix');
  }
}

async function bindDesktopDragDropListener(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  removeDesktopDragDropListener?.();
  removeDesktopDragDropListener = await listenDesktopDragDropEvent((event) => {
    switch (event.type) {
      case 'enter':
      case 'over':
        fileDragActive.value = true;
        reportDebugEvent('E', 'desktop drag event entered composer window', {
          type: event.type,
          paths: 'paths' in event ? event.paths : [],
        });
        break;
      case 'leave':
        fileDragActive.value = false;
        reportDebugEvent('E', 'desktop drag event left composer window', {
          type: event.type,
        });
        break;
      case 'drop': {
        fileDragActive.value = false;
        const extractedPaths = Array.from(new Set((event.paths ?? []).map((value) => value.trim()).filter(Boolean)));
        reportDebugEvent('A', 'desktop drag drop paths received', {
          type: event.type,
          extractedPathCount: extractedPaths.length,
          extractedPaths,
        });
        if (extractedPaths.length > 0) {
          appendDroppedEntries(extractedPaths);
        }
        break;
      }
      default:
        break;
    }
  });
}

onMounted(async () => {
  restoreComposerHeightRatio();
  await nextTick();
  refreshLayoutMetrics();
  bindLayoutResizeObserver();
  await bindDesktopDragDropListener();
});

onBeforeUnmount(() => {
  removeDesktopDragDropListener?.();
  removeDesktopDragDropListener = null;
  stopComposerResize?.();
  stopComposerResize = null;
  pendingComposerBottomAnchorDistance = null;
  pendingComposerBottomAnchorSync = false;
  fileDragActive.value = false;
  layoutResizeObserver?.disconnect();
  layoutResizeObserver = null;
  resetComposerDragState();
});
</script>

<template>
  <section
    ref="chatRef"
    class="chat panel"
    :class="{ 'has-banner': hasBanner, 'no-banner': !hasBanner }"
    :style="chatLayoutStyle"
  >
    <div ref="chatHeadRef" class="chat-head">
      <div class="chat-head-title">
        <h2>{{ currentRoom ? i18nText(currentRoom.i18n, 'display_name', currentRoom.room_name) : t('chat.noRoom') }}</h2>
      </div>
      <div class="chat-side-info">
        <template v-if="currentRoom">
          <span
            class="chat-head-pill"
            :class="isScheduling ? 'chat-head-pill-scheduling' : 'chat-head-pill-idle'"
            :data-tooltip="isScheduling && currentSpeaker ? t('chat.waitingSpeaker', { name: currentSpeaker }) : ''"
          >
            {{ isScheduling ? t('chat.active') : t('chat.idle') }}
          </span>
        </template>
        <button
          type="button"
          class="chat-members-button"
          :disabled="!currentRoom"
          @click="toggleMembers"
        >
          {{ t('chat.membersLabel', { count: currentMembers.length }) }}
        </button>
        <button
          type="button"
          class="chat-members-button chat-settings-button"
          :disabled="!currentRoom || isDeptRoom"
          @click="openRoomSettings"
        >
          <i class="fa-solid fa-gear" aria-hidden="true"></i>
          <span>{{ t('roomSettings.openButton') }}</span>
        </button>
      </div>
    </div>

    <div v-if="errorMessage" ref="chatBannerRef" class="banner error">{{ errorMessage }}</div>
    <div v-else-if="reloadingMessages" ref="chatBannerRef" class="banner">{{ t('chat.loadingMessages') }}</div>

    <div ref="messageViewportRef" class="message-viewport">
      <MessageStream
        :messages="messages"
        :member-profiles="memberProfiles"
        :working-agent="workingAgent"
        :has-more-history="hasMoreHistory"
        :loading-older-messages="loadingOlderMessages"
        :escalating-message-ids="escalatingMessageIds"
        @click-agent="emit('clickAgent', $event)"
        @click-working-agent="emit('clickWorkingAgent', $event)"
        @load-older-messages="emit('loadOlderMessages')"
        @escalate-message="emit('escalateMessage', $event)"
      />
    </div>

    <form
      v-if="currentRoom && !composerNotice"
      class="composer active"
      :class="{ 'composer-file-drag-active': fileDragActive }"
      :style="composerStyle"
      @submit.prevent="handleComposerSubmit"
      @dragenter="handleComposerDragEnter"
      @dragover="handleComposerDragOver"
      @dragleave="handleComposerDragLeave"
      @drop="handleComposerDrop"
    >
      <button
        type="button"
        class="composer-drag-zone"
        :class="{ dragging: composerDividerDragging }"
        aria-label="调整消息区和输入框高度"
        @pointerdown="startComposerResize"
      >
        <span class="composer-drag-zone__grip"></span>
      </button>
      <div class="composer-editor">
        <div v-if="fileDragActive" class="composer-file-drop-overlay">
          {{ t('chat.fileDropHint') }}
        </div>
        <textarea
          :value="composerDraft"
          :placeholder="t('chat.inputPlaceholder')"
          rows="2"
          @input="updateComposerDraft(($event.target as HTMLTextAreaElement).value)"
          @compositionstart="isDraftComposing = true"
          @compositionend="isDraftComposing = false"
          @keydown.enter.exact="handleEnterKey"
        ></textarea>
        <div class="composer-foot">
          <span>{{ t('chat.sendHint') }}</span>
          <button type="submit" class="composer-submit" :disabled="!composerDraft.trim()">{{ t('chat.send') }}</button>
        </div>
      </div>
    </form>

    <div v-else-if="composerNotice" class="composer-hint">{{ composerNotice }}</div>

    <Teleport to="body">
      <div v-if="membersOpen" class="chat-members-modal" @click.self="closeMembers">
        <section class="chat-members-dialog">
          <div class="chat-members-dialog__head">
            <div>
              <p class="chat-members-dialog__eyebrow">Room Members</p>
              <h3>{{ t('chat.roomMembers') }}</h3>
            </div>
            <div class="chat-members-dialog__actions">
              <span>{{ t('room.membersCount', { count: currentMembers.length }) }}</span>
              <button type="button" class="chat-members-dialog__close" @click="closeMembers">{{ t('common.close') }}</button>
            </div>
          </div>

          <div v-if="currentMembers.length" class="chat-members-grid">
            <article v-for="member in currentMembers" :key="member.name" class="chat-member-card">
              <span
                v-if="member.employee_number !== null && member.employee_number >= 0"
                class="chat-member-card__employee"
              >#{{ member.employee_number }}</span>
              <div class="chat-member-card__avatar-wrap">
                <span v-if="member.is_leader" class="chat-member-card__leader-flag">Leader</span>
                <img class="chat-member-card__avatar" :src="getAgentAvatarUrl(member.name)" :alt="`${displayName(member)} avatar`" />
              </div>
              <strong>{{ displayName(member) }}</strong>
              <span v-if="member.role_template_name" class="chat-member-card__meta">{{ member.role_template_name }}</span>
            </article>
          </div>
          <p v-else class="chat-members-empty">{{ t('chat.noMembers') }}</p>
        </section>
      </div>
    </Teleport>
  </section>
</template>

<style scoped>
.chat {
  display: grid;
  gap: 0;
  padding: 14px 14px 12px;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: color-mix(in srgb, var(--surface-chat) 94%, white 6%);
  border-color: color-mix(in srgb, var(--panel-border-soft) 82%, transparent);
  box-shadow: var(--shadow-soft);
}

.chat-head {
  display: flex;
  justify-content: space-between;
  gap: 14px;
  align-items: center;
  padding: 4px 4px 14px;
  border-bottom: 1px solid var(--border-subtle);
}

.chat-head h2 {
  margin: 0;
  font-family: 'IBM Plex Sans', 'Noto Sans SC', sans-serif;
  font-size: 1.02rem;
  font-weight: 600;
  letter-spacing: 0;
  color: var(--text-primary);
}

.chat-head-title {
  display: flex;
  align-items: center;
}

.chat-side-info {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  justify-content: flex-end;
  align-items: center;
  margin-left: auto;
}

.chat-head-pill,
.chat-members-button {
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--surface-panel-muted) 84%, var(--surface-panel) 16%);
  color: var(--text-secondary);
  font-size: 0.76rem;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  white-space: nowrap;
}

.chat-settings-button {
  gap: 6px;
}

.chat-settings-button i {
  font-size: 0.78rem;
}

.chat-head-pill {
  position: relative;
  min-height: 30px;
  padding: 5px 11px 3px;
  border-radius: 999px;
  font-weight: 600;
}

.chat-head-pill-scheduling {
  color: var(--state-success);
  border-color: color-mix(in srgb, var(--state-success) 35%, var(--border-default) 65%);
  background: color-mix(in srgb, var(--state-success) 14%, var(--surface-panel) 86%);
}

.chat-head-pill-idle {
  color: var(--text-secondary);
}

.chat-head-pill[data-tooltip]:not([data-tooltip=''])::after {
  content: attr(data-tooltip);
  position: absolute;
  right: 0;
  top: calc(100% + 8px);
  padding: 6px 10px;
  border-radius: 8px;
  background: var(--surface-overlay);
  border: 1px solid var(--border-default);
  box-shadow: 0 10px 24px rgba(15, 23, 42, 0.16);
  color: var(--text-primary);
  font-size: 0.72rem;
  font-weight: 500;
  line-height: 1.2;
  white-space: nowrap;
  opacity: 0;
  transform: translateY(-4px);
  pointer-events: none;
  transition:
    opacity 140ms ease,
    transform 140ms ease;
}

.chat-head-pill[data-tooltip]:not([data-tooltip='']):hover::after {
  opacity: 1;
  transform: translateY(0);
}

.chat-members-button {
  min-height: 30px;
  padding: 5px 11px 3px;
  border-radius: 999px;
  cursor: pointer;
  transition:
    border-color 140ms ease,
    background 140ms ease,
    color 140ms ease;
}

.chat-members-button:hover:not(:disabled) {
  border-color: var(--interactive-focus-border);
  color: var(--text-primary);
  background: color-mix(in srgb, var(--interactive-selected) 72%, var(--surface-panel-muted) 28%);
}

.chat-members-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.chat-members-modal {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(6, 10, 16, 0.42);
  backdrop-filter: blur(6px);
}

.chat-members-dialog {
  width: min(860px, 100%);
  max-height: min(720px, calc(100vh - 48px));
  overflow: auto;
  padding: 18px;
  border: 1px solid color-mix(in srgb, var(--interactive-focus-border) 20%, var(--border-default) 80%);
  border-radius: 22px;
  background: var(--surface-overlay);
  box-shadow: 0 18px 40px rgba(15, 23, 42, 0.14);
}

.chat-members-dialog__head,
.chat-members-dialog__actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.chat-members-dialog__head {
  margin-bottom: 16px;
}

.chat-members-dialog__eyebrow {
  margin: 0 0 4px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.14em;
  font-size: 0.68rem;
}

.chat-members-dialog__head h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 1.15rem;
}

.chat-members-dialog__actions span,
.chat-members-empty {
  color: var(--text-secondary);
  font-size: 0.74rem;
}

.chat-members-dialog__close {
  border: 1px solid var(--border-default);
  border-radius: 999px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 0.76rem;
  line-height: 1;
  padding: 7px 12px;
  cursor: pointer;
}

.chat-members-dialog__close:hover {
  border-color: var(--interactive-focus-border);
  color: var(--text-primary);
}

.chat-members-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 12px;
}

.chat-member-card {
  position: relative;
  padding: 12px 8px 10px;
  border: 1px solid color-mix(in srgb, var(--interactive-focus-border) 10%, var(--border-default) 90%);
  border-radius: 16px;
  background: color-mix(in srgb, var(--surface-panel-muted) 90%, var(--surface-panel) 10%);
  display: grid;
  justify-items: center;
  gap: 8px;
  text-align: center;
}

.chat-member-card__employee {
  position: absolute;
  top: 8px;
  left: 8px;
  color: var(--text-secondary);
  font-size: 0.8rem;
  line-height: 1;
  letter-spacing: 0.04em;
}

.chat-member-card__avatar-wrap {
  position: relative;
  padding-top: 14px;
}

.chat-member-card__leader-flag {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translate(-50%, -30%);
  color: color-mix(in srgb, var(--state-info) 72%, var(--text-primary) 28%);
  font-size: 0.72rem;
  line-height: 1;
  font-weight: 700;
  letter-spacing: 0.03em;
  white-space: nowrap;
}

.chat-member-card__avatar {
  width: 56px;
  height: 56px;
  border-radius: 16px;
  border: 1px solid color-mix(in srgb, var(--interactive-focus-border) 22%, var(--border-default) 78%);
  object-fit: cover;
  background: color-mix(in srgb, var(--surface-panel-muted) 76%, var(--surface-panel) 24%);
}

.chat-member-card strong {
  color: var(--text-primary);
  font-size: 0.78rem;
  line-height: 1.25;
  word-break: break-word;
}

.chat-member-card__meta {
  color: var(--text-secondary);
  font-size: 0.76rem;
  line-height: 1.25;
  word-break: break-word;
}

.chat-members-empty {
  margin: 0;
}

.banner {
  margin-top: 10px;
  border-radius: 12px;
  padding: 9px 11px;
  background: color-mix(in srgb, var(--surface-panel-muted) 90%, var(--surface-panel) 10%);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 76%, transparent);
  font-size: 0.78rem;
}

.banner.error {
  background: var(--banner-error-bg);
  color: var(--banner-error-text);
}

.message-viewport {
  min-height: 0;
  overflow: hidden;
  margin-top: 6px;
}

.composer-drag-zone {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  height: var(--composer-drag-zone-height, 8px);
  padding: 0;
  border: none;
  background: transparent;
  cursor: row-resize;
  touch-action: none;
}

.composer-drag-zone__grip {
  width: 100%;
  height: 1px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--border-default) 72%, transparent);
  transition:
    background 0.18s ease,
    transform 0.18s ease;
}

.composer-drag-zone:hover .composer-drag-zone__grip,
.composer-drag-zone.dragging .composer-drag-zone__grip {
  background: color-mix(in srgb, var(--interactive-focus-border) 48%, var(--border-default) 52%);
}

.composer-drag-zone.dragging .composer-drag-zone__grip {
  transform: scaleY(1.4);
}

.composer {
  box-sizing: border-box;
  background: transparent;
  padding: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.composer-editor {
  position: relative;
  background: color-mix(in srgb, var(--surface-input) 96%, white 4%);
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-height: 0;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 78%, var(--border-default) 22%);
  border-radius: 18px;
  overflow: hidden;
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 10%, transparent),
    0 10px 24px rgba(0, 0, 0, 0.08);
  transition:
    border-color 160ms ease,
    box-shadow 160ms ease;
}

.composer-editor:focus-within {
  border-color: var(--input-focus-border);
  box-shadow: 0 0 0 2px var(--input-focus-ring);
}

.composer-file-drag-active .composer-editor {
  border-color: var(--interactive-focus-border);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--interactive-selected) 30%, transparent);
}

.composer-file-drop-overlay {
  position: absolute;
  inset: 0;
  z-index: 1;
  display: grid;
  place-items: center;
  padding: 18px;
  border-radius: 18px;
  background: color-mix(in srgb, var(--surface-panel) 88%, var(--interactive-selected) 12%);
  color: var(--text-primary);
  font-size: 0.82rem;
  font-weight: 600;
  text-align: center;
  pointer-events: none;
}

.composer textarea {
  width: 100%;
  resize: none;
  flex: 1 1 auto;
  min-height: 40px;
  border: none;
  border-radius: 0;
  padding: 14px 14px 10px;
  color: var(--text-primary);
  background: transparent;
  outline: none;
  font-size: 0.84rem;
  line-height: 1.45;
  display: block;
}

.composer textarea::placeholder {
  color: var(--text-secondary);
}

.composer textarea:focus {
  box-shadow: none;
}

.composer textarea:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.composer-foot {
  position: relative;
  display: block;
  flex-shrink: 0;
  margin-top: 0;
  padding: 10px 76px 12px 14px;
  font-size: 0.74rem;
  background: linear-gradient(180deg, transparent 0%, color-mix(in srgb, var(--surface-panel) 10%, transparent) 100%);
}

.composer-foot span {
  display: block;
  color: var(--text-secondary);
  line-height: 1;
}

.composer-toggle {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: 5px;
  font-size: 0.74rem;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
}

.composer-toggle input {
  display: none;
}

.composer-toggle.active {
  color: #f59e0b;
}

.composer-hint {
  background: transparent;
  border-top: 1px solid var(--border-subtle);
  color: var(--text-tertiary);
  text-align: center;
  padding: 3px 8px;
  font-size: 0.74rem;
}

.composer-submit {
  position: absolute;
  right: 12px;
  bottom: 10px;
  border: 0;
  border-radius: 999px;
  padding: 8px 13px;
  background: color-mix(in srgb, var(--state-info) 68%, var(--surface-panel) 32%);
  color: var(--text-on-accent);
  font-weight: 700;
  cursor: pointer;
  font-size: 0.74rem;
  box-shadow: 0 8px 18px color-mix(in srgb, var(--state-info) 18%, transparent);
}

.composer-submit:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

:global(html.bp-layout-narrow) .chat-head,
:global(html.bp-layout-narrow) .composer-foot {
  flex-direction: column;
  align-items: flex-start;
}

:global(html.bp-layout-narrow) .chat-members-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

:global(html.bp-compact) .chat {
  padding: 10px 6px 8px;
  border-radius: 16px;
}

:global(html.bp-compact) .chat-head {
  gap: 8px;
  padding-bottom: 10px;
}

:global(html.bp-compact) .chat-head h2 {
  font-size: 1.06rem;
  line-height: 1.2;
}

:global(html.bp-compact) .chat-side-info {
  width: 100%;
  justify-content: flex-start;
  gap: 8px;
}

:global(html.bp-compact) .chat-head-pill,
:global(html.bp-compact) .chat-members-button {
  min-height: 32px;
  padding: 6px 12px 4px;
  font-size: 0.78rem;
}

:global(html.bp-compact) .banner {
  font-size: 0.76rem;
}

:global(html.bp-compact) .composer {
  --composer-drag-zone-height: 10px;
}

:global(html.bp-compact) .composer textarea {
  min-height: 40px;
  padding: 12px 12px 10px;
  font-size: 0.86rem;
  line-height: 1.45;
}

:global(html.bp-compact) .composer-foot {
  min-height: 58px;
  padding: 10px 76px 11px 12px;
}

:global(html.bp-compact) .composer-foot span {
  line-height: 1.35;
}

:global(html.bp-compact) .composer-submit {
  right: 10px;
  bottom: 10px;
  min-width: 56px;
  min-height: 36px;
  font-size: 0.76rem;
}

:global(html.bp-compact) .composer-hint {
  padding: 8px 10px calc(8px + env(safe-area-inset-bottom, 0px));
  line-height: 1.4;
}

:global(html.bp-compact) .chat-members-modal {
  padding: 14px;
}

:global(html.bp-compact) .chat-members-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

:global(html.bp-console-short) .chat {
  padding: 6px 5px 5px;
  border-radius: 12px;
}

:global(html.bp-console-short) .chat-head {
  gap: 4px;
  padding: 0 2px 4px;
}

:global(html.bp-console-short) .chat-head h2 {
  font-size: 0.92rem;
  line-height: 1.2;
}

:global(html.bp-console-short) .chat-side-info {
  gap: 4px;
}

:global(html.bp-console-short) .chat-head-pill,
:global(html.bp-console-short) .chat-members-button {
  min-height: 24px;
  padding: 3px 8px 2px;
  font-size: 0.68rem;
}

:global(html.bp-console-short) .banner {
  padding: 4px 6px;
  font-size: 0.68rem;
}

:global(html.bp-console-short) .chat.has-banner .banner {
  margin-top: 4px;
}

:global(html.bp-console-short) .chat.no-banner .message-viewport {
  margin-top: 0;
}

:global(html.bp-console-short) .composer {
  --composer-drag-zone-height: 4px;
}

:global(html.bp-console-short) .composer-editor {
  border-radius: 8px;
}

:global(html.bp-console-short) .composer textarea {
  min-height: 32px;
  padding: 8px 8px 6px;
  font-size: 0.76rem;
  line-height: 1.3;
}

:global(html.bp-console-short) .composer-foot {
  min-height: 34px;
  padding: 6px 54px 6px 8px;
  font-size: 0.64rem;
}

:global(html.bp-console-short) .composer-foot span {
  line-height: 1.2;
}

:global(html.bp-console-short) .composer-submit {
  right: 6px;
  bottom: 5px;
  min-width: 44px;
  min-height: 24px;
  font-size: 0.66rem;
}

:global(html.bp-console-short) .composer-hint {
  padding: 4px 6px;
  font-size: 0.66rem;
}
</style>
