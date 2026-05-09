import { writable, derived, get } from 'svelte/store';
import { getLabClient } from './plugins';
import { i18n } from '$lib/i18n';

export type ConnectionStatus = 'connected' | 'disconnected' | 'connecting';

export interface StatusBarState {
  connectionStatus: ConnectionStatus;
  lastRefreshAt: Date | null;
  refreshCount: number;
  activeExperiments: number;
  runningExperiments: number;
  backendInfo: string;
  computeBackend: string;
  uptime: number;
}

function createStatusBarStore() {
  const { subscribe, set, update } = writable<StatusBarState>({
    connectionStatus: 'disconnected',
    lastRefreshAt: null,
    refreshCount: 0,
    activeExperiments: 0,
    runningExperiments: 0,
    backendInfo: '',
    computeBackend: 'cpu',
    uptime: 0,
  });

  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let uptimeTimer: ReturnType<typeof setInterval> | null = null;
  let startTime: Date | null = null;

  return {
    subscribe,

    async initialize() {
      update((s) => ({ ...s, connectionStatus: 'connecting' }));
      try {
        const client = getLabClient();
        const [state, hardware] = await Promise.all([
          client.getState(),
          client.getHardwareInfo().catch(() => null),
        ]);

        const backend = hardware?.gpu_devices?.length
          ? hardware.gpu_devices[0].compute_backend
          : 'cpu';

        update((s) => ({
          ...s,
          connectionStatus: 'connected',
          activeExperiments: state.active_experiments,
          runningExperiments: state.active_experiments,
          backendInfo: hardware
            ? `${hardware.cpu_model} | ${Math.round(hardware.total_memory_mb / 1024)}GB`
            : '',
          computeBackend: backend,
        }));

        startTime = new Date();
      } catch {
        update((s) => ({ ...s, connectionStatus: 'disconnected' }));
      }
    },

    startAutoRefresh(intervalSeconds: number = 5) {
      if (refreshTimer) clearInterval(refreshTimer);
      refreshTimer = setInterval(async () => {
        try {
          const client = getLabClient();
          const state = await client.getState();
          update((s) => ({
            ...s,
            connectionStatus: 'connected',
            lastRefreshAt: new Date(),
            refreshCount: s.refreshCount + 1,
            activeExperiments: state.active_experiments,
            runningExperiments: state.active_experiments,
          }));
        } catch {
          update((s) => ({ ...s, connectionStatus: 'disconnected' }));
        }
      }, intervalSeconds * 1000);

      if (uptimeTimer) clearInterval(uptimeTimer);
      uptimeTimer = setInterval(() => {
        if (startTime) {
          update((s) => ({
            ...s,
            uptime: Math.floor((Date.now() - startTime!.getTime()) / 1000),
          }));
        }
      }, 1000);
    },

    stopAutoRefresh() {
      if (refreshTimer) {
        clearInterval(refreshTimer);
        refreshTimer = null;
      }
      if (uptimeTimer) {
        clearInterval(uptimeTimer);
        uptimeTimer = null;
      }
    },

    setConnectionStatus(status: ConnectionStatus) {
      update((s) => ({ ...s, connectionStatus: status }));
    },

    incrementRunning() {
      update((s) => ({
        ...s,
        runningExperiments: s.runningExperiments + 1,
        activeExperiments: s.activeExperiments + 1,
      }));
    },

    decrementRunning() {
      update((s) => ({
        ...s,
        runningExperiments: Math.max(0, s.runningExperiments - 1),
      }));
    },
  };
}

export const statusBarStore = createStatusBarStore();

export const connectionLabel = derived(statusBarStore, ($s) => {
  const t = get(i18n.t);
  switch ($s.connectionStatus) {
    case 'connected': return t('status.connected');
    case 'connecting': return t('status.connecting');
    case 'disconnected': return t('status.disconnected');
  }
});

export const uptimeLabel = derived(statusBarStore, ($s) => {
  const s = $s.uptime;
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`;
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return `${h}h ${m}m`;
});

export const lastRefreshLabel = derived(statusBarStore, ($s) => {
  const t = get(i18n.t);
  if (!$s.lastRefreshAt) return t('status.notRefreshed');
  const diff = Math.floor((Date.now() - $s.lastRefreshAt.getTime()) / 1000);
  if (diff < 5) return t('status.justRefreshed');
  if (diff < 60) return t('status.secondsAgoStatus', { seconds: diff });
  return t('status.minutesAgoStatus', { minutes: Math.floor(diff / 60) });
});
