import { invoke } from '@tauri-apps/api/core';
import type {
  LabStateSnapshot,
  DashboardStats,
  ResourceSnapshot,
  PluginInfo,
  HardwareInfo,
  TrainingRecommendation,
  DatasetInfo,
  DataPreview,
  DataLoadConfig,
  ModelArchDef,
} from './types';

export const labCommands = {
  getState: () =>
    invoke<LabStateSnapshot>('lab_get_state'),

  getDashboardStats: () =>
    invoke<DashboardStats>('lab_get_dashboard_stats'),

  getResourceSnapshot: () =>
    invoke<ResourceSnapshot>('lab_get_resource_snapshot'),

  listEngines: () =>
    invoke<PluginInfo[]>('lab_list_engines'),

  listTasks: () =>
    invoke<PluginInfo[]>('lab_list_tasks'),

  listModels: () =>
    invoke<PluginInfo[]>('lab_list_models'),

  listDataSources: () =>
    invoke<PluginInfo[]>('lab_list_data_sources'),

  getHardwareInfo: () =>
    invoke<HardwareInfo>('lab_get_hardware_info'),

  getRecommendations: (hardware: HardwareInfo, taskType: string, dataSize: number) =>
    invoke<TrainingRecommendation>('lab_get_recommendations', { hardware, taskType, dataSize }),

  loadData: (config: DataLoadConfig) =>
    invoke<DatasetInfo>('lab_load_data', { config }),

  previewData: (config: DataLoadConfig, offset?: number, limit?: number) =>
    invoke<DataPreview>('lab_preview_data', { config, offset: offset ?? null, limit: limit ?? null }),

  getModelArch: (modelId: string) =>
    invoke<ModelArchDef>('lab_get_model_arch', { modelId }),
};
