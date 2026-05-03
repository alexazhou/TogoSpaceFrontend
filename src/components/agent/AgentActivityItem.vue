<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { getTeamRooms } from '../../realtime/runtimeStore';
import type { AgentActivity, AgentActivityStatus } from '../../types';

const { t } = useI18n();

const props = defineProps<{
  activity: AgentActivity;
}>();

const toolName = computed(() => {
  const metadataToolName = props.activity.metadata?.tool_name;
  if (typeof metadataToolName === 'string' && metadataToolName.trim()) {
    return metadataToolName.trim();
  }
  const detail = props.activity.detail.trim();
  return props.activity.activity_type === 'tool_call' ? detail : '';
});

const roomName = computed(() => {
  const metadataRoomName = props.activity.metadata?.room_name;
  if (typeof metadataRoomName === 'string' && metadataRoomName.trim()) {
    return metadataRoomName.trim();
  }
  const roomId = props.activity.metadata?.room_id;
  if (typeof roomId !== 'number') {
    return '';
  }
  const room = getTeamRooms(props.activity.team_id).find((item) => item.room_id === roomId);
  return room?.room_name ?? '';
});

function activityStatusLabel(status: AgentActivityStatus): string {
  if (status === 'started') {
    return t('agent.activityState.running');
  }
  if (status === 'succeeded') {
    return t('agent.activityState.completed');
  }
  if (status === 'failed') {
    return t('agent.activityState.failed');
  }
  return t('agent.activityState.cancelled');
}

function activityStatusSymbol(status: AgentActivityStatus): string {
  if (status === 'started') {
    return '●';
  }
  if (status === 'succeeded') {
    return '✓';
  }
  return '×';
}

function shouldShowToolName(activity: AgentActivity): boolean {
  if (activity.activity_type !== 'tool_call') {
    return false;
  }
  return toolName.value !== 'send_chat_msg' && toolName.value !== 'finish_chat_turn' && toolName.value.length > 0;
}

function activityTitle(activity: AgentActivity): string {
  if (activity.activity_type === 'agent_state') {
    const detail = activity.detail.trim().toUpperCase();
    if (detail === 'ACTIVE') {
      return t('agent.activityType.startActivity');
    }
    if (detail === 'IDLE') {
      return t('agent.activityType.stopAction');
    }
  }

  if (activity.activity_type === 'tool_call') {
    if (toolName.value === 'send_chat_msg') {
      return t('agent.activityType.sendMessage');
    }
    if (toolName.value === 'finish_chat_turn') {
      return t('agent.activityType.finishTurn');
    }
  }

  switch (activity.activity_type) {
    case 'agent_state':
      return t('agent.activityType.agentState');
    case 'llm_infer':
      return t('agent.activityType.llmInfer');
    case 'reasoning':
      return t('agent.activityType.reasoning');
    case 'chat_reply':
      return t('agent.activityType.chatReply');
    case 'tool_call':
      return t('agent.activityType.toolCall');
    case 'compact':
      return t('agent.activityType.compact');
    case 'unknown':
      return t('agent.activityType.unknown');
    default:
      return t('agent.activityType.unknown');
  }
}

function getActivityToolCommand(activity: AgentActivity): string {
  const command = activity.metadata?.command;
  return typeof command === 'string' ? command : '';
}

function getActivityToolArguments(activity: AgentActivity): string {
  const toolArguments = activity.metadata?.tool_arguments;
  if (toolArguments == null) {
    return '';
  }
  const formatToolArguments = (value: unknown): string => {
    if (typeof value !== 'object' || value === null) {
      return t('agent.parseFailed');
    }
    if (toolName.value === 'execute_bash') {
      const command = (value as { command?: unknown }).command;
      if (typeof command === 'string' && command.trim()) {
        return command.trim();
      }
    }
    if (toolName.value === 'write_file' || toolName.value === 'read_file') {
      const filePath = (value as { file_path?: unknown }).file_path;
      if (typeof filePath === 'string' && filePath.trim()) {
        return filePath.trim();
      }
    }
    return JSON.stringify(value);
  };
  return formatToolArguments(toolArguments);
}

function getSendMessagePreview(activity: AgentActivity): string {
  const toolArguments = activity.metadata?.tool_arguments;
  if (toolArguments == null) {
    return '';
  }
  const truncatePreview = (value: string): string => {
    const normalized = value.replace(/\s+/g, ' ').trim();
    if (!normalized) {
      return '';
    }
    return normalized.length > 24 ? `${normalized.slice(0, 24)}...` : normalized;
  };
  const extractMessage = (value: unknown): string => {
    if (typeof value !== 'object' || value === null) {
      return t('agent.parseFailed');
    }
    const candidate = value as { msg?: unknown; content?: unknown; text?: unknown };
    const message = [candidate.msg, candidate.content, candidate.text].find((item) => typeof item === 'string');
    return typeof message === 'string' ? truncatePreview(message) : t('agent.parseFailed');
  };
  return extractMessage(toolArguments);
}

function getSendMessagePrefix(activity: AgentActivity): string {
  const room = roomName.value;
  if (!room) {
    return '';
  }
  return t('agent.sendToRoomPrefix', { room });
}

function activitySummary(activity: AgentActivity): string {
  if (activity.activity_type === 'tool_call') {
    if (toolName.value === 'send_chat_msg') {
      const preview = getSendMessagePreview(activity);
      return preview;
    }
    if (toolName.value === 'finish_chat_turn') {
      return '';
    }
    if (shouldShowToolName(activity) && getActivityToolArguments(activity)) {
      return '';
    }
  }

  const command = getActivityToolCommand(activity);
  if (command) {
    return command;
  }

  const detail = activity.detail.trim();
  if (activity.activity_type === 'agent_state' && (detail.toUpperCase() === 'ACTIVE' || detail.toUpperCase() === 'IDLE')) {
    return '';
  }

  if (detail && detail !== toolName.value) {
    return detail;
  }

  if (activity.error_message?.trim()) {
    return activity.error_message.trim();
  }
  return '';
}

function formatActivityTime(value: string | null | undefined): string {
  if (!value) {
    return '';
  }
  return value.replace('T', ' ').slice(0, 19);
}

function formatActivityTimeCompact(value: string | null | undefined): string {
  if (!value) {
    return '';
  }
  const normalized = formatActivityTime(value);
  return normalized.slice(-8);
}

function formatDuration(durationMs: number | null | undefined): string {
  if (durationMs == null || Number.isNaN(durationMs)) {
    return '0ms';
  }
  if (durationMs < 0) {
    return '0ms';
  }
  if (durationMs < 1000) {
    return `${durationMs}ms`;
  }
  return `${(durationMs / 1000).toFixed(durationMs >= 10_000 ? 0 : 1)}s`;
}

function activityMetaTokens(activity: AgentActivity): string {
  const metadata = activity.metadata ?? {};
  const currentTotal = typeof metadata.current_total_tokens === 'number' ? metadata.current_total_tokens : null;
  const finalTotal = typeof metadata.final_total_tokens === 'number' ? metadata.final_total_tokens : null;
  const estimated = typeof metadata.estimated_prompt_tokens === 'number' ? metadata.estimated_prompt_tokens : null;
  const currentCompletion = typeof metadata.current_completion_tokens === 'number' ? metadata.current_completion_tokens : null;
  if (finalTotal !== null) {
    return `tokens ${finalTotal}`;
  }
  if (currentTotal !== null) {
    return `tokens ${currentTotal}`;
  }
  if (estimated !== null) {
    const runningTotal = estimated + (currentCompletion ?? 0);
    return t('agent.tokenEstimate', { count: runningTotal });
  }
  return '';
}

function getActivityModel(activity: AgentActivity): string {
  const model = activity.metadata?.model;
  return typeof model === 'string' ? model : '';
}

function getActivityToolName(activity: AgentActivity): string {
  const toolName = activity.metadata?.tool_name;
  return typeof toolName === 'string' ? toolName : '';
}
</script>

<template>
  <article class="agent-activity-item" :data-status="activity.status" :data-activity-type="activity.activity_type">
    <div class="agent-activity-item__row">
      <span class="agent-activity-item__state" :data-status="activity.status">{{ activityStatusSymbol(activity.status) }}</span>
      <strong class="agent-activity-item__title">{{ activityTitle(activity) }}</strong>
      <span
        v-if="shouldShowToolName(activity)"
        class="agent-activity-item__chip agent-activity-item__chip--mono agent-activity-item__tool-name"
        :title="toolName"
      >{{ toolName }}</span>
      <span
        v-if="shouldShowToolName(activity) && getActivityToolArguments(activity)"
        class="agent-activity-item__summary agent-activity-item__summary--code agent-activity-item__tool-args"
        :title="getActivityToolArguments(activity)"
      >{{ getActivityToolArguments(activity) }}</span>
      <span
        v-if="activity.activity_type === 'llm_infer' && getActivityModel(activity)"
        class="agent-activity-item__chip agent-activity-item__chip--mono"
        :title="getActivityModel(activity)"
      >{{ getActivityModel(activity) }}</span>
      <span
        v-if="toolName === 'send_chat_msg' && getSendMessagePrefix(activity)"
        class="agent-activity-item__chip"
        :title="getSendMessagePrefix(activity)"
      >{{ getSendMessagePrefix(activity) }}</span>
      <span
        v-if="activitySummary(activity)"
        class="agent-activity-item__summary"
        :class="{ 'agent-activity-item__summary--code': !!getActivityToolCommand(activity) }"
        :title="activitySummary(activity)"
      >{{ activitySummary(activity) }}</span>
      <span
        v-if="activity.activity_type !== 'llm_infer' && getActivityModel(activity)"
        class="agent-activity-item__chip agent-activity-item__chip--mono"
        :title="getActivityModel(activity)"
      >{{ getActivityModel(activity) }}</span>
      <span
        v-if="activity.activity_type !== 'tool_call' && getActivityToolName(activity)"
        class="agent-activity-item__chip agent-activity-item__chip--mono"
        :title="getActivityToolName(activity)"
      >{{ getActivityToolName(activity) }}</span>
      <span class="agent-activity-item__tail">
        <span v-if="activityMetaTokens(activity)" class="agent-activity-item__tokens">{{ activityMetaTokens(activity) }}</span>
        <span class="agent-activity-item__status">{{ activityStatusLabel(activity.status) }}</span>
        <span class="agent-activity-item__time" :title="formatActivityTime(activity.started_at)">{{ formatActivityTimeCompact(activity.started_at) }}</span>
        <span class="agent-activity-item__duration">{{ formatDuration(activity.duration_ms) }}</span>
      </span>
    </div>
    <p v-if="activity.error_message" class="agent-activity-item__error">{{ activity.error_message }}</p>
  </article>
</template>

<style scoped>
.agent-activity-item {
  display: grid;
  gap: 3px;
  padding: 6px 8px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--panel-bg) 94%, var(--surface-soft) 6%);
  border: 1px solid color-mix(in srgb, var(--panel-border) 84%, transparent);
}

.agent-activity-item[data-status='started'] {
  background: color-mix(in srgb, var(--accent) 6%, var(--panel-bg) 94%);
  border-color: color-mix(in srgb, var(--accent) 20%, var(--panel-border) 80%);
}

.agent-activity-item[data-status='failed'] {
  border-color: color-mix(in srgb, var(--danger, #f85149) 26%, var(--panel-border) 74%);
  background: color-mix(in srgb, var(--danger, #f85149) 7%, var(--panel-bg) 93%);
}

.agent-activity-item__row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  flex-wrap: nowrap;
  overflow: hidden;
}

.agent-activity-item__state {
  flex: none;
  width: 14px;
  text-align: center;
  font-size: 0.78rem;
  font-weight: 700;
  line-height: 1;
  color: var(--muted);
}

.agent-activity-item__state[data-status='started'] {
  color: var(--good);
  animation: agent-activity-item-dot-pulse 2s ease-in-out infinite;
}

.agent-activity-item__state[data-status='succeeded'] {
  color: var(--good);
}

.agent-activity-item__state[data-status='failed'],
.agent-activity-item__state[data-status='cancelled'] {
  color: var(--danger, #f85149);
}

.agent-activity-item__title {
  flex: none;
  color: var(--text-strong);
  font-size: 0.82rem;
  line-height: 1.2;
}

.agent-activity-item__row span {
  color: var(--muted);
  font-size: 0.7rem;
  line-height: 1.2;
}

.agent-activity-item__chip {
  flex: none;
  display: inline-flex;
  align-items: center;
  max-width: 180px;
  min-width: 0;
  height: 18px;
  padding: 0 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface-soft) 74%, transparent);
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.agent-activity-item__chip--mono {
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.68rem;
}

.agent-activity-item__status {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 42px;
  height: 18px;
  padding: 0 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  color: color-mix(in srgb, var(--accent) 76%, var(--text) 24%);
  font-weight: 600;
}

.agent-activity-item__tail {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex: none;
  min-width: max-content;
  padding-left: 8px;
}

.agent-activity-item__tail > * {
  flex: none;
}

.agent-activity-item__tokens {
  min-width: 66px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.agent-activity-item__time {
  width: 56px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.agent-activity-item__duration {
  width: 40px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.agent-activity-item[data-status='started'] .agent-activity-item__status {
  background: color-mix(in srgb, var(--good) 16%, transparent);
  color: color-mix(in srgb, var(--good) 84%, var(--text) 16%);
}

.agent-activity-item[data-status='failed'] .agent-activity-item__status {
  background: color-mix(in srgb, var(--danger, #f85149) 18%, transparent);
  color: var(--danger, #f85149);
}

.agent-activity-item__summary {
  min-width: 0;
  flex: 1 1 auto;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.agent-activity-item__tool-name,
.agent-activity-item__tool-args {
  min-width: 0;
  white-space: nowrap;
}

.agent-activity-item__tool-name {
  max-width: 150px;
}

.agent-activity-item__tool-args {
  flex: 0 1 240px;
  max-width: 240px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.agent-activity-item[data-status='started'] .agent-activity-item__summary {
  color: var(--muted);
}

.agent-activity-item__summary--code {
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.69rem;
}

.agent-activity-item__error {
  margin: 0;
  color: var(--danger, #f85149);
  font-size: 0.7rem;
  line-height: 1.35;
  padding-left: 20px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

@keyframes agent-activity-item-dot-pulse {
  0%,
  100% {
    transform: scale(0.85);
    opacity: 0.55;
  }

  50% {
    transform: scale(1.35);
    opacity: 1;
  }
}
</style>
