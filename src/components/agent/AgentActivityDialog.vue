<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { getAgentDetail, getAgentTasks, getAgentsByTeamId, resumeAgent, stopAgent, superviseAgent } from '../../api';
import { connectionState, showGlobalSuccessToast } from '../../appUiState';
import { displayName, formatConnectionState, formatTime } from '../../utils';
import { loadAgentActivities } from '../../realtime/runtimeStore';
import { useAgentActivities, useAgentStatus } from '../../realtime/selectors';
import AgentCardBase from './AgentCardBase.vue';
import AgentActivityItem from './AgentActivityItem.vue';
import AgentTaskCard from './AgentTaskCard.vue';
import AgentTaskDetailModal from './AgentTaskDetailModal.vue';
import type {
  AgentDetail,
  AgentInfo,
  AgentTask,
  AgentTaskPriority,
  AgentTaskStatus,
  AgentStatus,
} from '../../types';

const { t } = useI18n();

const props = defineProps<{
  open: boolean;
  agentId: number | null;
  agentName: string | null;
  agentStatus?: AgentStatus | null;
  roleTemplateName?: string | null;
}>();

const emit = defineEmits<{
  close: [];
}>();

const agent = ref<AgentDetail | null>(null);
const activityListRef = ref<HTMLElement | null>(null);
const loading = ref(false);
const activitiesLoading = ref(false);
const resuming = ref(false);
const stopping = ref(false);
const errorMessage = ref('');
const superviseContent = ref('');
const supervising = ref(false);
const superviseError = ref('');
const superviseFocused = ref(false);
const superviseTextareaRef = ref<HTMLTextAreaElement | null>(null);
const activitiesErrorMessage = ref('');
const tasksErrorMessage = ref('');
const tasksLoading = ref(false);
const tasks = ref<AgentTask[]>([]);
const teamAgents = ref<AgentInfo[]>([]);
const activeTab = ref<'activities' | 'tasks'>('activities');
const taskFilter = ref<'all' | 'done' | 'undone'>('undone');
const selectedTask = ref<AgentTask | null>(null);
const shouldFollowActivities = ref(true);
const hasAutoScrolledForCurrentAgent = ref(false);

const runtimeStatus = useAgentStatus(() => props.agentId);
const activities = useAgentActivities(() => props.agentId);
const displayAgent = computed<AgentDetail | null>(() => {
  if (!agent.value || agent.value.id !== props.agentId) {
    return null;
  }
  return agent.value;
});

const currentStatus = computed<AgentStatus | null>(() => {
  if (runtimeStatus.value) {
    return runtimeStatus.value;
  }
  if (props.agentStatus) {
    return props.agentStatus;
  }
  return displayAgent.value?.status ?? null;
});

const statusLabel = computed(() => {
  if (!currentStatus.value) {
    return '';
  }
  if (currentStatus.value === 'active') {
    return t('agent.status.active');
  }
  if (currentStatus.value === 'failed') {
    return t('agent.status.failed');
  }
  return t('agent.status.idle');
});

const failureMessage = computed(() => {
  if (currentStatus.value !== 'failed') {
    return '';
  }
  return agent.value?.error_message?.trim() ?? '';
});

const failurePreview = computed(() => {
  const message = failureMessage.value;
  if (!message) {
    return '';
  }
  const preview = message.slice(0, 320).trimEnd();
  if (preview.length === message.length) {
    return preview;
  }
  return `${preview}...`;
});

const agentTemplateLabel = computed(() => {
  if (props.roleTemplateName?.trim()) {
    return props.roleTemplateName.trim();
  }
  if (!displayAgent.value) {
    return t('agent.noTemplate');
  }
  if (typeof displayAgent.value.role_template_id === 'number' && displayAgent.value.role_template_id > 0) {
    return t('agent.templateFallback', { id: displayAgent.value.role_template_id });
  }
  return t('agent.noTemplate');
});

const displayAgentName = computed(() => {
  if (displayAgent.value) {
    return displayName(displayAgent.value);
  }
  return props.agentName ?? 'Agent';
});
const displayEmployeeNumber = computed(() => String(displayAgent.value?.employee_number ?? ''));
const activityRealtimeState = computed(() => connectionState.value);
const activityBadgeLabel = computed(() =>
  activityRealtimeState.value === 'connected' ? t('agent.realtimeConnected') : formatConnectionState(activityRealtimeState.value),
);

const visibleActivities = computed(() =>
  activities.value
    .filter((a) => a.activity_type !== 'agent_state')
    .slice(-30),
);
const DONE_STATUSES = new Set(['DONE', 'CANCELLED']);

const visibleTasks = computed(() => {
  const filtered = taskFilter.value === 'all'
    ? tasks.value
    : taskFilter.value === 'done'
      ? tasks.value.filter((t) => DONE_STATUSES.has(t.status))
      : tasks.value.filter((t) => !DONE_STATUSES.has(t.status));
  return filtered.slice(0, 30);
});

function formatTaskDateTime(value: string | null): string {
  if (!value) {
    return '';
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return '';
  }
  return `${date.toLocaleDateString()} ${formatTime(value)}`.trim();
}

function taskStatusLabel(status: AgentTaskStatus): string {
  return t(`agent.taskStatus.${status}`);
}

function taskPriorityLabel(priority: AgentTaskPriority): string {
  return t(`agent.taskPriority.${priority}`);
}

function taskRefLabel(id: number | null): string {
  if (typeof id === 'number' && id > 0) {
    return `#${id}`;
  }
  return t('common.notSet');
}

function taskActorLabel(id: number | null): string {
  if (typeof id !== 'number' || id <= 0) {
    return t('common.notSet');
  }
  const matched = teamAgents.value.find((item) => item.id === id);
  return matched ? displayName(matched) : `#${id}`;
}

function taskActorDetailLabel(id: number | null): string {
  if (typeof id !== 'number' || id <= 0) {
    return t('common.none');
  }
  const matched = teamAgents.value.find((item) => item.id === id);
  return matched ? displayName(matched) : `#${id}`;
}

function taskRoomDetailLabel(id: number | null): string {
  if (typeof id !== 'number' || id <= 0) {
    return t('common.none');
  }
  return `#${id}`;
}

function openTaskDetail(task: AgentTask): void {
  selectedTask.value = task;
}

function closeTaskDetail(): void {
  selectedTask.value = null;
}

async function scrollActivitiesToBottom(): Promise<void> {
  await nextTick();
  if (!activityListRef.value) {
    return;
  }
  activityListRef.value.scrollTop = activityListRef.value.scrollHeight;
}

let _activityListResizeObserver: ResizeObserver | null = null;

function attachActivityListResizeObserver(el: HTMLElement): void {
  detachActivityListResizeObserver();
  _activityListResizeObserver = new ResizeObserver(() => {
    if (shouldFollowActivities.value) {
      el.scrollTop = el.scrollHeight;
    }
  });
  _activityListResizeObserver.observe(el);
}

function detachActivityListResizeObserver(): void {
  _activityListResizeObserver?.disconnect();
  _activityListResizeObserver = null;
}

onUnmounted(detachActivityListResizeObserver);

function isActivityListNearBottom(): boolean {
  const listEl = activityListRef.value;
  if (!listEl) {
    return true;
  }

  const distanceToBottom = listEl.scrollHeight - listEl.scrollTop - listEl.clientHeight;
  return distanceToBottom <= 12;
}

function syncActivityFollowState(): void {
  shouldFollowActivities.value = isActivityListNearBottom();
}

async function loadActivities(): Promise<void> {
  if (!props.open || props.agentId === null) {
    activitiesErrorMessage.value = '';
    activitiesLoading.value = false;
    return;
  }

  activitiesLoading.value = activities.value.length === 0;
  activitiesErrorMessage.value = '';

  try {
    await loadAgentActivities(props.agentId);
    await scrollActivitiesToBottom();
  } catch (error) {
    activitiesErrorMessage.value = t('agent.loadFailed');
    console.error(error);
  } finally {
    activitiesLoading.value = false;
  }
}

async function loadTasks(): Promise<void> {
  if (!props.open || props.agentId === null) {
    tasks.value = [];
    tasksErrorMessage.value = '';
    tasksLoading.value = false;
    return;
  }

  tasksLoading.value = true;
  tasksErrorMessage.value = '';

  try {
    tasks.value = await getAgentTasks(props.agentId, taskFilter.value !== 'undone');
  } catch (error) {
    tasksErrorMessage.value = t('agent.tasksLoadFailed');
    console.error(error);
  } finally {
    tasksLoading.value = false;
  }
}

async function loadDetail(): Promise<void> {
  if (!props.open || props.agentId === null) {
    agent.value = null;
    teamAgents.value = [];
    errorMessage.value = '';
    loading.value = false;
    return;
  }

  loading.value = true;
  errorMessage.value = '';

  try {
    agent.value = await getAgentDetail(props.agentId);
    if (typeof agent.value.team_id === 'number' && agent.value.team_id > 0) {
      teamAgents.value = await getAgentsByTeamId(agent.value.team_id);
    } else {
      teamAgents.value = [];
    }
  } catch (error) {
    errorMessage.value = t('agent.infoLoadFailed');
    console.error(error);
  } finally {
    loading.value = false;
  }
}

async function handleResume(): Promise<void> {
  if (props.agentId === null || currentStatus.value !== 'failed' || resuming.value) {
    return;
  }

  resuming.value = true;

  try {
    await resumeAgent(props.agentId);
    showGlobalSuccessToast(t('agent.resumeSuccess'));
  } catch (error) {
    console.error(error);
  } finally {
    resuming.value = false;
  }
}

async function handleStop(): Promise<void> {
  if (props.agentId === null || currentStatus.value !== 'active' || stopping.value) {
    return;
  }

  stopping.value = true;

  try {
    await stopAgent(props.agentId);
    showGlobalSuccessToast(t('agent.stopSuccess'));
  } catch (error) {
    console.error(error);
  } finally {
    stopping.value = false;
  }
}

async function copyFailureMessage(): Promise<void> {
  if (!failureMessage.value) {
    return;
  }

  try {
    await navigator.clipboard.writeText(failureMessage.value);
    showGlobalSuccessToast(t('agent.copiedError'));
  } catch (error) {
    console.error(error);
  }
}

async function sendSupervise(): Promise<void> {
  if (!props.agentId || !superviseContent.value.trim() || supervising.value) {
    return;
  }
  supervising.value = true;
  superviseError.value = '';
  try {
    await superviseAgent(props.agentId, superviseContent.value.trim());
    superviseContent.value = '';
    if (superviseTextareaRef.value) {
      superviseTextareaRef.value.style.height = '';
    }
  } catch (error) {
    superviseError.value = error instanceof Error ? error.message : String(error);
  } finally {
    supervising.value = false;
  }
}

function autoGrowSupervise(): void {
  const el = superviseTextareaRef.value;
  if (!el) return;
  el.style.height = 'auto';
  el.style.height = `${el.scrollHeight}px`;
}

watch(
  () => [props.open, props.agentId, props.agentName],
  () => {
    activeTab.value = 'activities';
    selectedTask.value = null;
    hasAutoScrolledForCurrentAgent.value = false;
    shouldFollowActivities.value = true;
    loadDetail().catch(console.error);
    loadActivities().catch(console.error);
    loadTasks().catch(console.error);
  },
  { immediate: true },
);

watch(
  () => connectionState.value,
  (state, previousState) => {
    if (
      !props.open
      || props.agentId === null
      || state !== 'connected'
      || previousState === 'connected'
      || previousState === 'connecting'
    ) {
      return;
    }
    loadDetail().catch(console.error);
    loadActivities().catch(console.error);
    loadTasks().catch(console.error);
  },
);

watch(
  () => currentStatus.value,
  (status, previousStatus) => {
    if (
      props.open
      && props.agentId !== null
      && status === 'failed'
      && previousStatus !== 'failed'
    ) {
      loadDetail().catch(console.error);
    }
  },
);

// Watch activityListRef: attach ResizeObserver when the list element mounts,
// so any height change (new items or streaming content) auto-scrolls to bottom.
watch(
  () => activityListRef.value,
  async (el, prevEl) => {
    if (prevEl) {
      detachActivityListResizeObserver();
    }
    if (!el) {
      return;
    }
    shouldFollowActivities.value = true;
    hasAutoScrolledForCurrentAgent.value = true;
    await scrollActivitiesToBottom();
    attachActivityListResizeObserver(el);
  },
);
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="agent-detail-overlay" @click.self="emit('close')">
      <section class="agent-detail-dialog panel">
        <div class="agent-detail-head">
          <p class="agent-detail-eyebrow">Agent Status Card</p>
          <button type="button" class="agent-detail-close" :aria-label="t('common.close')" @click="emit('close')">×</button>
        </div>

        <div v-if="errorMessage" class="error-banner">{{ errorMessage }}</div>
        <div v-else-if="loading && !displayAgent && !agentName" class="loading-card">{{ t('agent.loadingInfo') }}</div>

        <template v-else-if="displayAgent || agentName">
          <section class="agent-detail-stage">
            <div class="agent-detail-stage__left">
              <div class="agent-detail-stage__card-stack">
                <AgentCardBase
                  :title="displayAgentName"
                  :subtitle="agentTemplateLabel"
                  :employee-number="displayEmployeeNumber"
                  :avatar-name="displayAgentName"
                  variant="profile"
                  readonly
                />
                <div class="agent-status-panel" :data-status="currentStatus ?? undefined">
                  <span class="status-dot"></span>
                  <span class="agent-status-panel__value">{{ statusLabel }}</span>
                  <button
                    v-if="currentStatus === 'failed'"
                    type="button"
                    class="agent-status-panel__action"
                    :disabled="resuming"
                    @click="handleResume"
                  >
                    {{ resuming ? t('agent.resuming') : t('agent.resume') }}
                  </button>
                  <button
                    v-if="currentStatus === 'active'"
                    type="button"
                    class="agent-status-panel__action agent-status-panel__action--stop"
                    :disabled="stopping"
                    @click="handleStop"
                  >
                    {{ stopping ? t('agent.stopping') : t('agent.stop') }}
                  </button>
                </div>
                <div v-if="failureMessage" class="agent-error-panel">
                  <p class="agent-error-message">{{ failurePreview }}</p>
                  <button type="button" class="agent-error-panel__copy" @click="copyFailureMessage">
                    {{ t('agent.copyAll') }}
                  </button>
                </div>
              </div>
            </div>
            <div class="agent-detail-stage__right">
              <section class="agent-activity-panel">
                <div class="agent-activity-panel__head">
                  <div class="agent-activity-panel__title-line">
                    <div class="agent-activity-panel__tabs" role="tablist" :aria-label="t('agent.panelTabs')">
                      <button
                        type="button"
                        class="agent-activity-panel__tab"
                        :class="{ 'is-active': activeTab === 'activities' }"
                        :aria-selected="activeTab === 'activities'"
                        @click="activeTab = 'activities'"
                      >
                        {{ t('agent.activities') }}
                      </button>
                      <button
                        type="button"
                        class="agent-activity-panel__tab"
                        :class="{ 'is-active': activeTab === 'tasks' }"
                        :aria-selected="activeTab === 'tasks'"
                        @click="activeTab = 'tasks'"
                      >
                        {{ t('agent.tasks') }}
                      </button>
                    </div>
                  </div>
                  <span
                    v-if="activeTab === 'activities'"
                    class="agent-activity-panel__badge"
                    :data-state="activityRealtimeState"
                  >
                    <span
                      class="agent-activity-panel__badge-dot"
                    ></span>
                    {{ activityBadgeLabel }}
                  </span>
                  <span v-else class="agent-activity-panel__badge agent-activity-panel__badge--count">
                    {{ t('agent.taskCount', { count: visibleTasks.length }) }}
                  </span>
                </div>

                <template v-if="activeTab === 'activities'">
                  <div v-if="activitiesErrorMessage" class="error-banner">{{ activitiesErrorMessage }}</div>
                  <div v-else-if="activitiesLoading && !visibleActivities.length" class="loading-card">{{ t('agent.loadingActivities') }}</div>
                  <div v-else-if="!activitiesLoading && !visibleActivities.length" class="agent-activity-empty">
                    {{ t('agent.noActivities') }}
                  </div>
                  <div
                    v-else
                    ref="activityListRef"
                    class="agent-activity-list sidebar-scroll"
                    @scroll="syncActivityFollowState"
                  >
                    <AgentActivityItem
                      v-for="activity in visibleActivities"
                      :key="activity.id"
                      :activity="activity"
                    />
                  </div>
                </template>
                <template v-else>
                  <div class="agent-task-filter">
                    <button
                      type="button"
                      class="agent-task-filter__btn"
                      :class="{ 'is-active': taskFilter === 'undone' }"
                      @click="taskFilter = 'undone'; loadTasks()"
                    >{{ t('agent.taskFilterUndone') }}</button>
                    <button
                      type="button"
                      class="agent-task-filter__btn"
                      :class="{ 'is-active': taskFilter === 'done' }"
                      @click="taskFilter = 'done'; loadTasks()"
                    >{{ t('agent.taskFilterDone') }}</button>
                    <button
                      type="button"
                      class="agent-task-filter__btn"
                      :class="{ 'is-active': taskFilter === 'all' }"
                      @click="taskFilter = 'all'; loadTasks()"
                    >{{ t('agent.taskFilterAll') }}</button>
                  </div>
                  <div v-if="tasksErrorMessage" class="error-banner">{{ tasksErrorMessage }}</div>
                  <div v-else-if="tasksLoading && !visibleTasks.length" class="loading-card">{{ t('agent.loadingTasks') }}</div>
                  <div v-else-if="!tasksLoading && !visibleTasks.length" class="agent-activity-empty">
                    {{ taskFilter === 'undone' ? t('agent.noTasks') : taskFilter === 'done' ? t('agent.taskFilterNoTasksDone') : t('agent.taskFilterNoTasksAll') }}
                  </div>
                  <div v-else class="agent-task-list sidebar-scroll">
                    <AgentTaskCard
                      v-for="task in visibleTasks"
                      :key="task.id"
                      :task="task"
                      :assignee-label="taskActorLabel(task.assignee_id)"
                      :manager-label="task.manager_id !== null ? taskActorDetailLabel(task.manager_id) : null"
                      clickable
                      @select="openTaskDetail"
                    />
                  </div>
                </template>
              </section>

              <section v-if="activeTab === 'activities'" class="agent-supervise-section">
                <div class="agent-supervise-section__input-row">
                  <div class="agent-supervise-section__editor" :class="{ 'is-focused': superviseFocused }">
                    <textarea
                      ref="superviseTextareaRef"
                      v-model="superviseContent"
                      class="agent-supervise-section__textarea"
                      :placeholder="t('agent.supervise.placeholder')"
                      rows="1"
                      :disabled="supervising"
                      @focus="superviseFocused = true"
                      @blur="superviseFocused = false"
                      @input="autoGrowSupervise"
                      @keydown.ctrl.enter.prevent="sendSupervise"
                      @keydown.meta.enter.prevent="sendSupervise"
                    />
                  </div>
                  <button
                    type="button"
                    class="agent-supervise-section__send"
                    :disabled="supervising || !superviseContent.trim()"
                    @click="sendSupervise"
                  >
                    {{ supervising ? t('agent.supervise.sending') : t('agent.supervise.send') }}
                  </button>
                </div>
                <p v-if="superviseError" class="agent-supervise-section__error">{{ superviseError }}</p>
              </section>
            </div>
          </section>

          <AgentTaskDetailModal
            :task="selectedTask"
            :created-at-label="selectedTask ? (formatTaskDateTime(selectedTask.created_at) || t('common.notSet')) : ''"
            :creator-label="selectedTask ? taskActorLabel(selectedTask.creator_id) : ''"
            :assignee-label="selectedTask ? taskActorLabel(selectedTask.assignee_id) : ''"
            :manager-label="selectedTask ? taskActorDetailLabel(selectedTask.manager_id) : ''"
            :room-label="selectedTask ? taskRoomDetailLabel(selectedTask.room_id) : ''"
            :priority-label="selectedTask ? taskPriorityLabel(selectedTask.priority) : ''"
            :status-label="selectedTask ? taskStatusLabel(selectedTask.status) : ''"
            @close="closeTaskDetail"
          />
        </template>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.agent-detail-overlay {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(112, 133, 160, 0.22);
  backdrop-filter: blur(6px);
}

.agent-detail-dialog {
  width: min(1180px, calc(100vw - 40px));
  height: min(820px, calc(100vh - 40px));
  max-height: min(820px, calc(100vh - 40px));
  overflow: hidden;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 2px;
  padding: 10px 16px 16px;
  border-radius: 20px;
  box-shadow:
    0 20px 48px rgba(40, 67, 102, 0.16),
    inset 0 0 0 1px color-mix(in srgb, var(--panel-border) 88%, transparent);
}

.agent-detail-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.agent-detail-head > div {
  min-width: 0;
}

.agent-detail-close {
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--muted);
  font-size: 1.2rem;
  line-height: 1;
  cursor: pointer;
}

.agent-detail-close:hover {
  background: color-mix(in srgb, var(--surface-soft) 90%, transparent);
  color: var(--text-strong);
}

.agent-detail-eyebrow {
  margin: 0 0 2px;
  color: var(--accent);
  text-transform: uppercase;
  letter-spacing: 0.14em;
  font-size: 0.68rem;
}

.agent-detail-head h3,
.prompt-head h4 {
  margin: 0;
  color: var(--text-strong);
  font-size: 1.05rem;
}

.agent-detail-stage,
.loading-card {
  background: transparent;
}

.agent-detail-stage {
  min-height: 0;
  height: 100%;
  display: grid;
  grid-template-columns: 248px minmax(0, 1fr);
  gap: 12px;
  align-items: stretch;
  padding: 0;
  overflow: hidden;
}

.agent-detail-stage__left {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 0;
  height: 100%;
  padding-top: 0;
}

.agent-detail-stage__card-stack {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  justify-content: center;
  min-height: 100%;
  width: fit-content;
  max-width: 100%;
}

.agent-detail-stage__card-stack :deep(.entity-card) {
  cursor: default;
  margin: 0 auto;
  transform: scale(0.96);
  transform-origin: top center;
}

.agent-detail-stage__card-stack :deep(.entity-card:hover) {
  transform: scale(0.96);
}

.agent-status-panel {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: 8px;
  min-height: 42px;
  width: auto;
  max-width: 100%;
  padding: 4px 0 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--muted);
  font-size: 0.82rem;
  line-height: 1;
}

.agent-status-panel__value {
  color: inherit;
  font-size: inherit;
  font-weight: 600;
  line-height: inherit;
}

.agent-status-panel__action {
  height: 24px;
  padding: 0 8px;
  border: 1px solid currentColor;
  border-radius: 999px;
  background: transparent;
  color: inherit;
  font-size: 0.68rem;
  font-weight: 600;
  line-height: 1;
  cursor: pointer;
}

.agent-status-panel__action:disabled {
  opacity: 0.7;
  cursor: wait;
}

.agent-status-panel__action--stop {
  color: var(--danger, #f85149);
  border-color: var(--danger, #f85149);
}

.agent-status-panel[data-status='failed'] {
  color: var(--danger, #f85149);
}

.agent-error-panel {
  width: min(260px, 100%);
  margin: 0;
  padding: 10px 12px;
  border-radius: 14px;
  border: 1px solid color-mix(in srgb, var(--danger, #f85149) 18%, var(--panel-border) 82%);
  background: color-mix(in srgb, var(--danger, #f85149) 5%, var(--panel-bg) 95%);
}

.agent-error-message {
  margin: 0;
  max-height: 132px;
  overflow: hidden;
  color: color-mix(in srgb, var(--danger, #f85149) 88%, var(--text) 12%);
  font-size: 0.72rem;
  line-height: 1.4;
  text-align: left;
  white-space: pre-wrap;
  word-break: break-word;
  display: -webkit-box;
  -webkit-line-clamp: 8;
  -webkit-box-orient: vertical;
}

.agent-error-panel__copy {
  margin-top: 6px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--accent);
  font-size: 0.7rem;
  line-height: 1;
  cursor: pointer;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 999px;
  background: var(--status-dot-idle);
}

.agent-status-panel[data-status='active'] .status-dot {
  background: var(--state-success);
  box-shadow: none;
}

.agent-status-panel[data-status='failed'] .status-dot {
  background: var(--danger, #f85149);
  box-shadow: none;
}

.agent-detail-stage__right {
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 0;
  height: 100%;
  position: relative;
}

.agent-activity-panel {
  min-height: 0;
  flex: 1;
  border-radius: 18px 18px 0 0;
  padding: 0;
  background: color-mix(in srgb, var(--panel-bg) 97%, var(--surface-soft) 3%);
  border: 1px solid color-mix(in srgb, var(--panel-border) 82%, white 18%);
  border-bottom: none;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.7);
  display: flex;
  flex-direction: column;
}

.agent-activity-panel__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 6px 10px 6px;
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.agent-activity-panel__title-line {
  display: flex;
  align-items: baseline;
  gap: 6px;
  min-width: 0;
}

.agent-activity-panel__tabs {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}

.agent-activity-panel__tab {
  height: 28px;
  padding: 0 12px;
  border: 1px solid transparent;
  border-radius: 999px;
  background: transparent;
  color: var(--muted);
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
  transition:
    color 160ms ease,
    background 160ms ease,
    border-color 160ms ease;
}

.agent-activity-panel__tab:hover {
  color: var(--text-strong);
  background: color-mix(in srgb, var(--surface-soft) 72%, transparent);
}

.agent-activity-panel__tab.is-active {
  color: var(--text-strong);
  background: color-mix(in srgb, var(--surface-pill) 82%, var(--surface-panel-muted) 18%);
  border-color: color-mix(in srgb, var(--border-subtle) 78%, transparent);
}

.agent-activity-panel__head h4 {
  margin: 0;
  color: var(--text-strong);
  font-size: 0.96rem;
}

.agent-activity-panel__badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 22px;
  padding: 0 8px;
  border-radius: 999px;
  background: rgba(125, 163, 224, 0.12);
  color: color-mix(in srgb, var(--accent) 78%, var(--text) 22%);
  font-size: 0.68rem;
  font-weight: 600;
}

.agent-activity-panel__badge--count {
  background: color-mix(in srgb, var(--surface-pill) 84%, var(--surface-panel-muted) 16%);
  color: var(--text-secondary);
}

.agent-activity-panel__badge[data-state='connected'] {
  color: var(--good);
}

.agent-activity-panel__badge[data-state='waiting_reconnect'],
.agent-activity-panel__badge[data-state='reconnecting'] {
  color: var(--warn);
}

.agent-activity-panel__badge[data-state='disconnected'] {
  color: var(--danger);
}

.agent-activity-panel__badge-dot {
  position: relative;
  width: 10px;
  height: 10px;
  flex: 0 0 10px;
  color: var(--status-dot-idle);
}

.agent-activity-panel__badge-dot::before {
  content: '';
  position: absolute;
  inset: 1.5px;
  border-radius: 999px;
  background: currentColor;
}

.agent-activity-panel__badge[data-state='connected'] .agent-activity-panel__badge-dot {
  color: var(--good);
}

.agent-activity-panel__badge[data-state='waiting_reconnect'] .agent-activity-panel__badge-dot,
.agent-activity-panel__badge[data-state='reconnecting'] .agent-activity-panel__badge-dot,
.agent-activity-panel__badge[data-state='connecting'] .agent-activity-panel__badge-dot {
  color: var(--warn);
}

.agent-activity-panel__badge[data-state='disconnected'] .agent-activity-panel__badge-dot {
  color: var(--danger);
}

.agent-activity-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: visible;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 4px 10px 4px;
  scroll-padding-bottom: 16px;
  box-sizing: border-box;
}

.agent-task-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 10px 8px;
}

.agent-task-filter {
  display: flex;
  flex-shrink: 0;
  gap: 6px;
  padding: 8px 12px 4px;
}

.agent-task-filter__btn {
  padding: 3px 10px;
  border-radius: 99px;
  border: 1px solid var(--panel-border);
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}

.agent-task-filter__btn:hover {
  background: color-mix(in srgb, var(--surface-soft) 60%, transparent);
  color: var(--text-primary);
}

.agent-task-filter__btn.is-active {
  background: color-mix(in srgb, var(--interactive-selected) 22%, var(--surface-pill) 78%);
  border-color: color-mix(in srgb, var(--interactive-selected) 34%, var(--panel-border) 66%);
  color: var(--text-strong);
}


.agent-activity-empty {
  min-height: 120px;
  display: grid;
  place-items: center;
  color: var(--muted);
  margin: 0 10px 10px;
}

.loading-card,
.error-banner {
  padding: 14px;
  margin: 0 10px 10px;
}

.loading-card {
  border: 1px solid var(--panel-border);
  border-radius: 14px;
  background: var(--surface-soft);
}

.error-banner {
  border-radius: 10px;
  background: var(--banner-error-bg);
  color: var(--banner-error-text);
  border: 1px solid var(--banner-error-border);
}

@media (max-width: 720px) {
  .agent-detail-overlay {
    padding: 12px;
  }

  .agent-detail-dialog {
    width: min(100vw - 24px, 100%);
    height: min(100vh - 24px, 100%);
    max-height: calc(100vh - 24px);
    padding: 0 14px 14px;
    gap: 6px;
  }

  .agent-detail-stage {
    grid-template-columns: 1fr;
    min-height: 0;
    padding: 8px 0 0;
    gap: 12px;
  }

  .agent-detail-stage__right {
    min-height: 180px;
  }

  .agent-detail-stage__left {
    align-items: flex-start;
    justify-content: stretch;
  }

  .agent-detail-stage__card-stack {
    width: 100%;
  }

  .agent-activity-panel {
    min-height: 0;
    padding: 0;
  }

  .agent-task-detail-overlay {
    padding: 12px;
  }

  .agent-task-detail-modal__grid {
    grid-template-columns: 1fr;
  }
}

.agent-supervise-section {
  padding: 4px 10px 10px;
  flex-shrink: 0;
  background: color-mix(in srgb, var(--panel-bg) 97%, var(--surface-soft) 3%);
  border: 1px solid color-mix(in srgb, var(--panel-border) 82%, white 18%);
  border-top: 1px solid var(--border-subtle);
  border-radius: 0 0 18px 18px;
  box-shadow: inset 0 -1px 0 rgba(255, 255, 255, 0.5);
}

.agent-supervise-section__input-row {
  display: flex;
  align-items: flex-end;
  gap: 8px;
}

.agent-supervise-section__editor {
  flex: 1;
  min-width: 0;
  background: var(--surface-input);
  border: 1px solid color-mix(in srgb, var(--border-subtle) 78%, var(--border-default) 22%);
  border-radius: 8px;
  overflow: hidden;
  transition:
    border-color 160ms ease,
    box-shadow 160ms ease;
}

.agent-supervise-section__editor.is-focused {
  border-color: var(--input-focus-border);
  box-shadow: 0 0 0 2px var(--input-focus-ring);
}

.agent-supervise-section__textarea {
  display: block;
  width: 100%;
  resize: none;
  border: none;
  border-radius: 0;
  padding: 7px 10px;
  font-size: 0.8rem;
  font-family: inherit;
  background: transparent;
  color: var(--text-primary);
  line-height: 1.4;
  min-height: 30px;
  max-height: 160px;
  overflow-y: auto;
  outline: none;
}

.agent-supervise-section__textarea::placeholder {
  color: var(--text-secondary);
}

.agent-supervise-section__textarea:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.agent-supervise-section__send {
  flex-shrink: 0;
  border: 0;
  border-radius: 6px;
  padding: 5px 10px;
  background: var(--interactive-selected);
  color: var(--text-primary);
  font-weight: 700;
  cursor: pointer;
  font-size: 0.74rem;
  white-space: nowrap;
  transition: opacity 0.15s;
}

.agent-supervise-section__send:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

.agent-supervise-section__error {
  margin: 5px 4px 0;
  font-size: 0.75rem;
  color: var(--color-error, #ef4444);
}</style>
