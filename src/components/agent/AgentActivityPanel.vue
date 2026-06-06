<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { getAgentActivitiesPage } from '../../api';
import { connectionState } from '../../appUiState';
import { useAgentActivities } from '../../realtime/selectors';
import type { AgentActivity } from '../../types';
import AgentActivityItem from './AgentActivityItem.vue';

const props = defineProps<{
  open: boolean;
  agentId: number | null;
}>();

const emit = defineEmits<{
  followChange: [boolean];
}>();

const { t } = useI18n();

const activityListRef = ref<HTMLElement | null>(null);
const activityContentRef = ref<HTMLElement | null>(null);
const loadedActivities = ref<AgentActivity[]>([]);
const activitiesLoading = ref(false);
const activitiesLoadingOlder = ref(false);
const activitiesErrorMessage = ref('');
const activitiesHasMore = ref(false);
const shouldFollowActivities = ref(true);
watch(shouldFollowActivities, (val) => {
  emit('followChange', val);
}, { immediate: true });
const lastActivityScrollTop = ref(0);
const ACTIVITY_PAGE_SIZE = 50;
let activityRequestToken = 0;
let activityListResizeObserver: ResizeObserver | null = null;

const realtimeActivities = useAgentActivities(() => props.agentId);

function sortActivitiesAscending(items: AgentActivity[]): AgentActivity[] {
  return [...items].sort((left, right) => left.id - right.id);
}

function mergeActivityCollections(...groups: AgentActivity[][]): AgentActivity[] {
  const merged = new Map<number, AgentActivity>();
  for (const group of groups) {
    for (const activity of group) {
      merged.set(activity.id, activity);
    }
  }
  return sortActivitiesAscending([...merged.values()]);
}

const visibleActivities = computed(() =>
  mergeActivityCollections(loadedActivities.value, realtimeActivities.value)
    .filter((activity) => activity.activity_type !== 'agent_state'),
);

async function scrollActivitiesToBottom(): Promise<void> {
  await nextTick();
  if (!activityListRef.value) {
    return;
  }
  activityListRef.value.scrollTop = activityListRef.value.scrollHeight;
  lastActivityScrollTop.value = activityListRef.value.scrollTop;
}

function detachActivityListResizeObserver(): void {
  activityListResizeObserver?.disconnect();
  activityListResizeObserver = null;
}

function attachActivityListResizeObserver(contentEl: HTMLElement): void {
  detachActivityListResizeObserver();
  activityListResizeObserver = new ResizeObserver(() => {
    if (shouldFollowActivities.value && activityListRef.value) {
      activityListRef.value.scrollTop = activityListRef.value.scrollHeight;
      lastActivityScrollTop.value = activityListRef.value.scrollTop;
    }
  });
  activityListResizeObserver.observe(contentEl);
}

onUnmounted(detachActivityListResizeObserver);

function isActivityListNearBottom(): boolean {
  const listEl = activityListRef.value;
  if (!listEl) {
    return true;
  }
  const distanceToBottom = listEl.scrollHeight - listEl.scrollTop - listEl.clientHeight;
  return distanceToBottom <= 20;
}

function syncActivityFollowState(): void {
  shouldFollowActivities.value = isActivityListNearBottom();
}

async function loadActivities(): Promise<void> {
  if (!props.open || props.agentId === null) {
    loadedActivities.value = [];
    activitiesHasMore.value = false;
    activitiesErrorMessage.value = '';
    activitiesLoadingOlder.value = false;
    activitiesLoading.value = false;
    lastActivityScrollTop.value = 0;
    return;
  }

  const requestToken = ++activityRequestToken;
  activitiesLoading.value = true;
  activitiesErrorMessage.value = '';
  activitiesLoadingOlder.value = false;
  loadedActivities.value = [];
  activitiesHasMore.value = false;
  lastActivityScrollTop.value = 0;

  try {
    const page = await getAgentActivitiesPage(props.agentId, { limit: ACTIVITY_PAGE_SIZE });
    if (requestToken !== activityRequestToken) {
      return;
    }
    loadedActivities.value = sortActivitiesAscending(page.activities);
    activitiesHasMore.value = page.hasMore;
    await scrollActivitiesToBottom();
  } catch (error) {
    if (requestToken !== activityRequestToken) {
      return;
    }
    activitiesErrorMessage.value = t('agent.loadFailed');
    console.error(error);
  } finally {
    if (requestToken === activityRequestToken) {
      activitiesLoading.value = false;
    }
  }
}

async function loadOlderActivities(): Promise<void> {
  if (
    !props.open
    || props.agentId === null
    || activitiesLoading.value
    || activitiesLoadingOlder.value
    || !activitiesHasMore.value
    || loadedActivities.value.length === 0
  ) {
    return;
  }

  const oldestLoadedId = loadedActivities.value[0]?.id;
  const listEl = activityListRef.value;
  if (!oldestLoadedId || !listEl) {
    return;
  }

  const requestToken = activityRequestToken;
  const previousScrollHeight = listEl.scrollHeight;
  const previousScrollTop = listEl.scrollTop;
  activitiesLoadingOlder.value = true;

  try {
    const page = await getAgentActivitiesPage(props.agentId, {
      limit: ACTIVITY_PAGE_SIZE,
      beforeId: oldestLoadedId,
    });
    if (requestToken !== activityRequestToken) {
      return;
    }

    if (page.activities.length > 0) {
      loadedActivities.value = mergeActivityCollections(page.activities, loadedActivities.value);
      await nextTick();
      if (activityListRef.value === listEl) {
        listEl.scrollTop = listEl.scrollHeight - previousScrollHeight + previousScrollTop;
        lastActivityScrollTop.value = listEl.scrollTop;
      }
    }

    activitiesHasMore.value = page.hasMore;
  } catch (error) {
    if (requestToken !== activityRequestToken) {
      return;
    }
    if (loadedActivities.value.length === 0 && realtimeActivities.value.length === 0) {
      activitiesErrorMessage.value = t('agent.loadFailed');
    }
    console.error(error);
  } finally {
    if (requestToken === activityRequestToken) {
      activitiesLoadingOlder.value = false;
    }
  }
}

function handleActivityListScroll(): void {
  const listEl = activityListRef.value;
  if (!listEl) {
    return;
  }
  const currentScrollTop = listEl.scrollTop;
  const scrollingUp = currentScrollTop < lastActivityScrollTop.value;
  if (currentScrollTop !== lastActivityScrollTop.value) {
    syncActivityFollowState();
  }
  if (scrollingUp && currentScrollTop <= 24) {
    loadOlderActivities().catch(console.error);
  }
  lastActivityScrollTop.value = currentScrollTop;
}

watch(
  () => [props.open, props.agentId] as const,
  () => {
    shouldFollowActivities.value = true;
    loadActivities().catch(console.error);
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
    loadActivities().catch(console.error);
  },
);

watch(
  () => activityContentRef.value,
  async (el, prevEl) => {
    if (prevEl) {
      detachActivityListResizeObserver();
    }
    if (!el) {
      return;
    }
    shouldFollowActivities.value = true;
    await scrollActivitiesToBottom();
    lastActivityScrollTop.value = activityListRef.value?.scrollTop ?? 0;
    attachActivityListResizeObserver(el);
  },
);
</script>

<template>
  <div class="agent-activity-panel-body">
    <div v-if="activitiesErrorMessage" class="error-banner">{{ activitiesErrorMessage }}</div>
    <div v-else-if="activitiesLoading && !visibleActivities.length" class="loading-card">{{ t('agent.loadingActivities') }}</div>
    <div v-else-if="!activitiesLoading && !visibleActivities.length" class="agent-activity-empty">
      {{ t('agent.noActivities') }}
    </div>
    <div
      v-else
      ref="activityListRef"
      class="agent-activity-list sidebar-scroll"
      @scroll="handleActivityListScroll"
    >
      <div ref="activityContentRef" class="agent-activity-list__content">
        <div v-if="activitiesLoadingOlder" class="agent-activity-list__loading-more">
          {{ t('agent.loadingEarlierActivities') }}
        </div>
        <AgentActivityItem
          v-for="activity in visibleActivities"
          :key="activity.id"
          :activity="activity"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.agent-activity-panel-body {
  min-height: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
}

.agent-activity-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: visible;
  padding: 0;
  scroll-padding-bottom: 16px;
  box-sizing: border-box;
}

.agent-activity-list__content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 4px 0 10px;
}

.agent-activity-list__loading-more {
  padding: 4px 0 6px;
  color: var(--text-secondary);
  font-size: 0.72rem;
  text-align: center;
}

.agent-activity-empty {
  min-height: 120px;
  display: grid;
  place-items: center;
  color: var(--muted);
  margin: 0 0 10px;
}

.loading-card,
.error-banner {
  padding: 14px;
  margin: 0 0 10px;
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
</style>
