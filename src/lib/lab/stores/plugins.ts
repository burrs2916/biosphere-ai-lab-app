import { writable } from 'svelte/store';
import type { LabClient } from '../adapter/client';
import type { PluginInfo, PluginKind } from '../adapter/types';
import { TauriClient } from '../adapter/tauri_client';
import { MockClient } from '../adapter/mock_client';

function createClient(): LabClient {
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    return new TauriClient();
  }
  return new MockClient();
}

let clientInstance: LabClient | null = null;

export function getLabClient(): LabClient {
  if (!clientInstance) {
    clientInstance = createClient();
  }
  return clientInstance;
}

export function setLabClient(client: LabClient): void {
  clientInstance = client;
}

function createPluginStore() {
  const { subscribe, set } = writable<Record<PluginKind, PluginInfo[]>>({
    engine: [],
    task: [],
    model: [],
    data_source: [],
    remote: [],
  });
  const { subscribe: loadingSub, set: setLoading } = writable(false);

  return {
    subscribe,
    loading: { subscribe: loadingSub },
    async refresh() {
      setLoading(true);
      try {
        const client = getLabClient();
        const [engines, tasks, models, dataSources] = await Promise.all([
          client.listEngines(),
          client.listTasks(),
          client.listModels(),
          client.listDataSources(),
        ]);
        set({
          engine: engines,
          task: tasks,
          model: models,
          data_source: dataSources,
          remote: [],
        });
      } catch (e) {
        console.error('Failed to load plugins:', e);
      } finally {
        setLoading(false);
      }
    },
  };
}

export const pluginStore = createPluginStore();
