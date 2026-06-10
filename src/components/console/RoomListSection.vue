<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { RoomState } from '../../types';
import { formatTime, i18nText } from '../../utils';

const props = defineProps<{
  loading: boolean;
  rooms: RoomState[];
  currentRoomId: number | null;
  createDisabled?: boolean;
}>();

const emit = defineEmits<{
  selectRoom: [roomId: number];
  createRoom: [];
}>();

const { t } = useI18n();

type SortOrder = 'none' | 'asc' | 'desc';
const sortOrder = ref<SortOrder>('none');

const sortedRooms = computed(() => {
  if (sortOrder.value === 'none') {
    return props.rooms;
  }
  return [...props.rooms].sort((a, b) => {
    const timeA = a.last_message_time ?? '';
    const timeB = b.last_message_time ?? '';
    if (sortOrder.value === 'desc') {
      return timeB.localeCompare(timeA);
    }
    return timeA.localeCompare(timeB);
  });
});

function toggleSortOrder(): void {
  if (sortOrder.value === 'none' || sortOrder.value === 'asc') {
    sortOrder.value = 'desc';
  } else {
    sortOrder.value = 'asc';
  }
}

function isDeptRoom(room: RoomState): boolean {
  return Array.isArray(room.tags) && room.tags.includes('DEPT');
}
</script>

<template>
  <section class="sidebar-card panel">
    <div class="block-head">
      <h2>{{ t('room.chatRooms') }}</h2>
      <div class="room-list-head-actions">
        <button
          type="button"
          class="room-sort-button"
          :class="{ active: sortOrder !== 'none' }"
          :aria-label="t('room.sortByTime')"
          :title="t('room.sortByTime')"
          @click="toggleSortOrder"
        >
          <svg class="sort-icon" viewBox="0 0 16 18" fill="none" xmlns="http://www.w3.org/2000/svg">
            <template v-if="sortOrder === 'desc'">
              <path d="M8 1v15M8 16L3 11M8 16l5-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </template>
            <template v-else-if="sortOrder === 'asc'">
              <path d="M8 17V2M8 1L3 6M8 1l5 5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </template>
            <template v-else>
              <path d="M2 4h12M2 9h9M2 14h6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </template>
          </svg>
        </button>
        <span>{{ loading ? 0 : rooms.length }}</span>
        <button
          type="button"
          class="room-add-button"
          :disabled="createDisabled"
          :aria-label="t('room.newRoom')"
          @click="emit('createRoom')"
        >
          +
        </button>
      </div>
    </div>

    <div class="sidebar-scroll">
      <div v-if="loading" class="placeholder">{{ t('room.syncing') }}</div>

      <template v-else-if="rooms.length > 0">
        <button
          v-for="room in sortedRooms"
          :key="room.room_id"
          class="room-card sidebar-item-card"
          :class="{ selected: room.room_id === currentRoomId }"
          type="button"
          @click="emit('selectRoom', room.room_id)"
        >
          <div class="room-head">
            <div class="room-title">
              <span
                class="room-icon"
                :class="room.room_type === 'private' ? 'room-icon-private' : 'room-icon-group'"
              >
                {{ room.room_type === 'private' ? t('room.private') : t('room.group') }}
              </span>
              <strong>{{ i18nText(room.i18n, 'display_name', room.room_name) }}</strong>
              <span v-if="room.unread > 0" class="unread-inline active">{{ room.unread }}</span>
            </div>
            <div class="room-head-right">
              <span v-if="isDeptRoom(room)" class="room-tag room-tag-dept">
                <span class="room-tag-dept__label">{{ t('room.deptGroup') }}</span>
              </span>
              <span v-if="room.last_message_time" class="room-time">{{ formatTime(room.last_message_time) }}</span>
              <div class="room-meta">{{ t('room.membersCount', { count: room.agents.length }) }}</div>
            </div>
          </div>
          <p class="room-preview">{{ room.preview }}</p>
        </button>
      </template>

      <div v-else class="placeholder">{{ t('room.noRooms') }}</div>
    </div>
  </section>
</template>

<style scoped>
.sidebar-card.panel {
  box-shadow: inset 0 0 0 1px var(--panel-border-soft);
}

.room-card {
  width: 100%;
  min-width: 0;
  display: block;
  padding: 8px 10px;
  text-align: left;
  transition:
    background 120ms ease,
    box-shadow 120ms ease;
  cursor: pointer;
}

.room-card + .room-card {
  margin-top: 4px;
}

.room-card:hover,
.room-card.selected {
  background: var(--interactive-selected);
  box-shadow: inset 0 0 0 1px var(--room-card-border-active);
}

.room-head {
  display: flex;
  justify-content: space-between;
  gap: 6px;
  align-items: flex-start;
  min-width: 0;
}

.room-title {
  display: flex;
  flex: 1 1 auto;
  align-items: center;
  gap: 5px;
  min-width: 0;
}

.room-title strong {
  font-size: 0.88rem;
  line-height: 1.15;
  color: var(--text-primary);
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.room-icon {
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  font-size: 0.72rem;
  line-height: 1;
  padding-top: 0.5px;
  flex: 0 0 auto;
}

.room-icon-private {
  border: 1px solid var(--room-private-border);
  background: var(--room-private-bg);
  color: var(--room-private-text);
}

.room-icon-group {
  border: 1px solid var(--room-group-border);
  background: var(--room-group-bg);
  color: var(--room-group-text);
}

.unread-inline {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 15px;
  margin-left: 4px;
  padding: 0 5px;
  border-radius: 999px;
  background: var(--unread-bg);
  color: var(--unread-text);
  font-size: 0.66rem;
  font-weight: 600;
  line-height: 1;
  align-self: center;
}

.room-head-right {
  display: flex;
  align-items: center;
  gap: 6px;
  justify-content: flex-end;
  flex: 0 0 auto;
}

.room-tag {
  display: inline-flex;
  align-items: center;
  height: 18px;
  padding: 0 7px;
  border-radius: 6px;
  font-size: 0.66rem;
  line-height: 1;
  white-space: nowrap;
}

.room-tag-dept {
  border: 1px solid color-mix(in srgb, var(--state-success) 24%, var(--border-default) 76%);
  background: color-mix(in srgb, var(--state-success) 12%, var(--surface-panel) 88%);
  color: color-mix(in srgb, var(--state-success) 84%, var(--text-primary) 16%);
}

.room-tag-dept__label {
  display: inline-block;
  line-height: normal;
}

.room-meta {
  margin: 0;
  color: var(--text-secondary);
  font-size: 0.72rem;
  white-space: nowrap;
}

.room-time {
  color: var(--text-tertiary);
  font-size: 0.68rem;
  white-space: nowrap;
}

.room-preview {
  margin: 2px 0 0;
  color: color-mix(in srgb, var(--text-primary) 72%, var(--text-secondary) 28%);
  line-height: 1.15;
  font-size: 0.74rem;
  transform: translateY(2px);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 1;
  -webkit-box-orient: vertical;
}

.placeholder {
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--surface-panel-muted);
  color: var(--text-secondary);
  font-size: 0.78rem;
}

.room-list-head-actions {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.room-add-button {
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-strong);
  border-radius: 999px;
  background: var(--surface-panel);
  color: var(--text-primary);
  font-size: 0.92rem;
  line-height: 1;
  cursor: pointer;
  transition:
    border-color 0.18s ease,
    background 0.18s ease,
    transform 0.18s ease;
}

.room-add-button:hover:not(:disabled) {
  border-color: var(--interactive-focus-border);
  background: var(--interactive-selected);
  transform: translateY(-1px);
}

.room-add-button:disabled {
  opacity: 0.56;
  cursor: not-allowed;
  transform: none;
}

.room-sort-button {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition:
    background 0.18s ease,
    color 0.18s ease;
}

.room-sort-button:hover {
  background: var(--interactive-selected);
  color: var(--text-primary);
}

.room-sort-button.active {
  color: var(--accent);
}

.sort-icon {
  width: 18px;
  height: 18px;
}
</style>
