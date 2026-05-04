<script setup lang="ts">
import { getAgentAvatarUrl } from '../../avatar';
import { nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageInfo, RoomMemberProfile } from '../../types';
import { bubbleSide, displayName, formatTime } from '../../utils';

const props = defineProps<{
  messages: MessageInfo[];
  memberProfiles: RoomMemberProfile[];
  workingAgent?: RoomMemberProfile | null;
}>();

const emit = defineEmits<{
  clickWorkingAgent: [agentId: number];
}>();

const { t } = useI18n();

const streamRef = useTemplateRef('streamRef');
const hasScrollbar = ref(false);
let resizeObserver: ResizeObserver | null = null;

const NAME_COLORS = [
  '#56d4b0',
  '#7eb8d4',
  '#c4a55a',
  '#d4847e',
  '#b392d4',
  '#8cc152',
  '#4fc1e9',
  '#ffce54',
  '#fc6e51',
  '#ac92ec',
  '#e8a838',
  '#a0d468',
  '#5d9cec',
  '#ed5565',
  '#fb6e52',
];

function senderColor(sender: string): string {
  if (sender.toUpperCase() === 'OPERATOR') {
    return '#7f91a4';
  }
  const hash = Array.from(sender).reduce((sum, char) => sum + char.charCodeAt(0), 0);
  return NAME_COLORS[hash % NAME_COLORS.length];
}

function resolveSenderProfile(senderId: number): RoomMemberProfile | null {
  return props.memberProfiles.find((member) => member.id === senderId) ?? null;
}

function resolveSenderStableName(senderId: number): string {
  if (senderId === -1) {
    return 'OPERATOR';
  }
  if (senderId === -2) {
    return 'SYSTEM';
  }
  return resolveSenderProfile(senderId)?.name ?? String(senderId);
}

function resolveSenderDisplayName(senderId: number): string {
  if (senderId === -1) {
    return 'OPERATOR';
  }
  if (senderId === -2) {
    return 'SYSTEM';
  }
  const profile = resolveSenderProfile(senderId);
  return profile ? displayName(profile) : String(senderId);
}

type MessageStatus = 'published' | 'immediate' | 'pending-immediate' | 'queued';

function resolveMessageStatus(message: MessageInfo): MessageStatus {
  if (message.seq !== null) {
    return message.insert_immediately ? 'immediate' : 'published';
  }
  return message.insert_immediately ? 'pending-immediate' : 'queued';
}

function updateScrollbarState(): void {
  const stream = streamRef.value;
  if (!stream) {
    hasScrollbar.value = false;
    return;
  }

  hasScrollbar.value = stream.scrollHeight - stream.clientHeight > 1;
}

function isAtBottom(): boolean {
  const stream = streamRef.value;
  if (!stream) return true;
  return stream.scrollTop + stream.clientHeight >= stream.scrollHeight - 2;
}

function scrollToBottom(): void {
  const stream = streamRef.value;
  if (stream) stream.scrollTop = stream.scrollHeight;
}

watch(
  () => props.messages,
  async () => {
    const shouldScroll = isAtBottom();
    await nextTick();
    updateScrollbarState();
    if (shouldScroll) scrollToBottom();
  },
  { deep: true },
);

watch(
  () => props.workingAgent,
  async () => {
    const shouldScroll = isAtBottom();
    await nextTick();
    updateScrollbarState();
    if (shouldScroll) scrollToBottom();
  },
);

onMounted(() => {
  updateScrollbarState();
  scrollToBottom();
  if (typeof ResizeObserver === 'undefined' || !streamRef.value) {
    return;
  }

  resizeObserver = new ResizeObserver(() => {
    updateScrollbarState();
  });
  resizeObserver.observe(streamRef.value);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<template>
  <div ref="streamRef" class="message-stream" :class="{ 'has-scrollbar': hasScrollbar }">
    <div
      v-for="(message, index) in messages"
      :key="message.db_id ?? `${message.time}-${message.sender_id}-${index}`"
      class="message-row"
      :class="`side-${bubbleSide(message.sender_id)}`"
    >
      <template v-if="bubbleSide(message.sender_id) === 'center'">
        <div class="system-note">{{ message.content }}</div>
      </template>
      <template v-else>
        <div class="message-meta">
          <template v-if="bubbleSide(message.sender_id) === 'left'">
            <img
              class="sender-avatar"
              :src="getAgentAvatarUrl(resolveSenderStableName(message.sender_id))"
              :alt="`${resolveSenderDisplayName(message.sender_id)} avatar`"
            />
            <span class="sender" :style="{ color: senderColor(resolveSenderStableName(message.sender_id)) }">
              {{ resolveSenderDisplayName(message.sender_id) }}
            </span>
          </template>
          <span class="time">{{ formatTime(message.time) }}</span>
          <span
            v-if="resolveMessageStatus(message) === 'queued'"
            class="msg-status msg-status--queued"
            title="消息排队中，等待 Agent 回复后注入"
          >⏳ 排队中</span>
          <span
            v-else-if="resolveMessageStatus(message) === 'pending-immediate'"
            class="msg-status msg-status--pending-immediate"
            title="立即注入，等待发布"
          >⚡ 等待注入</span>
          <span
            v-else-if="resolveMessageStatus(message) === 'immediate'"
            class="msg-status msg-status--immediate"
            title="已立即注入"
          >⚡</span>
          <template v-if="bubbleSide(message.sender_id) === 'right'">
            <span class="sender" :style="{ color: senderColor(resolveSenderStableName(message.sender_id)) }">
              {{ resolveSenderDisplayName(message.sender_id) }}
            </span>
            <img
              class="sender-avatar"
              :src="getAgentAvatarUrl(resolveSenderStableName(message.sender_id))"
              :alt="`${resolveSenderDisplayName(message.sender_id)} avatar`"
            />
          </template>
        </div>
        <div class="bubble" :class="{ 'bubble--pending': message.seq === null }">{{ message.content }}</div>
      </template>
    </div>

    <div
      v-if="workingAgent"
      class="working-indicator working-indicator--clickable"
      role="button"
      tabindex="0"
      @click="emit('clickWorkingAgent', workingAgent.id)"
      @keydown.enter="emit('clickWorkingAgent', workingAgent.id)"
    >
      <img
        class="working-indicator-avatar"
        :src="getAgentAvatarUrl(workingAgent.name)"
        :alt="`${displayName(workingAgent)} avatar`"
      />
      <span class="working-indicator-text">{{ t('chat.processing', { name: displayName(workingAgent) }) }}</span>
      <span class="working-indicator-dots">
        <span class="dot"></span>
        <span class="dot"></span>
        <span class="dot"></span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.message-stream {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 0;
  display: flex;
  flex-direction: column;
  gap: 14px;
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}

.message-stream::-webkit-scrollbar {
  width: 10px;
}

.message-stream::-webkit-scrollbar-track {
  background: var(--scrollbar-track);
  border-radius: 999px;
}

.message-stream::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 999px;
  border: 2px solid var(--scrollbar-track);
}

.message-stream::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}

.message-row {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.message-row.side-left {
  align-items: flex-start;
}

.message-row.side-right {
  align-items: flex-end;
}

.message-stream.has-scrollbar .message-row.side-right {
  padding-right: 6px;
}

.message-row.side-center {
  align-items: center;
  padding: 2px 0 4px;
}

.message-meta {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  color: var(--text-secondary);
  font-size: 0.72rem;
  padding: 0 6px;
}

.sender {
  font-weight: 600;
  font-size: 0.84rem;
  line-height: 1;
}

.sender-avatar {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
  border: 1px solid color-mix(in srgb, var(--border-strong) 30%, transparent);
  background: color-mix(in srgb, var(--surface-elevated) 84%, var(--border-default) 16%);
}

.time {
  color: var(--text-tertiary);
}

.badge-immediate,
.msg-status {
  font-size: 0.72rem;
  line-height: 1;
}

.msg-status--immediate {
  color: #f59e0b;
}

.msg-status--queued {
  color: var(--text-tertiary);
  font-style: italic;
}

.msg-status--pending-immediate {
  color: #f59e0b;
  font-style: italic;
}

.bubble,
.system-note {
  max-width: min(80%, 820px);
  border-radius: 6px;
  padding: 10px 14px;
  line-height: 1.55;
  word-break: break-word;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
  font-size: 0.82rem;
}

.bubble {
  background: var(--bubble-left);
  color: var(--bubble-left-text, inherit);
  border: 1px solid color-mix(in srgb, var(--border-default) 18%, transparent);
  box-shadow: var(--bubble-shadow, none);
  transition: opacity 0.25s ease;
}

.bubble--pending {
  opacity: 0.55;
}

.side-right .bubble {
  background: var(--bubble-right);
  color: var(--bubble-right-text);
  border: 1px solid color-mix(in srgb, var(--interactive-focus-border) 22%, transparent);
  box-shadow: var(--bubble-right-shadow, none);
}

.system-note {
  text-align: center;
  color: color-mix(in srgb, var(--text-secondary) 78%, var(--text-primary) 22%);
  background: transparent;
  padding: 0;
  max-width: min(72%, 760px);
  line-height: 1.5;
  font-size: 0.76rem;
  letter-spacing: 0.01em;
}

:global(html.bp-layout-narrow) .bubble,
:global(html.bp-layout-narrow) .system-note {
  max-width: 100%;
}

:global(html.bp-compact) .message-stream {
  gap: 12px;
  padding: 6px 0 10px;
}

:global(html.bp-compact) .message-meta {
  gap: 5px;
  padding: 0 2px;
  font-size: 0.7rem;
  flex-wrap: wrap;
}

:global(html.bp-compact) .message-stream.has-scrollbar .message-row.side-right {
  padding-right: 0;
}

:global(html.bp-compact) .sender {
  font-size: 0.8rem;
}

:global(html.bp-compact) .sender-avatar {
  width: 32px;
  height: 32px;
  border-radius: 9px;
}

:global(html.bp-compact) .bubble {
  max-width: min(92%, 640px);
  padding: 10px 12px;
  font-size: 0.8rem;
  line-height: 1.5;
}

:global(html.bp-compact) .system-note {
  max-width: 100%;
  font-size: 0.72rem;
  line-height: 1.45;
}

:global(html.bp-compact) .working-indicator {
  padding: 10px 8px;
  font-size: 0.76rem;
}

.working-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  color: var(--text-secondary);
  font-size: 0.78rem;
  animation: fade-in 0.25s ease-out;
}

.working-indicator--clickable {
  cursor: pointer;
  border-radius: 8px;
  transition: background 0.15s ease;
}

.working-indicator--clickable:hover {
  background: color-mix(in srgb, var(--border-default) 20%, transparent);
}

.working-indicator-avatar {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  object-fit: cover;
  flex-shrink: 0;
  border: 1px solid color-mix(in srgb, var(--border-strong) 30%, transparent);
  background: color-mix(in srgb, var(--surface-elevated) 84%, var(--border-default) 16%);
}

.working-indicator-text {
  color: var(--text-primary);
  font-weight: 500;
}

.working-indicator-dots {
  display: inline-flex;
  gap: 3px;
  align-items: center;
}

.working-indicator-dots .dot {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--text-secondary);
  animation: dot-pulse 1.4s infinite ease-in-out;
}

.working-indicator-dots .dot:nth-child(2) {
  animation-delay: 0.2s;
}

.working-indicator-dots .dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes dot-pulse {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1); }
}

@keyframes fade-in {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
