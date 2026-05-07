import { writable, derived } from 'svelte/store';
import type { SessionId, LabEvent } from '../adapter/types';
import { onLabEvent } from '../adapter/events';

export interface TrainingProgress {
  sessionId: string;
  experimentId: string | null;
  progress: number;
  currentEpoch: number;
  totalEpochs: number;
  currentBatch: number;
  totalBatches: number;
  message: string;
  startedAt: Date | null;
  lastUpdatedAt: Date | null;
  estimatedTimeRemaining: number | null;
  epochTimes: number[];
}

function createProgressStore() {
  const { subscribe, update, set } = writable<Map<string, TrainingProgress>>(new Map());

  let unlisten: (() => void) | null = null;
  const epochStartTimes = new Map<string, number>();
  const sessionToExperiment = new Map<string, string>();

  return {
    subscribe,

    getByExperimentId(experimentId: string): TrainingProgress | undefined {
      let result: TrainingProgress | undefined;
      const unsub = subscribe((map) => {
        for (const progress of map.values()) {
          if (progress.experimentId === experimentId || progress.sessionId === experimentId) {
            result = progress;
            break;
          }
        }
      });
      unsub();
      return result;
    },

    startListening() {
      if (unlisten) return;
      onLabEvent((event: LabEvent) => {
        update((map) => {
          const next = new Map(map);

          switch (event.type) {
            case 'SessionStarted': {
              const sid = event.payload.session_id;
              const expId = sessionToExperiment.get(sid) || null;
              next.set(sid, {
                sessionId: sid,
                experimentId: expId,
                progress: 0,
                currentEpoch: 0,
                totalEpochs: 0,
                currentBatch: 0,
                totalBatches: 0,
                message: '训练已启动',
                startedAt: new Date(),
                lastUpdatedAt: new Date(),
                estimatedTimeRemaining: null,
                epochTimes: [],
              });
              epochStartTimes.set(sid, Date.now());
              break;
            }
            case 'EpochCompleted': {
              const sid = event.payload.session_id;
              const existing = next.get(sid);
              if (existing) {
                const now = Date.now();
                const epochStartTime = epochStartTimes.get(sid) || now;
                const epochDuration = (now - epochStartTime) / 1000;
                epochStartTimes.set(sid, now);

                const newEpochTimes = [...existing.epochTimes, epochDuration].slice(-10);
                const avgEpochTime = newEpochTimes.reduce((a, b) => a + b, 0) / newEpochTimes.length;
                const remainingEpochs = event.payload.total_epochs - event.payload.epoch;
                const eta = remainingEpochs * avgEpochTime;

                next.set(sid, {
                  ...existing,
                  progress: event.payload.epoch / event.payload.total_epochs,
                  currentEpoch: event.payload.epoch,
                  totalEpochs: event.payload.total_epochs,
                  message: event.payload.val_loss != null
                    ? `Epoch ${event.payload.epoch}/${event.payload.total_epochs} - loss: ${event.payload.train_loss.toFixed(4)} - val_loss: ${event.payload.val_loss.toFixed(4)}`
                    : `Epoch ${event.payload.epoch}/${event.payload.total_epochs} - loss: ${event.payload.train_loss.toFixed(4)}`,
                  lastUpdatedAt: new Date(),
                  estimatedTimeRemaining: eta,
                  epochTimes: newEpochTimes,
                });
              } else {
                next.set(sid, {
                  sessionId: sid,
                  experimentId: sessionToExperiment.get(sid) || null,
                  progress: event.payload.epoch / event.payload.total_epochs,
                  currentEpoch: event.payload.epoch,
                  totalEpochs: event.payload.total_epochs,
                  currentBatch: 0,
                  totalBatches: 0,
                  message: `Epoch ${event.payload.epoch}/${event.payload.total_epochs}`,
                  startedAt: null,
                  lastUpdatedAt: new Date(),
                  estimatedTimeRemaining: null,
                  epochTimes: [],
                });
                epochStartTimes.set(sid, Date.now());
              }
              break;
            }
            case 'BatchCompleted': {
              const sid = event.payload.session_id;
              const existing = next.get(sid);
              if (existing) {
                const epochProgress = existing.totalEpochs > 0
                  ? (existing.currentEpoch - 1 + event.payload.batch / event.payload.total_batches) / existing.totalEpochs
                  : event.payload.batch / event.payload.total_batches * 0.1;

                next.set(sid, {
                  ...existing,
                  progress: epochProgress,
                  currentBatch: event.payload.batch,
                  totalBatches: event.payload.total_batches,
                  message: `Batch ${event.payload.batch}/${event.payload.total_batches} - loss: ${event.payload.loss.toFixed(4)}`,
                  lastUpdatedAt: new Date(),
                });
              }
              break;
            }
            case 'ProgressUpdate': {
              const sid = event.payload.session_id;
              const existing = next.get(sid);
              if (existing) {
                next.set(sid, {
                  ...existing,
                  progress: event.payload.progress,
                  message: event.payload.message,
                  lastUpdatedAt: new Date(),
                });
              } else {
                next.set(sid, {
                  sessionId: sid,
                  experimentId: sessionToExperiment.get(sid) || null,
                  progress: event.payload.progress,
                  currentEpoch: 0,
                  totalEpochs: 0,
                  currentBatch: 0,
                  totalBatches: 0,
                  message: event.payload.message,
                  startedAt: null,
                  lastUpdatedAt: new Date(),
                  estimatedTimeRemaining: null,
                  epochTimes: [],
                });
              }
              break;
            }
            case 'Custom': {
              const payload = event.payload;
              let customType: string | undefined;
              let customData: Record<string, unknown> = {};
              if (Array.isArray(payload)) {
                customType = payload[0] as string;
                if (payload[1] && typeof payload[1] === 'object') {
                  customData = payload[1] as Record<string, unknown>;
                }
              } else if (payload && typeof payload === 'object') {
                customType = (payload as Record<string, unknown>).type as string | undefined;
                customData = payload as Record<string, unknown>;
              }
              if (customType === 'SessionExperimentMapping') {
                const sid = customData.session_id;
                const eid = customData.experiment_id;
                const sidStr = typeof sid === 'string' ? sid : String(sid);
                const eidStr = typeof eid === 'string' ? eid : String(eid);
                if (sidStr && eidStr) {
                  sessionToExperiment.set(sidStr, eidStr);
                  const existing = next.get(sidStr);
                  if (existing) {
                    next.set(sidStr, { ...existing, experimentId: eidStr });
                  }
                }
              }
              break;
            }
            case 'SessionCompleted':
            case 'SessionFailed':
            case 'SessionCancelled': {
              const sid = event.payload.session_id;
              const existing = next.get(sid);
              if (existing) {
                next.set(sid, {
                  ...existing,
                  progress: event.type === 'SessionCompleted' ? 1 : existing.progress,
                  lastUpdatedAt: new Date(),
                  estimatedTimeRemaining: event.type === 'SessionCompleted' ? 0 : null,
                });
              }
              epochStartTimes.delete(sid);
              break;
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

    clear() {
      set(new Map());
    },
  };
}

export const progressStore = createProgressStore();

export function formatETA(seconds: number | null): string {
  if (seconds === null) return '计算中...';
  if (seconds <= 0) return '即将完成';
  if (seconds < 60) return `${Math.ceil(seconds)} 秒`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分 ${Math.ceil(seconds % 60)} 秒`;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return `${h} 时 ${m} 分`;
}

export function formatDuration(startTime: Date | null, endTime: Date | null): string {
  if (!startTime) return '-';
  const end = endTime || new Date();
  const seconds = (end.getTime() - startTime.getTime()) / 1000;
  return formatETA(seconds);
}
