import { writable, derived } from 'svelte/store';
import { getLabClient } from './plugins';
import { onLabEvent } from '../adapter/events';
import { toastStore } from './toast';
import { taskManagerStore } from './taskManager';
import { translateError } from '../utils/errorMessages';
import type { DatasetInfo, DataPreview, DataLoadConfig, DatasetRegistration, DatasetSummary, ColumnProfile, LabEvent } from '../adapter/types';

function createDatasetStore() {
  const { subscribe, set, update } = writable<{
    info: DatasetInfo | null;
    preview: DataPreview | null;
  }>({
    info: null,
    preview: null,
  });

  return {
    subscribe,
    async load(config: DataLoadConfig) {
      try {
        const client = getLabClient();
        const info = await client.loadData(config);
        update((s) => ({ ...s, info }));
      } catch (e) {
        console.error('Failed to load dataset:', e);
      }
    },
    async preview(config: DataLoadConfig, offset?: number, limit?: number) {
      try {
        const client = getLabClient();
        const preview = await client.previewData(config, offset, limit);
        update((s) => ({ ...s, preview }));
      } catch (e) {
        console.error('Failed to preview dataset:', e);
      }
    },
    clear() {
      set({ info: null, preview: null });
    },
  };
}

export const datasetStore = createDatasetStore();

function createDatasetRegistryStore() {
  const { subscribe, set, update } = writable<{
    datasets: DatasetSummary[];
    loading: boolean;
    error: string | null;
    currentDataset: DatasetRegistration | null;
    currentProfiles: ColumnProfile[];
  }>({
    datasets: [],
    loading: false,
    error: null,
    currentDataset: null,
    currentProfiles: [],
  });

  let unlisten: (() => void) | null = null;

  const notifications = writable<Array<{ id: string; type: string; message: string; progress: number | null }>>([]);

  let notificationTaskMap = new Map<string, string>();

  function findNotificationByTask(taskId: string): string | null {
    return notificationTaskMap.get(taskId) || null;
  }

  function addNotification(type: string, message: string, progress: number | null = null, taskId?: string) {
    const id = Math.random().toString(36).slice(2);
    notifications.update((n) => [...n, { id, type, message, progress }]);
    if (taskId) {
      notificationTaskMap.set(taskId, id);
    }
    if (progress === null || progress >= 100) {
      setTimeout(() => {
        notifications.update((n) => n.filter((item) => item.id !== id));
        if (taskId) notificationTaskMap.delete(taskId);
      }, 5000);
    }
    return id;
  }

  function updateNotification(id: string, message: string, progress: number | null) {
    notifications.update((n) =>
      n.map((item) => (item.id === id ? { ...item, message, progress } : item))
    );
    if (progress !== null && progress >= 100) {
      setTimeout(() => {
        notifications.update((n) => n.filter((item) => item.id !== id));
      }, 3000);
    }
  }

  function handleDatasetEvent(event: LabEvent) {
    switch (event.type) {
      case 'DatasetRegistered': {
        const p = event.payload;
        toastStore.success(`数据集 "${p.name}" 注册成功 (${p.rows}行 x ${p.columns}列, ${p.format})`);
        break;
      }
      case 'DatasetDeleted': {
        const p = event.payload;
        update((s) => ({
          ...s,
          datasets: s.datasets.filter((d) => d.id !== p.dataset_id),
          currentDataset: s.currentDataset?.id === p.dataset_id ? null : s.currentDataset,
        }));
        toastStore.info('数据集已删除');
        break;
      }
      case 'DatasetArchived': {
        const p = event.payload;
        update((s) => ({
          ...s,
          datasets: s.datasets.map((d) =>
            d.id === p.dataset_id ? { ...d, status: 'archived' as const } : d
          ),
        }));
        toastStore.info('数据集已归档');
        break;
      }
      case 'DatasetRestored': {
        const p = event.payload;
        update((s) => ({
          ...s,
          datasets: s.datasets.map((d) =>
            d.id === p.dataset_id ? { ...d, status: 'active' as const } : d
          ),
        }));
        toastStore.success('数据集已恢复');
        break;
      }
      case 'DownloadProgress': {
        const p = event.payload;
        const pct = Math.round(p.progress * 100);
        const msg = `${p.message} (${pct}%, ${p.speed_mbps.toFixed(1)} MB/s)`;
        const existing = findNotificationByTask(p.task_id);
        if (existing) {
          updateNotification(existing, msg, pct);
        } else {
          addNotification('download', msg, pct, p.task_id);
        }
        break;
      }
      case 'DedupProgress': {
        const p = event.payload;
        const pct = Math.round(p.progress * 100);
        const msg = `${p.message} (${p.processed}/${p.total}, 发现${p.duplicates_found}个重复)`;
        const existing = findNotificationByTask(p.task_id);
        if (existing) {
          updateNotification(existing, msg, pct);
        } else {
          addNotification('dedup', msg, pct, p.task_id);
        }
        break;
      }
      case 'CurationProgress': {
        const p = event.payload;
        const pct = Math.round(p.progress * 100);
        const msg = `${p.step}: ${p.message} (${pct}%)`;
        const existing = findNotificationByTask(p.task_id);
        if (existing) {
          updateNotification(existing, msg, pct);
        } else {
          addNotification('curation', msg, pct, p.task_id);
        }
        break;
      }
      case 'OperationCompleted': {
        const p = event.payload;
        toastStore.success(`${p.operation} 完成`);
        break;
      }
      case 'OperationFailed': {
        const p = event.payload;
        toastStore.error(`${p.operation} 失败: ${p.error}`);
        break;
      }
    }
  }

  return {
    subscribe,
    notifications: { subscribe: notifications.subscribe },

    startListening() {
      if (unlisten) return;
      onLabEvent(handleDatasetEvent).then((fn) => { unlisten = fn; });
    },

    stopListening() {
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    },

    async fetchDatasets(statusFilter?: string, formatFilter?: string, nameContains?: string) {
      update((s) => ({ ...s, loading: true, error: null }));
      try {
        const client = getLabClient();
        const datasets = await client.listDatasets(statusFilter, formatFilter, nameContains);
        update((s) => ({ ...s, datasets, loading: false }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message, loading: false }));
        toastStore.error(advice.message + '\n' + advice.suggestion);
      }
    },

    async registerDataset(name: string, format: string, path: string) {
      try {
        const client = getLabClient();
        const dataset = await client.registerDataset(name, format, path);
        const summary: DatasetSummary = {
          id: dataset.id,
          name: dataset.name,
          version: dataset.version,
          status: dataset.status,
          format: dataset.format,
          rows: dataset.rows,
          columns: dataset.columns,
          has_missing_values: dataset.column_profiles.some((c) => c.null_count > 0),
          memory_size_mb: dataset.memory_size_mb,
          tags: dataset.tags,
          experiment_count: dataset.experiment_ids.length,
          created_at: dataset.created_at,
          updated_at: dataset.updated_at,
        };
        update((s) => ({
          ...s,
          datasets: [summary, ...s.datasets],
          currentDataset: dataset,
          currentProfiles: dataset.column_profiles || [],
        }));
        return dataset;
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
        throw e;
      }
    },

    async loadDataset(datasetId: string) {
      update((s) => ({ ...s, loading: true, error: null }));
      try {
        const client = getLabClient();
        const dataset = await client.getDataset(datasetId);
        update((s) => ({
          ...s,
          currentDataset: dataset,
          currentProfiles: dataset.column_profiles || [],
          loading: false,
        }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message, loading: false }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async deleteDataset(datasetId: string) {
      try {
        const client = getLabClient();
        await client.deleteDataset(datasetId);
        update((s) => ({
          ...s,
          datasets: s.datasets.filter((d) => d.id !== datasetId),
          currentDataset: s.currentDataset?.id === datasetId ? null : s.currentDataset,
        }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async archiveDataset(datasetId: string) {
      try {
        const client = getLabClient();
        await client.archiveDataset(datasetId);
        update((s) => ({
          ...s,
          datasets: s.datasets.map((d) =>
            d.id === datasetId ? { ...d, status: 'archived' as const } : d
          ),
        }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async restoreDataset(datasetId: string) {
      try {
        const client = getLabClient();
        await client.restoreDataset(datasetId);
        update((s) => ({
          ...s,
          datasets: s.datasets.map((d) =>
            d.id === datasetId ? { ...d, status: 'active' as const } : d
          ),
        }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async addTag(datasetId: string, tag: string) {
      try {
        const client = getLabClient();
        await client.datasetAddTag(datasetId, tag);
        update((s) => ({
          ...s,
          datasets: s.datasets.map((d) =>
            d.id === datasetId ? { ...d, tags: [...d.tags, tag] } : d
          ),
          currentDataset:
            s.currentDataset?.id === datasetId
              ? { ...s.currentDataset, tags: [...s.currentDataset.tags, tag] }
              : s.currentDataset,
        }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async removeTag(datasetId: string, tag: string) {
      try {
        const client = getLabClient();
        await client.datasetRemoveTag(datasetId, tag);
        update((s) => ({
          ...s,
          datasets: s.datasets.map((d) =>
            d.id === datasetId ? { ...d, tags: d.tags.filter((t) => t !== tag) } : d
          ),
          currentDataset:
            s.currentDataset?.id === datasetId
              ? { ...s.currentDataset, tags: s.currentDataset.tags.filter((t) => t !== tag) }
              : s.currentDataset,
        }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async setDescription(datasetId: string, description: string) {
      try {
        const client = getLabClient();
        await client.datasetSetDescription(datasetId, description);
        update((s) => ({
          ...s,
          currentDataset:
            s.currentDataset?.id === datasetId
              ? { ...s.currentDataset, description }
              : s.currentDataset,
        }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async linkExperiment(datasetId: string, experimentId: string) {
      try {
        const client = getLabClient();
        await client.datasetLinkExperiment(datasetId, experimentId);
        update((s) => ({
          ...s,
          currentDataset:
            s.currentDataset?.id === datasetId
              ? { ...s.currentDataset, experiment_ids: [...s.currentDataset.experiment_ids, experimentId] }
              : s.currentDataset,
        }));
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
      }
    },

    async newVersion(datasetId: string) {
      try {
        const client = getLabClient();
        const dataset = await client.datasetNewVersion(datasetId);
        update((s) => ({
          ...s,
          currentDataset: dataset,
          currentProfiles: dataset.column_profiles || [],
          datasets: s.datasets.map((d) =>
            d.id === datasetId ? { ...d, version: dataset.version, rows: dataset.rows, columns: dataset.columns } : d
          ),
        }));
        return dataset;
      } catch (e: any) {
        const advice = translateError(e.message || '');
        update((s) => ({ ...s, error: advice.message }));
        toastStore.error(advice.message + '\n💡 ' + advice.suggestion);
        throw e;
      }
    },

    clearCurrent() {
      update((s) => ({ ...s, currentDataset: null, currentProfiles: [] }));
    },

    clearError() {
      update((s) => ({ ...s, error: null }));
    },
  };
}

export const datasetRegistryStore = createDatasetRegistryStore();

export const activeDatasets = derived(datasetRegistryStore, ($s) =>
  $s.datasets.filter((d) => d.status === 'active')
);

export const archivedDatasets = derived(datasetRegistryStore, ($s) =>
  $s.datasets.filter((d) => d.status === 'archived')
);

export const numericColumns = derived(datasetRegistryStore, ($s) =>
  ($s.currentProfiles || []).filter(
    (c) => c.column_type === 'integer' || c.column_type === 'float'
  )
);

export const categoricalColumns = derived(datasetRegistryStore, ($s) =>
  ($s.currentProfiles || []).filter(
    (c) => c.column_type === 'categorical' || c.column_type === 'boolean'
  )
);

export const columnsWithMissing = derived(datasetRegistryStore, ($s) =>
  ($s.currentProfiles || []).filter((c) => c.null_count > 0)
);
