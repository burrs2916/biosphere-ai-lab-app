import { writable, derived } from 'svelte/store';
import { getLabClient } from './plugins';
import { onLabEvent } from '../adapter/events';
import type { DashboardStats, LabEvent } from '../adapter/types';

function createDashboardStore() {
  const { subscribe, set, update } = writable<DashboardStats>({
    total_experiments: 0,
    running_experiments: 0,
    completed_experiments: 0,
    failed_experiments: 0,
    total_models: 0,
    production_models: 0,
    status_counts: {},
    task_type_counts: {},
  });

  const { subscribe: loadingSub, set: setLoading } = writable(false);
  let unlisten: (() => void) | null = null;

  return {
    subscribe,
    loading: { subscribe: loadingSub },

    async refresh() {
      setLoading(true);
      try {
        const client = getLabClient();
        const stats = await client.getDashboardStats();
        set(stats);
      } catch (e) {
        console.error('Failed to load dashboard stats:', e);
      } finally {
        setLoading(false);
      }
    },

    startListening() {
      if (unlisten) return;
      onLabEvent((event: LabEvent) => {
        switch (event.type) {
          case 'SessionCompleted':
            update((s) => ({
              ...s,
              running_experiments: Math.max(0, s.running_experiments - 1),
              completed_experiments: s.completed_experiments + 1,
            }));
            break;
          case 'SessionFailed':
            update((s) => ({
              ...s,
              running_experiments: Math.max(0, s.running_experiments - 1),
              failed_experiments: s.failed_experiments + 1,
            }));
            break;
          case 'SessionStarted':
            update((s) => ({
              ...s,
              running_experiments: s.running_experiments + 1,
              total_experiments: s.total_experiments + 1,
            }));
            break;
          case 'SessionCancelled':
            update((s) => ({
              ...s,
              running_experiments: Math.max(0, s.running_experiments - 1),
            }));
            break;
        }
      }).then((fn) => { unlisten = fn; });
    },

    stopListening() {
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    },

    incrementRunning() {
      update((s) => ({
        ...s,
        running_experiments: s.running_experiments + 1,
        total_experiments: s.total_experiments + 1,
      }));
    },

    decrementRunning() {
      update((s) => ({
        ...s,
        running_experiments: Math.max(0, s.running_experiments - 1),
      }));
    },

    markCompleted() {
      update((s) => ({
        ...s,
        running_experiments: Math.max(0, s.running_experiments - 1),
        completed_experiments: s.completed_experiments + 1,
      }));
    },

    markFailed() {
      update((s) => ({
        ...s,
        running_experiments: Math.max(0, s.running_experiments - 1),
        failed_experiments: s.failed_experiments + 1,
      }));
    },
  };
}

export const dashboardStore = createDashboardStore();

export const successRate = derived(dashboardStore, ($s) => {
  const total = $s.completed_experiments + $s.failed_experiments;
  if (total === 0) return 0;
  return Math.round(($s.completed_experiments / total) * 100);
});
