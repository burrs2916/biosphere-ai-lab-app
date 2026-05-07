import { writable, derived } from 'svelte/store';
import { getLabClient } from './plugins';

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
  switch ($s.connectionStatus) {
    case 'connected': return '已连接';
    case 'connecting': return '连接中...';
    case 'disconnected': return '未连接';
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
  if (!$s.lastRefreshAt) return '未刷新';
  const diff = Math.floor((Date.now() - $s.lastRefreshAt.getTime()) / 1000);
  if (diff < 5) return '刚刚刷新';
  if (diff < 60) return `${diff}秒前`;
  return `${Math.floor(diff / 60)}分钟前`;
});
