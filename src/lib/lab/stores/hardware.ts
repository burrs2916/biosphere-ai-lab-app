import { writable } from 'svelte/store';
import { getLabClient } from './plugins';
import type { HardwareInfo, TrainingRecommendation, TaskType, ResourceSnapshot } from '../adapter/types';

function createHardwareStore() {
  const { subscribe, set, update } = writable<HardwareInfo | null>(null);
  const { subscribe: loadingSub, set: setLoading } = writable(false);

  return {
    subscribe,
    loading: { subscribe: loadingSub },
    async refresh() {
      setLoading(true);
      try {
        const client = getLabClient();
        const info = await client.getHardwareInfo();
        set(info);
      } catch (e) {
        console.error('Failed to get hardware info:', e);
      } finally {
        setLoading(false);
      }
    },
  };
}

function createRecommendationStore() {
  const { subscribe, set } = writable<TrainingRecommendation | null>(null);

  return {
    subscribe,
    async getRecommendations(hardware: HardwareInfo, taskType: TaskType, dataSize: number) {
      try {
        const client = getLabClient();
        const rec = await client.getRecommendations(hardware, taskType, dataSize);
        set(rec);
      } catch (e) {
        console.error('Failed to get recommendations:', e);
      }
    },
  };
}

const MAX_HISTORY = 360;

function createResourceStore() {
  const { subscribe, set, update } = writable<ResourceSnapshot | null>(null);
  const { subscribe: historySub, update: updateHistory } = writable<ResourceSnapshot[]>([]);
  let unlisten: (() => void) | null = null;
  let fallbackTimer: ReturnType<typeof setInterval> | null = null;

  function handleHardwareAlert(payload: {
    cpu_usage: number;
    memory_usage: number;
    memory_total_mb?: number;
    memory_available_mb?: number;
    disk_total_gb?: number;
    disk_available_gb?: number;
    disk_usage_percent?: number;
    gpu_usage?: number | null;
    gpu_memory_used_mb?: number | null;
    gpu_memory_total_mb?: number | null;
  }) {
    const snapshot: ResourceSnapshot = {
      cpu_usage_percent: payload.cpu_usage,
      memory_total_mb: payload.memory_total_mb ?? 0,
      memory_available_mb: payload.memory_available_mb ?? 0,
      memory_usage_percent: payload.memory_usage,
      disk_total_gb: payload.disk_total_gb ?? 0,
      disk_available_gb: payload.disk_available_gb ?? 0,
      disk_usage_percent: payload.disk_usage_percent ?? 0,
      gpu_usage_percent: payload.gpu_usage ?? null,
      gpu_memory_used_mb: payload.gpu_memory_used_mb ?? null,
      gpu_memory_total_mb: payload.gpu_memory_total_mb ?? null,
      timestamp: Date.now(),
    };
    set(snapshot);
    updateHistory((prev) => [...prev.slice(-(MAX_HISTORY - 1)), snapshot]);
  }

  return {
    subscribe,
    history: { subscribe: historySub },

    async refresh() {
      try {
        const client = getLabClient();
        const snapshot = await client.getResourceSnapshot();
        set(snapshot);
        updateHistory((prev) => [...prev.slice(-(MAX_HISTORY - 1)), snapshot]);
      } catch (e) {
        console.error('Failed to get resource snapshot:', e);
      }
    },

    startListening() {
      if (unlisten) return;
      const client = getLabClient();
      client.onLabEvent((event) => {
        if (event.type === 'HardwareAlert') {
          const payload = event.payload as {
            cpu_usage: number;
            memory_usage: number;
            memory_total_mb?: number;
            memory_available_mb?: number;
            disk_total_gb?: number;
            disk_available_gb?: number;
            disk_usage_percent?: number;
            gpu_usage?: number | null;
            gpu_memory_used_mb?: number | null;
            gpu_memory_total_mb?: number | null;
          };
          handleHardwareAlert(payload);

          if (fallbackTimer) {
            clearInterval(fallbackTimer);
            fallbackTimer = null;
          }
        }
      }).then((fn) => { unlisten = fn; });

      this.refresh();
      fallbackTimer = setInterval(() => this.refresh(), 10000);
    },

    stopListening() {
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
      if (fallbackTimer) {
        clearInterval(fallbackTimer);
        fallbackTimer = null;
      }
    },

    startAutoRefresh(intervalSeconds: number = 5) {
      this.startListening();
    },

    stopAutoRefresh() {
      this.stopListening();
    },
  };
}

export const hardwareStore = createHardwareStore();
export const recommendationStore = createRecommendationStore();
export const resourceStore = createResourceStore();
