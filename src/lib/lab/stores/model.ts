import { writable } from 'svelte/store';
import { getLabClient } from './plugins';
import type { ModelRegistration, ModelRegistrationStatus } from '../adapter/types';

function createModelStore() {
  const { subscribe, set, update } = writable<ModelRegistration[]>([]);
  const { subscribe: loadingSub, set: setLoading } = writable(false);
  const { subscribe: selectedSub, set: setSelected } = writable<ModelRegistration | null>(null);

  return {
    subscribe,
    loading: { subscribe: loadingSub },
    selected: { subscribe: selectedSub },

    async refresh(statusFilter?: string) {
      setLoading(true);
      try {
        const client = getLabClient();
        const models = await client.listModelRegistrations(statusFilter);
        set(models);
      } catch (e) {
        console.error('Failed to load model registrations:', e);
      } finally {
        setLoading(false);
      }
    },

    async loadDetail(modelId: string) {
      try {
        const client = getLabClient();
        const model = await client.getModelRegistration(modelId);
        setSelected(model);
        update((models) => {
          const idx = models.findIndex((m) => m.id === modelId);
          if (idx >= 0) {
            const next = [...models];
            next[idx] = model;
            return next;
          }
          return [...models, model];
        });
        return model;
      } catch (e) {
        console.error('Failed to load model detail:', e);
        return null;
      }
    },

    async register(name: string, version: string, framework: string) {
      try {
        const client = getLabClient();
        await client.registerModel(name, version, framework);
        await this.refresh();
      } catch (e) {
        console.error('Failed to register model:', e);
        throw e;
      }
    },

    async promoteStaging(modelId: string) {
      try {
        const client = getLabClient();
        await client.promoteModelStaging(modelId);
        await this.refresh();
      } catch (e) {
        console.error('Failed to promote model to staging:', e);
        throw e;
      }
    },

    async promoteProduction(modelId: string) {
      try {
        const client = getLabClient();
        await client.promoteModelProduction(modelId);
        await this.refresh();
      } catch (e) {
        console.error('Failed to promote model to production:', e);
        throw e;
      }
    },

    async archive(modelId: string) {
      try {
        const client = getLabClient();
        await client.archiveModel(modelId);
        await this.refresh();
      } catch (e) {
        console.error('Failed to archive model:', e);
        throw e;
      }
    },

    async delete(modelId: string) {
      try {
        const client = getLabClient();
        await client.deleteModelRegistration(modelId);
        update((models) => models.filter((m) => m.id !== modelId));
      } catch (e) {
        console.error('Failed to delete model:', e);
        throw e;
      }
    },

    async setPath(modelId: string, path: string) {
      try {
        const client = getLabClient();
        await client.setModelPath(modelId, path);
        await this.refresh();
      } catch (e) {
        console.error('Failed to set model path:', e);
        throw e;
      }
    },

    async registerFromExperiment(experimentId: string, name: string, version: string) {
      try {
        const client = getLabClient();
        await client.registerModelFromExperiment(experimentId, name, version);
        await this.refresh();
      } catch (e) {
        console.error('Failed to register model from experiment:', e);
        throw e;
      }
    },
  };
}

export const modelStore = createModelStore();
