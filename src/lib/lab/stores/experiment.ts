import { writable } from 'svelte/store';
import { get } from 'svelte/store';
import { getLabClient } from './plugins';
import { toastStore } from './toast';
import { notificationStore } from './notification';
import { translateError } from '../utils/errorMessages';
import { i18n } from '$lib/i18n';
import type { ExperimentSummary, ExperimentDetail, MetricsTimeline, TrainingConfig, LabEvent } from '../adapter/types';

function createExperimentStore() {
  const { subscribe, set, update } = writable<Map<string, ExperimentSummary>>(new Map());
  const { subscribe: detailSub, set: setDetail, update: updateDetail } = writable<Map<string, ExperimentDetail>>(new Map());
  const { subscribe: metricsSub, set: setMetrics, update: updateMetrics } = writable<Map<string, MetricsTimeline>>(new Map());
  const { subscribe: loadingSub, set: setLoading } = writable(false);
  const { subscribe: logsSub, update: updateLogs } = writable<Map<string, Array<{ level: string; message: string; timestamp: number }>>>(new Map());

  let unlisten: (() => void) | null = null;
  let sessionToExperiment: Map<string, string> = new Map();
  let refreshTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingRefresh = false;

  function debouncedRefresh(delay = 300) {
    if (refreshTimer) clearTimeout(refreshTimer);
    pendingRefresh = true;
    refreshTimer = setTimeout(async () => {
      pendingRefresh = false;
      await doRefresh();
    }, delay);
  }

  async function doRefresh() {
    setLoading(true);
    try {
      const client = getLabClient();
      const experiments = await client.listExperiments();
      const map = new Map<string, ExperimentSummary>();
      for (const exp of experiments) {
        map.set(exp.id, exp);
      }
      set(map);
    } catch (e) {
      console.error('Failed to load experiments:', e);
    } finally {
      setLoading(false);
    }
  }

  let metricsDebounceTimers: Map<string, ReturnType<typeof setTimeout>> = new Map();

  function debouncedLoadMetrics(experimentId: string, metricNames: string[], delay = 500) {
    const key = experimentId;
    const existing = metricsDebounceTimers.get(key);
    if (existing) clearTimeout(existing);
    metricsDebounceTimers.set(key, setTimeout(async () => {
      metricsDebounceTimers.delete(key);
      try {
        const client = getLabClient();
        const timeline = await client.queryMetrics(experimentId, metricNames);
        updateMetrics((map) => {
          const next = new Map(map);
          next.set(experimentId, timeline);
          return next;
        });
      } catch (e) {
        console.error('Failed to load metrics:', e);
      }
    }, delay));
  }

  return {
    subscribe,
    details: { subscribe: detailSub },
    metrics: { subscribe: metricsSub },
    logs: { subscribe: logsSub },
    loading: { subscribe: loadingSub },

    async refresh() {
      if (pendingRefresh) return;
      await doRefresh();
    },

    async loadDetail(experimentId: string) {
      try {
        const client = getLabClient();
        const detail = await client.getExperimentDetail(experimentId);
        updateDetail((map) => {
          const next = new Map(map);
          next.set(experimentId, detail);
          return next;
        });
        return detail;
      } catch (e) {
        console.error('Failed to load experiment detail:', e);
        return null;
      }
    },

    async loadMetrics(experimentId: string, metricNames: string[]) {
      try {
        const client = getLabClient();
        const timeline = await client.queryMetrics(experimentId, metricNames);
        updateMetrics((map) => {
          const next = new Map(map);
          next.set(experimentId, timeline);
          return next;
        });
        return timeline;
      } catch (e) {
        console.error('Failed to load metrics:', e);
        return null;
      }
    },

    async loadPersistedLogs(experimentId: string, limit?: number) {
      try {
        const client = getLabClient();
        const logs = await client.loadLogs(experimentId, limit);
        if (logs && logs.length > 0) {
          updateLogs((map) => {
            const next = new Map(map);
            const existing = next.get(experimentId) || [];
            const existingKeys = new Set(existing.map((l: { message: string; timestamp: number }) => `${l.timestamp}:${l.message}`));
            const merged = [...existing];
            for (const log of logs) {
              const key = `${new Date(log.timestamp).getTime()}:${log.message}`;
              if (!existingKeys.has(key)) {
                merged.push({
                  level: log.level,
                  message: log.message,
                  timestamp: new Date(log.timestamp).getTime(),
                });
              }
            }
            merged.sort((a: { timestamp: number }, b: { timestamp: number }) => a.timestamp - b.timestamp);
            if (merged.length > 1000) {
              merged.splice(0, merged.length - 800);
            }
            next.set(experimentId, merged);
            return next;
          });
        }
      } catch (e) {
        console.error('Failed to load persisted logs:', e);
      }
    },

    addExperiment(experiment: ExperimentSummary) {
      update((experiments) => {
        const next = new Map(experiments);
        next.set(experiment.id, experiment);
        return next;
      });
    },

    async startTraining(name: string, taskType: string, config: TrainingConfig): Promise<string | null> {
      try {
        const client = getLabClient();
        const experimentId = await client.startTraining(name, taskType, config);
        await this.refresh();
        toastStore.success(get(i18n.t)('experiment.trainingStarted', { name }));
        return experimentId;
      } catch (e: any) {
        console.error('Failed to start training:', e);
        const advice = translateError(e.message || '');
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
        return null;
      }
    },

    async stopTraining(experimentId: string) {
      try {
        const client = getLabClient();
        await client.stopTraining(experimentId);
        await this.refresh();
        toastStore.warning(get(i18n.t)('experiment.trainingStopped'));
      } catch (e: any) {
        console.error('Failed to stop training:', e);
        const advice = translateError(e.message || '');
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async pauseTraining(experimentId: string) {
      try {
        const client = getLabClient();
        await client.pauseTraining(experimentId);
        await this.refresh();
        toastStore.info(get(i18n.t)('experiment.trainingPaused'));
      } catch (e: any) {
        console.error('Failed to pause training:', e);
        const advice = translateError(e.message || '');
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async resumeTraining(experimentId: string) {
      try {
        const client = getLabClient();
        await client.resumeTraining(experimentId);
        await this.refresh();
        toastStore.info(get(i18n.t)('experiment.trainingResumed'));
      } catch (e: any) {
        console.error('Failed to resume training:', e);
        const advice = translateError(e.message || '');
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    startListening() {
      if (unlisten) return;
      const client = getLabClient();
      client.onLabEvent((event: LabEvent) => {
        switch (event.type) {
          case 'SessionStarted':
            debouncedRefresh();
            break;
          case 'SessionCompleted': {
            debouncedRefresh();
            const sid = event.payload.session_id;
            const expId = sid ? sessionToExperiment.get(sid) : undefined;
            if (sid) sessionToExperiment.delete(sid);
            notificationStore.notify('training_completed', get(i18n.t)('experiment.trainingCompleted'), get(i18n.t)('experiment.trainingCompletedMsg'), expId);
            break;
          }
          case 'SessionFailed': {
            debouncedRefresh();
            const errMsg = (event.payload as { error?: string })?.error || get(i18n.t)('experiment.unknownError');
            const sid2 = event.payload.session_id;
            const expId2 = sid2 ? sessionToExperiment.get(sid2) : undefined;
            if (sid2) sessionToExperiment.delete(sid2);
            notificationStore.notify('training_failed', get(i18n.t)('experiment.trainingFailed'), errMsg, expId2);
            break;
          }
          case 'SessionCancelled': {
            debouncedRefresh();
            const sid3 = event.payload.session_id;
            const expId3 = sid3 ? sessionToExperiment.get(sid3) : undefined;
            if (sid3) sessionToExperiment.delete(sid3);
            notificationStore.notify('training_cancelled', get(i18n.t)('experiment.trainingCancelled'), get(i18n.t)('experiment.trainingCancelledMsg'), expId3);
            break;
          }
          case 'SessionPaused':
            debouncedRefresh();
            break;
          case 'SessionResumed':
            debouncedRefresh();
            break;
          case 'EpochCompleted': {
            const sid = event.payload.session_id;
            const expId = sessionToExperiment.get(sid);
            if (expId) {
              this.loadDetail(expId);
              debouncedLoadMetrics(expId, ['train_loss', 'val_loss', 'accuracy']);
            }
            break;
          }
          case 'BatchCompleted': {
            const sid = event.payload.session_id;
            const expId = sessionToExperiment.get(sid);
            if (expId) {
              updateLogs(map => {
                const logs = map.get(expId) || [];
                if (logs.length > 500) {
                  logs.splice(0, logs.length - 400);
                }
                logs.push({
                  level: 'debug',
                  message: `Batch ${event.payload.batch}/${event.payload.total_batches} loss=${event.payload.loss.toFixed(4)}`,
                  timestamp: Date.now()
                });
                map.set(expId, logs);
                return map;
              });
            }
            break;
          }
          case 'LogOutput': {
            const sid = event.payload.session_id;
            const expId = sessionToExperiment.get(sid);
            if (expId) {
              updateLogs(map => {
                const logs = map.get(expId) || [];
                if (logs.length > 500) {
                  logs.splice(0, logs.length - 400);
                }
                logs.push({
                  level: event.payload.level,
                  message: event.payload.message,
                  timestamp: Date.now()
                });
                map.set(expId, logs);
                return map;
              });
            }
            break;
          }
          case 'CheckpointSaved': {
            const sid = event.payload.session_id;
            const expId = sessionToExperiment.get(sid);
            if (expId) {
              this.loadDetail(expId);
            }
            break;
          }
          case 'Custom': {
            const payload = event.payload as [string, unknown] | Record<string, unknown>;
            let customType: string | undefined;
            let customData: Record<string, unknown> = {};

            if (Array.isArray(payload)) {
              customType = payload[0] as string;
              const second = payload[1];
              if (second && typeof second === 'object') {
                const obj = second as Record<string, unknown>;
                if (obj.payload && typeof obj.payload === 'object') {
                  customData = (obj as { payload: Record<string, unknown> }).payload;
                } else {
                  customData = obj;
                }
              }
            } else if (payload && typeof payload === 'object') {
              customType = (payload as Record<string, unknown>).type as string | undefined;
              customData = payload;
            }

            if (customType === 'SessionExperimentMapping') {
              const sid = customData.session_id;
              const eid = customData.experiment_id;
              if (sid && eid) {
                const sidStr = typeof sid === 'string' ? sid : (sid as { id: string }).id;
                const eidStr = typeof eid === 'string' ? eid : (eid as { id: string }).id;
                sessionToExperiment.set(sidStr, eidStr);
              }
            }
            if (customType === 'TrainingStarted' || customType === 'TrainingStopped' || customType === 'TrainingPaused' || customType === 'TrainingResumed' || customType === 'ExperimentStarted' || customType === 'ExperimentCompleted' || customType === 'ExperimentFailed' || customType === 'ExperimentCancelled' || customType === 'ExperimentCreated') {
              debouncedRefresh();
            }
            if (customType === 'EpochCompleted' && customData.experiment_id) {
              const expId = customData.experiment_id as { id?: string } | string;
              const expIdStr = typeof expId === 'string' ? expId : expId?.id;
              if (expIdStr) {
                this.loadDetail(expIdStr);
                debouncedLoadMetrics(expIdStr, ['train_loss', 'val_loss']);
              }
            }
            if (customType === 'MetricTracked' && customData.experiment_id) {
              const expId = (customData.experiment_id as { id?: string } | string);
              const expIdStr = typeof expId === 'string' ? expId : expId?.id;
              if (expIdStr) {
                debouncedLoadMetrics(expIdStr, ['train_loss', 'val_loss', 'accuracy'], 1000);
              }
            }
            break;
          }
          default:
            break;
        }
      }).then((fn) => { unlisten = fn; });
    },

    stopListening() {
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
      if (refreshTimer) {
        clearTimeout(refreshTimer);
        refreshTimer = null;
      }
      for (const timer of metricsDebounceTimers.values()) {
        clearTimeout(timer);
      }
      metricsDebounceTimers.clear();
      sessionToExperiment.clear();
    },
  };
}

export const experimentStore = createExperimentStore();
