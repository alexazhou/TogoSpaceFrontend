<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { RouterView, useRoute, useRouter } from 'vue-router';
import {
  clearGlobalRequestError,
  connectionState,
  setGlobalRequestErrorAutoDismiss,
  globalRequestErrors,
  globalSuccessToasts,
  type GlobalRequestErrorToast,
  reconnectProgress,
  scheduleNotRunningReason,
  scheduleState,
  showGlobalRequestError,
  showGlobalSuccessToast,
  showQuickInit,
  showTokenDialog,
  authEnabled,
  totalMessageCount,
  updateScheduleState,
} from './appUiState';
import { getSystemStatus, resumeSchedule, setTeamEnabled } from './api';
import { getToken } from './authStore';
import QuickInitModal from './components/layout/QuickInitModal.vue';
import TokenDialog from './components/ui/TokenDialog.vue';
import TopBar from './components/layout/TopBar.vue';
import { DEFAULT_SETTINGS_SECTION } from './components/settings/sections';
import ConfirmDialog from './components/ui/ConfirmDialog.vue';
import { startRealtimeClient, stopRealtimeClient } from './realtime/wsClient';
import { installViewportRootClasses } from './responsive/breakpoints';
import { findTeamById, firstTeamId, loadTeams, preferredTeamId, setPreferredTeamId, teams, teamsLoaded } from './teamStore';
import {
  appendDesktopDebugEvent,
  getDesktopStatus,
  isTauriRuntime,
  listenDesktopBackendStatus,
  openLocalTogoSpaceRoot,
  startLocalBackend,
  stopLocalBackend,
  type DesktopStatus,
} from './desktop';
import { formatConnectionState } from './utils';

type ThemeMode = 'dark' | 'light';
type ConsoleMainView = 'chat' | 'tasks';

const route = useRoute();
const router = useRouter();
const { t } = useI18n();

const themeMode = ref<ThemeMode>((localStorage.getItem('theme-mode') as ThemeMode) || 'dark');

const teamIdFromRoute = computed<number | null>(() => {
  const raw = route.params.teamId;
  if (typeof raw !== 'string') {
    return null;
  }

  const value = Number(raw);
  return Number.isFinite(value) ? value : null;
});

const currentTeam = computed(() => findTeamById(teamIdFromRoute.value));
const activeTeamId = computed(() => currentTeam.value?.id ?? preferredTeamId.value ?? firstTeamId.value);
const activeTeam = computed(() => findTeamById(activeTeamId.value));
const activeTeamEnabled = computed(() => {
  const enabled = activeTeam.value?.enabled as boolean | number | undefined;
  return enabled !== false && enabled !== 0;
});
const activeTeamEnabledPending = ref<Record<number, boolean>>({});
const isActiveTeamTogglePending = computed(() => (
  activeTeamId.value !== null ? Boolean(activeTeamEnabledPending.value[activeTeamId.value]) : false
));
const teamToggleConfirm = ref<{
  open: boolean;
  teamId: number | null;
  enabled: boolean;
}>({
  open: false,
  teamId: null,
  enabled: false,
});
const showTeamDisabledPill = computed(() => route.name === 'console' && !activeTeamEnabled.value);
const showTopbarConnectionStatus = computed(() => route.name === 'console');
const showTopbarBackToConsole = computed(() => route.name === 'settings');
const statusLabel = computed(() => formatConnectionState(connectionState.value));
const isLightMode = computed(() => themeMode.value === 'light');
const scheduleResumePending = ref(false);
const isConsoleRoute = computed(() => route.name === 'console');
const consoleView = computed<ConsoleMainView>(() => (route.query.view === 'tasks' ? 'tasks' : 'chat'));
const desktopStatus = ref<DesktopStatus | null>(null);
const desktopCommandPending = ref<'start' | 'stop' | 'open' | null>(null);
const isTauriDesktop = computed(() => isTauriRuntime());
const localBackendRunning = computed(() => desktopStatus.value?.backendRunning ?? false);
const desktopConnected = computed(() => localBackendRunning.value && connectionState.value === 'connected');
let removeViewportRootClasses: (() => void) | null = null;
let removeDesktopBackendListener: (() => void) | null = null;

// ── V13: Quick Init Modal ──

function getRequestErrorCountdownStyle(toast: GlobalRequestErrorToast): Record<string, string> | undefined {
  if (toast.dismissAt === null || toast.autoDismissMs === null) {
    return undefined;
  }

  return {
    '--countdown-duration': `${toast.autoDismissMs}ms`,
    '--countdown-delay': `${toast.countdownDelayMs}ms`,
  };
}

function formatRequestErrorDetail(toast: GlobalRequestErrorToast): string {
  const parts: string[] = [];
  if (toast.statusCode !== null) {
    parts.push(`${t('error.statusLabel')} ${toast.statusCode}`);
  }
  if (toast.detail) {
    parts.push(toast.detail);
  }
  return parts.join(' · ');
}

async function checkSystemStatus(): Promise<void> {
  try {
    const status = await getSystemStatus();
    showQuickInit.value = !status.initialized;
    authEnabled.value = status.auth_enabled ?? false;
    updateScheduleState(status.schedule_state ?? '', status.not_running_reason ?? '');
    setGlobalRequestErrorAutoDismiss(status.development_mode ? null : 5000);

    // 鉴权启用且无 token 时，触发 token 输入
    if (authEnabled.value && !getToken()) {
      showTokenDialog.value = true;
    }
  } catch {
    // Backend unreachable — don't show init modal
    showQuickInit.value = false;
    authEnabled.value = false;
    updateScheduleState('', '');
  }
}

function handleInitSkip(): void {
  showQuickInit.value = false;
}

function handleInitDone(): void {
  showQuickInit.value = false;
  // scheduleState will be updated via WebSocket event
}

function applyTheme(mode: ThemeMode): void {
  document.documentElement.dataset.theme = mode;
}

function toggleTheme(): void {
  themeMode.value = themeMode.value === 'dark' ? 'light' : 'dark';
}

function openSettings(): void {
  if (activeTeamId.value === null) {
    return;
  }
  router.push({
    name: 'settings',
    params: { teamId: activeTeamId.value, section: DEFAULT_SETTINGS_SECTION },
  }).catch(console.error);
}

function backToConsole(): void {
  if (activeTeamId.value === null) {
    router.push({ name: 'home' }).catch(console.error);
    return;
  }

  router.push({ name: 'console', params: { teamId: activeTeamId.value } }).catch(console.error);
}

function selectTeam(teamId: number): void {
  setPreferredTeamId(teamId);
  router.push({ name: 'console', params: { teamId } }).catch(console.error);
}

function switchConsoleView(view: ConsoleMainView): void {
  if (!isConsoleRoute.value) {
    return;
  }

  router.replace({
    query: {
      ...route.query,
      view: view === 'chat' ? undefined : view,
    },
  }).catch(console.error);
}

function requestActiveTeamEnabledToggle(enabled: boolean): void {
  if (!activeTeam.value) {
    return;
  }

  teamToggleConfirm.value = {
    open: true,
    teamId: activeTeam.value.id,
    enabled,
  };
}

function closeTeamToggleConfirm(): void {
  teamToggleConfirm.value = {
    open: false,
    teamId: null,
    enabled: false,
  };
}

async function updateTeamEnabledState(teamIdToUpdate: number, enabled: boolean): Promise<void> {
  if (activeTeamEnabledPending.value[teamIdToUpdate]) {
    return;
  }

  activeTeamEnabledPending.value = {
    ...activeTeamEnabledPending.value,
    [teamIdToUpdate]: true,
  };

  try {
    await setTeamEnabled(teamIdToUpdate, enabled);
    await loadTeams();
    showGlobalSuccessToast(enabled ? t('settings.page.teamEnabled') : t('settings.page.teamDisabled'));
  } catch (error) {
    console.error(error);
  } finally {
    const nextPending = { ...activeTeamEnabledPending.value };
    delete nextPending[teamIdToUpdate];
    activeTeamEnabledPending.value = nextPending;
  }
}

function confirmTeamToggle(): void {
  const { teamId: targetTeamId, enabled } = teamToggleConfirm.value;
  closeTeamToggleConfirm();
  if (targetTeamId === null) {
    return;
  }
  void updateTeamEnabledState(targetTeamId, enabled);
}

async function handleResumeSchedule(): Promise<void> {
  if (scheduleResumePending.value) {
    return;
  }

  scheduleResumePending.value = true;
  try {
    const result = await resumeSchedule();
    updateScheduleState(result.schedule_state ?? '', result.not_running_reason ?? '');
    if (String(result.schedule_state ?? '').trim().toLowerCase() === 'running') {
      showGlobalSuccessToast(t('topbar.resumeScheduleSuccess'));
    }
  } catch (error) {
    console.error(error);
  } finally {
    scheduleResumePending.value = false;
  }
}

async function refreshDesktopStatus(): Promise<void> {
  if (!isTauriDesktop.value) {
    desktopStatus.value = null;
    return;
  }

  try {
    desktopStatus.value = await getDesktopStatus();
  } catch (error) {
    console.error(error);
  }
}

async function handleStartLocalBackend(): Promise<void> {
  if (desktopCommandPending.value) {
    return;
  }

  desktopCommandPending.value = 'start';
  try {
    const status = await startLocalBackend();
    if (status) {
      desktopStatus.value = status;
    }
    showGlobalSuccessToast('TogoSpace 后端已启动');
  } catch (error) {
    console.error(error);
    showGlobalRequestError({
      title: '启动 TogoSpace 后端失败',
      path: 'desktop://start-backend',
      detail: error instanceof Error ? error.message : String(error),
    });
  } finally {
    desktopCommandPending.value = null;
  }
}

async function handleStopLocalBackend(): Promise<void> {
  if (desktopCommandPending.value) {
    return;
  }

  desktopCommandPending.value = 'stop';
  try {
    const status = await stopLocalBackend();
    if (status) {
      desktopStatus.value = status;
    }
    showGlobalSuccessToast('TogoSpace 后端已停止');
  } catch (error) {
    console.error(error);
    showGlobalRequestError({
      title: '停止 TogoSpace 后端失败',
      path: 'desktop://stop-backend',
      detail: error instanceof Error ? error.message : String(error),
    });
  } finally {
    desktopCommandPending.value = null;
  }
}

async function handleOpenLocalRoot(): Promise<void> {
  if (desktopCommandPending.value) {
    return;
  }

  desktopCommandPending.value = 'open';
  try {
    await openLocalTogoSpaceRoot();
  } catch (error) {
    console.error(error);
    showGlobalRequestError({
      title: '打开 TogoSpace 目录失败',
      path: 'desktop://open-root',
      detail: error instanceof Error ? error.message : String(error),
    });
  } finally {
    desktopCommandPending.value = null;
  }
}

function redirectToTeam(teamId: number | null): void {
  if (teamId === null) {
    return;
  }
  router.replace({ name: 'console', params: { teamId } }).catch(console.error);
}

watch(currentTeam, (team) => {
  if (team) {
    setPreferredTeamId(team.id);
  }
});

watch(
  [teamsLoaded, teamIdFromRoute, () => route.name],
  ([loaded, routeTeamId, routeName]) => {
    if (!loaded) {
      return;
    }

    if (routeName === 'home') {
      redirectToTeam(preferredTeamId.value ?? firstTeamId.value);
      return;
    }

    if (routeName === 'team-create') {
      return;
    }

    if (routeTeamId === null || !findTeamById(routeTeamId)) {
      redirectToTeam(preferredTeamId.value ?? firstTeamId.value);
    }
  },
  { immediate: true },
);

watch(
  themeMode,
  (mode) => {
    localStorage.setItem('theme-mode', mode);
    applyTheme(mode);
  },
  { immediate: true },
);

watch(
  () => desktopStatus.value?.backendRunning ?? false,
  (running, previousRunning) => {
    if (running && !previousRunning) {
      void Promise.all([loadTeams(), checkSystemStatus()]).catch(console.error);
    }
  },
);

onMounted(async () => {
  if (isTauriDesktop.value) {
    void appendDesktopDebugEvent({
      sessionId: 'drag-path-missing',
      runId: 'pre-fix',
      hypothesisId: 'D',
      location: 'src/App.vue',
      msg: '[DEBUG] app mounted',
      data: {
        routeName: String(route.name ?? ''),
      },
      ts: Date.now(),
    }).catch(() => {});
  }
  removeViewportRootClasses = installViewportRootClasses();
  applyTheme(themeMode.value);
  startRealtimeClient();
  if (isTauriDesktop.value) {
    removeDesktopBackendListener = await listenDesktopBackendStatus((status) => {
      desktopStatus.value = status;
    });
  }
  await Promise.all([loadTeams(), checkSystemStatus()]);
  await refreshDesktopStatus();
});

onBeforeUnmount(() => {
  removeViewportRootClasses?.();
  removeViewportRootClasses = null;
  removeDesktopBackendListener?.();
  removeDesktopBackendListener = null;
  stopRealtimeClient();
});
</script>

<template>
  <div class="shell" :class="{ 'shell-console': isConsoleRoute }">
    <div class="ambient ambient-left"></div>
    <div class="ambient ambient-right"></div>

    <TopBar
      :connection-state="connectionState"
      :is-light-mode="isLightMode"
      :status-label="statusLabel"
      :reconnect-progress="reconnectProgress"
      :total-message-count="totalMessageCount"
      :teams="teams"
      :active-team-id="activeTeamId"
      :active-team-enabled="activeTeamEnabled"
      :active-team-enabled-pending="isActiveTeamTogglePending"
      :show-team-disabled-pill="showTeamDisabledPill"
      :show-connection-status="showTopbarConnectionStatus"
      :show-back-to-console="showTopbarBackToConsole"
      :schedule-state="scheduleState"
      :schedule-not-running-reason="scheduleNotRunningReason"
      :schedule-resume-pending="scheduleResumePending"
      :auth-enabled="authEnabled"
      :show-console-view-tabs="isConsoleRoute"
      :console-view="consoleView"
      :is-tauri-desktop="isTauriDesktop"
      :desktop-backend-running="localBackendRunning"
      :desktop-connected="desktopConnected"
      :desktop-status="desktopStatus"
      :desktop-command-pending="desktopCommandPending"
      @toggle-theme="toggleTheme"
      @select-team="selectTeam"
      @toggle-active-team-enabled="requestActiveTeamEnabledToggle"
      @back-to-console="backToConsole"
      @open-settings="openSettings"
      @resume-schedule="handleResumeSchedule"
      @switch-console-view="switchConsoleView"
      @start-local-backend="handleStartLocalBackend"
      @stop-local-backend="handleStopLocalBackend"
      @open-local-root="handleOpenLocalRoot"
    />

    <Teleport to="body">
      <div class="global-error-toast-layer">
        <div
          v-for="toast in globalRequestErrors"
          :key="toast.id"
          class="global-error-toast"
          role="alert"
        >
          <div class="global-error-toast__header">
            <strong class="global-error-toast__title">{{ toast.title }}</strong>
            <div class="global-error-toast__actions">
              <button
                type="button"
                class="global-error-toast__close"
                :aria-label="t('common.closeAlert')"
                @click="clearGlobalRequestError(toast.id)"
              >
                <svg
                  v-if="toast.dismissAt !== null"
                  viewBox="0 0 36 36"
                  class="global-error-toast__countdown-svg"
                  :style="getRequestErrorCountdownStyle(toast)"
                  aria-hidden="true"
                >
                  <circle class="global-error-toast__countdown-track" cx="18" cy="18" r="15"></circle>
                  <circle class="global-error-toast__countdown-progress" cx="18" cy="18" r="15"></circle>
                </svg>
                <span class="global-error-toast__close-icon">×</span>
              </button>
            </div>
          </div>

          <div class="global-error-toast__line">
            <span class="global-error-toast__line-label">{{ t('error.pathLabel') }}</span>
            <code class="global-error-toast__path">{{ toast.path }}</code>
          </div>

          <div class="global-error-toast__line">
            <span class="global-error-toast__line-label">{{ t('error.detailLabel') }}</span>
            <p class="global-error-toast__detail">{{ formatRequestErrorDetail(toast) || t('error.noDetail') }}</p>
          </div>
        </div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div class="global-success-toast-layer">
        <div
          v-for="toast in globalSuccessToasts"
          :key="toast.id"
          class="global-success-toast"
          role="status"
          aria-live="polite"
        >
          <span>{{ toast.message }}</span>
        </div>
      </div>
    </Teleport>

    <main class="workspace" :class="{ 'workspace-console': isConsoleRoute }">
      <RouterView />
    </main>

    <QuickInitModal
      v-if="showQuickInit"
      @skip="handleInitSkip"
      @done="handleInitDone"
    />

    <TokenDialog
      v-if="showTokenDialog"
    />

    <ConfirmDialog
      :open="teamToggleConfirm.open"
      :title="teamToggleConfirm.enabled ? t('settings.page.toggleEnableTitle') : t('settings.page.toggleDisableTitle')"
      :message="teamToggleConfirm.enabled ? t('settings.page.toggleEnableMsg') : t('settings.page.toggleDisableMsg')"
      :confirm-label="teamToggleConfirm.enabled ? t('settings.page.toggleEnableBtn') : t('settings.page.toggleDisableBtn')"
      @close="closeTeamToggleConfirm"
      @confirm="confirmTeamToggle"
    />
  </div>
</template>

<style scoped>
.shell {
  position: relative;
  height: 100%;
  min-height: 100%;
  width: min(100%, 1720px);
  margin: 0 auto;
  padding: 16px 18px;
  overflow: hidden;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 14px;
  box-sizing: border-box;
}

.ambient {
  position: absolute;
  z-index: 0;
  width: 22rem;
  height: 22rem;
  border-radius: 999px;
  filter: blur(54px);
  opacity: 0.7;
  pointer-events: none;
}

.ambient-left {
  top: -7rem;
  left: -7rem;
  background: var(--shell-glow-left);
}

.ambient-right {
  right: -7rem;
  bottom: -9rem;
  background: var(--shell-glow-right);
}

.workspace {
  min-height: 0;
  height: 100%;
  overflow: hidden;
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-rows: minmax(0, 1fr);
  padding: 4px 0 0;
}

:global(html.bp-console-mobile) .shell {
  height: auto;
  min-height: 100%;
  width: 100%;
  padding: 8px 8px calc(12px + env(safe-area-inset-bottom, 0px));
  overflow-x: hidden;
  overflow-y: auto;
}

:global(html.bp-console-mobile) .workspace {
  height: auto;
  overflow: visible;
}

:global(html.bp-console-mobile) .shell.shell-console {
  height: 100%;
  min-height: 0;
  padding: 12px 0 0;
  gap: 6px;
  overflow: hidden;
}

:global(html.bp-console-mobile) .workspace.workspace-console {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

:global(html.bp-console-short) .shell.shell-console {
  padding: 8px 0 0;
  gap: 6px;
}

.global-error-toast-layer {
  position: fixed;
  top: 8px;
  right: 8px;
  z-index: 140;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 12px;
  pointer-events: none;
}

.global-error-toast {
  display: grid;
  gap: 10px;
  width: min(392px, calc(100vw - 16px));
  padding: 14px 15px;
  border: 1px solid color-mix(in srgb, var(--state-danger) 38%, var(--border-default) 62%);
  border-radius: 18px;
  background: color-mix(in srgb, var(--surface-overlay) 90%, var(--state-danger) 10%);
  color: color-mix(in srgb, var(--state-danger) 72%, var(--text-primary) 28%);
  box-shadow: 0 16px 32px rgba(0, 0, 0, 0.18);
  pointer-events: auto;
}

.global-error-toast__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
}

.global-error-toast__title {
  min-width: 0;
  margin: 0;
  color: color-mix(in srgb, var(--state-danger) 92%, var(--text-primary) 8%);
  font-size: 1.08rem;
  font-weight: 700;
  line-height: 1.15;
}

.global-error-toast__actions {
  display: flex;
  align-items: center;
  gap: 0;
  flex-shrink: 0;
}

.global-error-toast__close {
  position: relative;
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border: none;
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface-panel) 82%, transparent);
  color: inherit;
  cursor: pointer;
  padding: 0;
  line-height: 1;
  font-size: 1.08rem;
  transition: transform 0.15s ease, background 0.15s ease;
  isolation: isolate;
  box-shadow:
    inset 0 0 0 1px color-mix(in srgb, var(--border-default) 72%, transparent),
    0 0 0 1px color-mix(in srgb, var(--state-danger) 10%, transparent);
}

.global-error-toast__countdown-svg {
  position: absolute;
  inset: 0;
  display: block;
  width: 34px;
  height: 34px;
  transform: rotate(-90deg);
  overflow: visible;
  pointer-events: none;
  z-index: 0;
}

.global-error-toast__countdown-track,
.global-error-toast__countdown-progress {
  fill: none;
  stroke-width: 4;
}

.global-error-toast__countdown-track {
  stroke: color-mix(in srgb, var(--state-danger) 14%, var(--border-default) 86%);
}

.global-error-toast__countdown-progress {
  stroke: color-mix(in srgb, var(--state-danger) 82%, white 18%);
  stroke-linecap: round;
  stroke-dasharray: 94.2478;
  stroke-dashoffset: 94.2478;
  animation-name: error-toast-countdown;
  animation-duration: var(--countdown-duration, 5000ms);
  animation-timing-function: linear;
  animation-delay: var(--countdown-delay, 0ms);
  animation-fill-mode: forwards;
  will-change: stroke-dashoffset;
}

.global-error-toast__close:hover {
  transform: translateY(-1px);
  background: color-mix(in srgb, var(--state-danger) 10%, var(--surface-panel) 90%);
}

.global-error-toast__close-icon {
  position: relative;
  z-index: 1;
  display: block;
  transform: translateY(-0.5px);
}

.global-error-toast__line-label {
  display: block;
  margin-bottom: 2px;
  color: color-mix(in srgb, var(--text-secondary) 70%, var(--state-danger) 30%);
  font-size: 0.69rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.global-error-toast__line {
  display: grid;
  gap: 3px;
}

.global-error-toast__path {
  display: block;
  color: color-mix(in srgb, var(--state-info) 68%, var(--text-primary) 32%);
  font-family: "IBM Plex Mono", "SFMono-Regular", Consolas, monospace;
  font-size: 0.78rem;
  line-height: 1.42;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
}

.global-error-toast__detail {
  margin: 0;
  color: color-mix(in srgb, var(--state-danger) 88%, var(--text-primary) 12%);
  font-size: 0.92rem;
  line-height: 1.42;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  word-break: break-word;
}

.global-success-toast {
  width: min(360px, calc(100vw - 40px));
  min-height: 44px;
  padding: 10px 18px;
  border: 1px solid color-mix(in srgb, var(--state-success) 42%, var(--border-default) 58%);
  border-radius: 14px;
  background: color-mix(in srgb, var(--surface-panel) 88%, var(--state-success) 12%);
  color: color-mix(in srgb, var(--state-success) 78%, var(--text-primary) 22%);
  box-shadow: 0 12px 28px rgba(0, 0, 0, 0.12);
  font-size: 0.92rem;
  line-height: 1.3;
  text-align: center;
  pointer-events: none;
  animation: success-toast-drop 0.22s ease-out;
}

.global-success-toast-layer {
  position: fixed;
  top: 18px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 140;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  pointer-events: none;
}

.global-success-toast span {
  flex: 1;
}

@keyframes success-toast-drop {
  from {
    opacity: 0;
    transform: translateY(-12px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes error-toast-countdown {
  from {
    stroke-dashoffset: 94.2478;
  }

  to {
    stroke-dashoffset: 0;
  }
}
</style>
