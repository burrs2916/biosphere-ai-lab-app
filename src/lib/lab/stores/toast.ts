import { writable } from 'svelte/store';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
  duration: number;
}

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);

  let nextId = 0;

  function add(type: ToastType, message: string, duration: number = 4000) {
    const id = nextId++;
    update((toasts) => [...toasts, { id, type, message, duration }]);

    if (duration > 0) {
      setTimeout(() => {
        remove(id);
      }, duration);
    }

    return id;
  }

  function remove(id: number) {
    update((toasts) => toasts.filter(t => t.id !== id));
  }

  function success(message: string) { return add('success', message); }
  function error(message: string) { return add('error', message, 6000); }
  function warning(message: string) { return add('warning', message, 5000); }
  function info(message: string) { return add('info', message); }

  return { subscribe, add, remove, success, error, warning, info };
}

export const toastStore = createToastStore();
