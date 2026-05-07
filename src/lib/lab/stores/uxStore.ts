import { writable, derived } from 'svelte/store';

interface Notification {
  id: string;
  type: 'success' | 'error' | 'warning' | 'info';
  title: string;
  message: string;
  timestamp: number;
  autoDismiss: boolean;
  duration: number;
}

interface OptimisticUpdate {
  id: string;
  type: string;
  targetId: string;
  previousState: any;
  timestamp: number;
}

function createUXStore() {
  const { subscribe, update } = writable<{
    notifications: Notification[];
    optimisticUpdates: OptimisticUpdate[];
    keyboardShortcutsEnabled: boolean;
    lastRefreshTime: number | null;
  }>({
    notifications: [],
    optimisticUpdates: [],
    keyboardShortcutsEnabled: true,
    lastRefreshTime: null,
  });

  let notifCounter = 0;

  return {
    subscribe,

    notify(type: Notification['type'], title: string, message: string, autoDismiss = true, duration = 4000) {
      const id = `notif-${++notifCounter}`;
      update(s => ({
        ...s,
        notifications: [...s.notifications, { id, type, title, message, timestamp: Date.now(), autoDismiss, duration }],
      }));
      if (autoDismiss) {
        setTimeout(() => this.dismiss(id), duration);
      }
      return id;
    },

    success(title: string, message: string) { return this.notify('success', title, message); },
    error(title: string, message: string) { return this.notify('error', title, message, true, 6000); },
    warning(title: string, message: string) { return this.notify('warning', title, message); },
    info(title: string, message: string) { return this.notify('info', title, message); },

    dismiss(id: string) {
      update(s => ({
        ...s,
        notifications: s.notifications.filter(n => n.id !== id),
      }));
    },

    clearAll() {
      update(s => ({ ...s, notifications: [] }));
    },

    addOptimisticUpdate(type: string, targetId: string, previousState: any) {
      const id = `opt-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
      update(s => ({
        ...s,
        optimisticUpdates: [...s.optimisticUpdates, { id, type, targetId, previousState, timestamp: Date.now() }],
      }));
      return id;
    },

    commitOptimisticUpdate(id: string) {
      update(s => ({
        ...s,
        optimisticUpdates: s.optimisticUpdates.filter(u => u.id !== id),
      }));
    },

    rollbackOptimisticUpdate(id: string) {
      let rolledBack: OptimisticUpdate | null = null;
      update(s => {
        const target = s.optimisticUpdates.find(u => u.id === id);
        if (target) rolledBack = target;
        return {
          ...s,
          optimisticUpdates: s.optimisticUpdates.filter(u => u.id !== id),
        };
      });
      return rolledBack;
    },

    markRefresh() {
      update(s => ({ ...s, lastRefreshTime: Date.now() }));
    },
  };
}

export const uxStore = createUXStore();
