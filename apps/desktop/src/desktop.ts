export interface DesktopStatus {
  tauriAvailable: boolean;
  backendRunning: boolean;
  backendPid: number | null;
  togospaceRoot: string | null;
  appVersion: string;
}

const BACKEND_STATUS_EVENT = 'desktop://backend-status';
const REALTIME_EVENT = 'desktop://realtime-event';
const REALTIME_STATUS_EVENT = 'desktop://realtime-status';

export interface DesktopRealtimeStatus {
  state: 'connecting' | 'connected' | 'reconnecting' | 'disconnected';
  detail: string | null;
}

export interface DesktopHttpRequest {
  url: string;
  method: string;
  headers?: Record<string, string>;
  body?: string | null;
}

export interface DesktopHttpResponse {
  status: number;
  contentType: string | null;
  headers: Record<string, string>;
  body: string;
}

export interface DesktopDebugEvent {
  sessionId: string;
  runId: string;
  hypothesisId: string;
  location: string;
  msg: string;
  data: Record<string, unknown>;
  ts: number;
}

export type DesktopDragDropEvent = {
  type: 'enter';
  paths: string[];
} | {
  type: 'over';
} | {
  type: 'drop';
  paths: string[];
} | {
  type: 'leave';
};

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function invokeDesktop<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

export async function getDesktopStatus(): Promise<DesktopStatus | null> {
  return invokeDesktop<DesktopStatus>('get_desktop_status');
}

export async function startLocalBackend(): Promise<DesktopStatus | null> {
  return invokeDesktop<DesktopStatus>('start_togospace_backend');
}

export async function stopLocalBackend(): Promise<DesktopStatus | null> {
  return invokeDesktop<DesktopStatus>('stop_togospace_backend');
}

export async function openLocalTogoSpaceRoot(): Promise<void> {
  await invokeDesktop('open_togospace_root');
}

export async function openPathInFinder(path: string): Promise<void> {
  await invokeDesktop('reveal_path_in_finder', { path });
}

export async function appendDesktopDebugEvent(event: DesktopDebugEvent): Promise<void> {
  await invokeDesktop('append_desktop_debug_event', { event });
}

export async function listenDesktopDragDropEvent(
  callback: (event: DesktopDragDropEvent) => void,
): Promise<(() => void) | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { getCurrentWebview } = await import('@tauri-apps/api/webview');
  return getCurrentWebview().onDragDropEvent((event) => {
    callback(event.payload as DesktopDragDropEvent);
  });
}

export async function startDesktopRealtimeBridge(token?: string | null): Promise<void> {
  await invokeDesktop('start_desktop_realtime_bridge', { token: token ?? null });
}

export async function stopDesktopRealtimeBridge(): Promise<void> {
  await invokeDesktop('stop_desktop_realtime_bridge');
}

export async function desktopHttpRequest(
  request: DesktopHttpRequest,
): Promise<DesktopHttpResponse | null> {
  return invokeDesktop<DesktopHttpResponse>('desktop_http_request', { request });
}

export async function listenDesktopBackendStatus(
  callback: (status: DesktopStatus) => void,
): Promise<(() => void) | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { listen } = await import('@tauri-apps/api/event');
  return listen<DesktopStatus>(BACKEND_STATUS_EVENT, (event) => {
    callback(event.payload);
  });
}

export async function listenDesktopRealtimeEvent(
  callback: (payload: string) => void,
): Promise<(() => void) | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { listen } = await import('@tauri-apps/api/event');
  return listen<string>(REALTIME_EVENT, (event) => {
    callback(event.payload);
  });
}

export async function listenDesktopRealtimeStatus(
  callback: (status: DesktopRealtimeStatus) => void,
): Promise<(() => void) | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { listen } = await import('@tauri-apps/api/event');
  return listen<DesktopRealtimeStatus>(REALTIME_STATUS_EVENT, (event) => {
    callback(event.payload);
  });
}
