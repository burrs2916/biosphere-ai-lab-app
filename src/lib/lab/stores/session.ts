import { writable } from 'svelte/store';
import type { SessionInfo, SessionStatus, LabEvent, EpochMetrics } from '../adapter/types';
import { onLabEvent } from '../adapter/events';

function createSessionStore() {
  const { subscribe, set, update } = writable<Map<string, SessionInfo>>(new Map());
  const metricsStore = writable<Map<string, EpochMetrics[]>>(new Map());

  let unlisten: (() => void) | null = null;

  return {
    subscribe,
    metrics: { subscribe: metricsStore.subscribe },
    addSession(session: SessionInfo) {
      update((sessions) => {
        const next = new Map(sessions);
        next.set(session.id, session);
        return next;
      });
    },
    updateMetrics(sessionId: string, metrics: EpochMetrics[]) {
      metricsStore.update((map) => {
        const next = new Map(map);
        next.set(sessionId, metrics);
        return next;
      });
    },
    startListening() {
      if (unlisten) return;
      onLabEvent((event) => {
        update((sessions) => {
          const next = new Map(sessions);
          if (event.type === 'SessionStarted') {
            const sid = event.payload.session_id;
            const existing = next.get(sid);
            if (existing) {
              next.set(sid, { ...existing, status: 'training' as SessionStatus });
            }
          } else if (event.type === 'EpochCompleted') {
            const sid = event.payload.session_id;
            const existing = next.get(sid);
            if (existing) {
              next.set(sid, { ...existing, current_epoch: event.payload.epoch, total_epochs: event.payload.total_epochs });
            }
            metricsStore.update((map) => {
              const nextMap = new Map(map);
              const existingMetrics = nextMap.get(sid) || [];
              nextMap.set(sid, [...existingMetrics, {
                epoch: event.payload.epoch,
                train_loss: event.payload.train_loss,
                val_loss: event.payload.val_loss,
                metrics: (event.payload.metrics as Record<string, number>) || {},
                learning_rate: 0,
                epoch_time_ms: 0,
              }]);
              return nextMap;
            });
          } else if (event.type === 'SessionCompleted') {
            const sid = event.payload.session_id;
            const existing = next.get(sid);
            if (existing) {
              next.set(sid, { ...existing, status: 'completed' as SessionStatus });
            }
          } else if (event.type === 'SessionFailed') {
            const sid = event.payload.session_id;
            const existing = next.get(sid);
            if (existing) {
              next.set(sid, { ...existing, status: 'failed' as SessionStatus, error_message: event.payload.error });
            }
          } else if (event.type === 'SessionPaused') {
            const sid = event.payload.session_id;
            const existing = next.get(sid);
            if (existing) {
              next.set(sid, { ...existing, status: 'paused' as SessionStatus });
            }
          } else if (event.type === 'SessionResumed') {
            const sid = event.payload.session_id;
            const existing = next.get(sid);
            if (existing) {
              next.set(sid, { ...existing, status: 'training' as SessionStatus });
            }
          }
          return next;
        });
      }).then((fn) => { unlisten = fn; });
    },
    stopListening() {
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    },
  };
}

export const sessionStore = createSessionStore();
