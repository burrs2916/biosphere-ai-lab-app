import { writable, get } from 'svelte/store';
import { toastStore } from './toast';

export interface NotificationEntry {
  id: number;
  type: 'training_completed' | 'training_failed' | 'training_cancelled' | 'info';
  title: string;
  message: string;
  experimentId?: string;
  timestamp: number;
  read: boolean;
}

function createNotificationStore() {
  const { subscribe, update } = writable<NotificationEntry[]>([]);
  let nextId = 0;

  function notify(type: NotificationEntry['type'], title: string, message: string, experimentId?: string) {
    const id = nextId++;
    const entry: NotificationEntry = {
      id,
      type,
      title,
      message,
      experimentId,
      timestamp: Date.now(),
      read: false,
    };

    update((notifications) => [entry, ...notifications].slice(0, 100));

    if (type === 'training_completed') {
      toastStore.success(title);
    } else if (type === 'training_failed') {
      toastStore.error(title);
    } else if (type === 'training_cancelled') {
      toastStore.warning(title);
    } else {
      toastStore.info(title);
    }

    if (document.hidden && 'Notification' in window) {
      try {
        if (Notification.permission === 'granted') {
          new Notification(title, { body: message });
        } else if (Notification.permission !== 'denied') {
          Notification.requestPermission().then((perm) => {
            if (perm === 'granted') {
              new Notification(title, { body: message });
            }
          });
        }
      } catch {}
    }

    return id;
  }

  function markRead(id: number) {
    update((notifications) =>
      notifications.map((n) => (n.id === id ? { ...n, read: true } : n))
    );
  }

  function markAllRead() {
    update((notifications) => notifications.map((n) => ({ ...n, read: true })));
  }

  function clear(id: number) {
    update((notifications) => notifications.filter((n) => n.id !== id));
  }

  function clearAll() {
    update(() => []);
  }

  function unreadCount(): number {
    let count = 0;
    const unsub = subscribe((n) => { count = n.filter((x) => !x.read).length; });
    unsub();
    return count;
  }

  return { subscribe, notify, markRead, markAllRead, clear, clearAll, unreadCount };
}

export const notificationStore = createNotificationStore();

export async function requestNotificationPermission(): Promise<boolean> {
  if (!('Notification' in window)) return false;
  if (Notification.permission === 'granted') return true;
  if (Notification.permission === 'denied') return false;
  const perm = await Notification.requestPermission();
  return perm === 'granted';
}
