import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import type { LabClient } from './client';
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
  LabEvent,
  SessionId,
  ExperimentSummary,
  ExperimentDetail,
  MetricsTimeline,
  TrainingConfig,
  FileFilter,
  ModelRegistration,
  AppSettings,
  InferenceResult,
  PipelineStep,
  ArtifactRef,
  ModelVersion,
  CheckpointInfo,
  EvaluationResult,
  DatasetRegistration,
  DatasetSummary,
  TuneConfig,
  TuneResult,
  HparamSpace,
  TuneStrategy,
  HparamValue,
  ExportFormat,
  ExportResult,
  DataExportFormat,
  DataExportResult,
  ServeResponse,
  ServeEndpoint,
  ServeStats,
  DatasetVersionRecord,
  ConnectorInfo,
  DiscoveredItem,
  ResolvedDataSource,
  LogEntry,
} from './types';

export class TauriClient implements LabClient {
  async getState(): Promise<LabStateSnapshot> {
    return invoke<LabStateSnapshot>('lab_get_state');
  }

  async getDashboardStats(): Promise<DashboardStats> {
    return invoke<DashboardStats>('lab_get_dashboard_stats');
  }

  async getResourceSnapshot(): Promise<ResourceSnapshot> {
    return invoke<ResourceSnapshot>('lab_get_resource_snapshot');
  }

  async listEngines(): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('lab_list_engines');
  }

  async listTasks(): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('lab_list_tasks');
  }

  async listModels(): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('lab_list_models');
  }

  async listDataSources(): Promise<PluginInfo[]> {
    return invoke<PluginInfo[]>('lab_list_data_sources');
  }

  async getHardwareInfo(): Promise<HardwareInfo> {
    return invoke<HardwareInfo>('lab_get_hardware_info');
  }

  async getRecommendations(hardware: HardwareInfo, taskType: string, dataSize: number): Promise<TrainingRecommendation> {
    return invoke<TrainingRecommendation>('lab_get_recommendations', { hardware, taskType, dataSize });
  }

  async loadData(config: DataLoadConfig): Promise<DatasetInfo> {
    return invoke<DatasetInfo>('lab_load_data', { config });
  }

  async previewData(config: DataLoadConfig, offset?: number, limit?: number): Promise<DataPreview> {
    return invoke<DataPreview>('lab_preview_data', { config, offset: offset ?? null, limit: limit ?? null });
  }

  async getModelArch(modelId: string): Promise<ModelArchDef> {
    return invoke<ModelArchDef>('lab_get_model_arch', { modelId });
  }

  async createExperiment(name: string, taskType: string, config: TrainingConfig): Promise<string> {
    return invoke<string>('lab_create_experiment', { name, taskType, config });
  }

  async listExperiments(group?: string): Promise<ExperimentSummary[]> {
    return invoke<ExperimentSummary[]>('lab_list_experiments', { group: group || null });
  }

  async getExperimentDetail(experimentId: string): Promise<ExperimentDetail> {
    return invoke<ExperimentDetail>('lab_get_experiment_detail', { experimentId });
  }

  async queryMetrics(experimentId: string, metricNames: string[]): Promise<MetricsTimeline> {
    return invoke<MetricsTimeline>('lab_query_metrics', { experimentId, metricNames });
  }

  async queryMetricsDownsampled(experimentId: string, metricNames: string[], maxPoints?: number, smoothAlpha?: number): Promise<MetricsTimeline> {
    return invoke<MetricsTimeline>('lab_query_metrics_downsampled', {
      experimentId,
      metricNames,
      maxPoints: maxPoints ?? null,
      smoothAlpha: smoothAlpha ?? null,
    });
  }

  async loadLogs(experimentId: string, limit?: number): Promise<LogEntry[]> {
    return invoke<LogEntry[]>('lab_load_logs', { experimentId, limit: limit ?? null });
  }

  async trackMetric(experimentId: string, metricName: string, value: number, step: number): Promise<void> {
    return invoke<void>('lab_track_metric', { experimentId, metricName, value, step });
  }

  async registerModel(name: string, version: string, framework: string): Promise<string> {
    return invoke<string>('lab_register_model', { name, version, framework });
  }

  async listModelRegistrations(statusFilter?: string): Promise<ModelRegistration[]> {
    return invoke<ModelRegistration[]>('lab_list_model_registrations', { statusFilter: statusFilter || null });
  }

  async getModelRegistration(modelId: string): Promise<ModelRegistration> {
    return invoke<ModelRegistration>('lab_get_model_registration', { modelId });
  }

  async promoteModelStaging(modelId: string): Promise<void> {
    return invoke<void>('lab_promote_model_staging', { modelId });
  }

  async promoteModelProduction(modelId: string): Promise<void> {
    return invoke<void>('lab_promote_model_production', { modelId });
  }

  async demoteModelStaging(modelId: string): Promise<void> {
    return invoke<void>('lab_demote_model_staging', { modelId });
  }

  async archiveModel(modelId: string): Promise<void> {
    return invoke<void>('lab_archive_model', { modelId });
  }

  async addModelAlias(modelId: string, alias: string): Promise<void> {
    return invoke<void>('lab_add_model_alias', { modelId, alias });
  }

  async removeModelAlias(modelId: string, alias: string): Promise<void> {
    return invoke<void>('lab_remove_model_alias', { modelId, alias });
  }

  async deleteModelRegistration(modelId: string): Promise<void> {
    return invoke<void>('lab_delete_model_registration', { modelId });
  }

  async setModelPath(modelId: string, path: string): Promise<void> {
    return invoke<void>('lab_set_model_path', { modelId, path });
  }

  async registerModelFromExperiment(experimentId: string, name: string, version: string): Promise<void> {
    return invoke<void>('lab_register_model_from_experiment', { experimentId, name, version });
  }

  async startTraining(name: string, taskType: string, config: TrainingConfig): Promise<string> {
    return invoke<string>('lab_start_training', { name, taskType, configValue: config });
  }

  async stopTraining(experimentId: string): Promise<void> {
    return invoke<void>('lab_stop_training', { experimentId });
  }

  async pauseTraining(experimentId: string): Promise<void> {
    return invoke<void>('lab_pause_training', { experimentId });
  }

  async resumeTraining(experimentId: string): Promise<void> {
    return invoke<void>('lab_resume_training', { experimentId });
  }

  async resumeFromCheckpoint(experimentId: string, checkpointEpoch: number): Promise<void> {
    return invoke<void>('lab_resume_from_checkpoint', { experimentId, checkpointEpoch });
  }

  async runInference(experimentId: string, input_data: number[][]): Promise<InferenceResult> {
    return invoke<InferenceResult>('lab_run_inference', { experimentId, inputData: input_data });
  }

  async preprocessData(dataPath: string, dataFormat: string, steps: PipelineStep[]): Promise<DataPreview> {
    return invoke<DataPreview>('lab_preprocess_data', { dataPath, dataFormat, steps });
  }

  async experimentAddTag(experimentId: string, tag: string): Promise<void> {
    return invoke('lab_experiment_add_tag', { experimentId, tag });
  }

  async experimentSetParam(experimentId: string, key: string, value: unknown): Promise<void> {
    return invoke('lab_experiment_set_param', { experimentId, key, value });
  }

  async listArtifacts(experimentId: string): Promise<ArtifactRef[]> {
    return invoke<ArtifactRef[]>('lab_list_artifacts', { experimentId });
  }

  async getArtifactContent(experimentId: string, artifactPath: string): Promise<number[]> {
    return invoke<number[]>('lab_get_artifact_content', { experimentId, artifactPath });
  }

  async scanArtifacts(experimentId: string): Promise<ArtifactRef[]> {
    return invoke<ArtifactRef[]>('lab_scan_artifacts', { experimentId });
  }

  async openArtifactDir(experimentId: string): Promise<void> {
    const { revealItemInDir } = await import('@tauri-apps/plugin-opener');
    const detail = await this.getExperimentDetail(experimentId);
    if (detail.artifacts.length > 0) {
      await revealItemInDir(detail.artifacts[0].path);
    }
  }

  async addModelVersion(modelId: string, path: string, description?: string): Promise<void> {
    return invoke<void>('lab_add_model_version', { modelId, path, description });
  }

  async listModelVersions(modelId: string): Promise<ModelVersion[]> {
    return invoke<ModelVersion[]>('lab_list_model_versions', { modelId });
  }

  async setModelDescription(modelId: string, description: string): Promise<void> {
    return invoke<void>('lab_set_model_description', { modelId, description });
  }

  async addModelTag(modelId: string, tag: string): Promise<void> {
    return invoke<void>('lab_add_model_tag', { modelId, tag });
  }

  async removeModelTag(modelId: string, tag: string): Promise<void> {
    return invoke<void>('lab_remove_model_tag', { modelId, tag });
  }

  async experimentSetDescription(experimentId: string, description: string): Promise<void> {
    return invoke<void>('lab_experiment_set_description', { experimentId, description });
  }

  async listCheckpoints(experimentId: string): Promise<CheckpointInfo[]> {
    return invoke<CheckpointInfo[]>('lab_list_checkpoints', { experimentId });
  }

  async deleteCheckpoint(experimentId: string, checkpointName: string): Promise<void> {
    return invoke<void>('lab_delete_checkpoint', { experimentId, checkpointName });
  }

  async evaluateModel(experimentId: string, testDataPath: string): Promise<EvaluationResult> {
    return invoke<EvaluationResult>('lab_evaluate_model', { experimentId, testDataPath });
  }

  async saveEvaluation(experimentId: string, evaluationResult: any, testDataPath: string): Promise<string> {
    return invoke<string>('lab_save_evaluation', { experimentId, evaluationResult, testDataPath });
  }

  async listEvaluations(experimentId: string): Promise<any[]> {
    return invoke<any[]>('lab_list_evaluations', { experimentId });
  }

  async deleteExperiment(experimentId: string): Promise<void> {
    return invoke<void>('lab_delete_experiment', { experimentId });
  }

  async archiveExperiment(experimentId: string): Promise<void> {
    return invoke<void>('lab_archive_experiment', { experimentId });
  }

  async restoreExperiment(experimentId: string): Promise<void> {
    return invoke<void>('lab_restore_experiment', { experimentId });
  }

  async batchDeleteExperiments(experimentIds: string[]): Promise<number> {
    return invoke<number>('lab_batch_delete_experiments', { experimentIds });
  }

  async cloneExperiment(experimentId: string, newName: string): Promise<string> {
    return invoke<string>('lab_clone_experiment', { experimentId, newName });
  }

  async setExperimentGroup(experimentId: string, group: string): Promise<void> {
    return invoke<void>('lab_set_experiment_group', { experimentId, group });
  }

  async listExperimentGroups(): Promise<string[]> {
    return invoke<string[]>('lab_list_experiment_groups');
  }

  async registerDataset(name: string, format: string, path: string): Promise<DatasetRegistration> {
    return invoke<DatasetRegistration>('lab_register_dataset', { name, format, path });
  }

  async listDatasets(statusFilter?: string, formatFilter?: string, nameContains?: string): Promise<DatasetSummary[]> {
    return invoke<DatasetSummary[]>('lab_list_datasets', { statusFilter, formatFilter, nameContains });
  }

  async getDataset(datasetId: string): Promise<DatasetRegistration> {
    return invoke<DatasetRegistration>('lab_get_dataset', { datasetId });
  }

  async deleteDataset(datasetId: string): Promise<void> {
    return invoke<void>('lab_delete_dataset', { datasetId });
  }

  async archiveDataset(datasetId: string): Promise<void> {
    return invoke<void>('lab_archive_dataset', { datasetId });
  }

  async restoreDataset(datasetId: string): Promise<void> {
    return invoke<void>('lab_restore_dataset', { datasetId });
  }

  async datasetAddTag(datasetId: string, tag: string): Promise<void> {
    return invoke<void>('lab_dataset_add_tag', { datasetId, tag });
  }

  async datasetRemoveTag(datasetId: string, tag: string): Promise<void> {
    return invoke<void>('lab_dataset_remove_tag', { datasetId, tag });
  }

  async datasetSetDescription(datasetId: string, description: string): Promise<void> {
    return invoke<void>('lab_dataset_set_description', { datasetId, description });
  }

  async datasetLinkExperiment(datasetId: string, experimentId: string): Promise<void> {
    return invoke<void>('lab_dataset_link_experiment', { datasetId, experimentId });
  }

  async datasetNewVersion(datasetId: string): Promise<DatasetRegistration> {
    return invoke<DatasetRegistration>('lab_dataset_new_version', { datasetId });
  }

  async datasetVersionHistory(datasetId: string): Promise<DatasetVersionRecord[]> {
    return invoke<DatasetVersionRecord[]>('lab_dataset_version_history', { datasetId });
  }

  async datasetNewVersionWithNote(datasetId: string, changeNote: string): Promise<DatasetRegistration> {
    return invoke<DatasetRegistration>('lab_dataset_new_version_with_note', { datasetId, changeNote });
  }

  async listConnectors(): Promise<ConnectorInfo[]> {
    return invoke<ConnectorInfo[]>('lab_list_connectors');
  }

  async scanDataSources(uri: string, options?: { recursive?: boolean; max_depth?: number; extensions?: string[]; max_results?: number }): Promise<DiscoveredItem[]> {
    return invoke<DiscoveredItem[]>('lab_scan_data_sources', {
      uri,
      recursive: options?.recursive,
      maxDepth: options?.max_depth,
      extensions: options?.extensions,
      maxResults: options?.max_results,
    });
  }

  async testDataConnection(uri: string): Promise<boolean> {
    return invoke<boolean>('lab_test_data_connection', { uri });
  }

  async resolveDataItem(item: DiscoveredItem): Promise<ResolvedDataSource> {
    return invoke<ResolvedDataSource>('lab_resolve_data_item', { item });
  }

  async quickRegisterDataset(item: DiscoveredItem): Promise<DatasetRegistration> {
    return invoke<DatasetRegistration>('lab_quick_register_dataset', { item });
  }

  async batchInference(experimentId: string, inputData: number[][]): Promise<InferenceResult> {
    return invoke<InferenceResult>('lab_batch_inference', { experimentId, inputData });
  }

  async selectFile(filters?: FileFilter[]): Promise<string | null> {
    const result = await open({
      multiple: false,
      filters: filters?.map(f => ({ name: f.name, extensions: f.extensions })),
    });
    if (result === null) return null;
    if (Array.isArray(result)) return result[0] || null;
    return result;
  }

  async selectDirectory(): Promise<string | null> {
    const result = await open({
      directory: true,
      multiple: false,
    });
    if (result === null) return null;
    if (Array.isArray(result)) return result[0] || null;
    return result;
  }

  async getSettings(): Promise<AppSettings> {
    return invoke<AppSettings>('lab_get_settings');
  }

  async saveSettings(settings: AppSettings): Promise<void> {
    return invoke<void>('lab_save_settings', { settings });
  }

  async onLabEvent(handler: (event: LabEvent) => void): Promise<() => void> {
    return listen<LabEvent>('lab-event', (e) => {
      handler(e.payload);
    });
  }

  async onSessionEvent(sessionId: SessionId, handler: (event: LabEvent) => void): Promise<() => void> {
    return listen<LabEvent>('lab-event', (e) => {
      const event = e.payload;
      if ('session_id' in (event.payload as Record<string, unknown>)) {
        const payload = event.payload as { session_id: SessionId };
        if (payload.session_id === sessionId) {
          handler(event);
        }
      }
    });
  }

  async startHyperparameterTuning(tuneConfig: TuneConfig): Promise<TuneResult> {
    return invoke<TuneResult>('lab_start_hyperparameter_tuning', {
      tuneConfigJson: JSON.stringify(tuneConfig),
    });
  }

  async generateHparamCombinations(hparamSpace: HparamSpace, strategy: TuneStrategy): Promise<Record<string, HparamValue>[]> {
    return invoke<Record<string, HparamValue>[]>('lab_generate_hparam_combinations', {
      hparamSpaceJson: JSON.stringify(hparamSpace),
      strategyJson: JSON.stringify(strategy),
    });
  }

  async exportModel(experimentId: string, format: string, outputPath: string | null, opsetVersion: number | null, inputShapes: number[][]): Promise<ExportResult> {
    return invoke<ExportResult>('lab_export_model', {
      experimentId,
      format,
      outputPath,
      opsetVersion,
      inputShapes,
    });
  }

  async exportDataset(datasetId: string, targetFormat: DataExportFormat, outputPath: string | null): Promise<DataExportResult> {
    return invoke<DataExportResult>('lab_export_dataset', {
      datasetId,
      targetFormat,
      outputPath,
    });
  }

  async listExportFormats(engineId: string): Promise<string[]> {
    return invoke<string[]>('lab_list_export_formats', { engineId });
  }

  async modelDeploy(modelId: string): Promise<void> {
    return invoke<void>('lab_model_deploy', { modelId });
  }

  async modelUndeploy(modelId: string): Promise<void> {
    return invoke<void>('lab_model_undeploy', { modelId });
  }

  async modelPredict(modelId: string, inputs: number[][]): Promise<ServeResponse> {
    return invoke<ServeResponse>('lab_model_predict', { modelId, inputs });
  }

  async modelListEndpoints(): Promise<ServeEndpoint[]> {
    return invoke<ServeEndpoint[]>('lab_model_list_endpoints');
  }

  async modelServeStats(): Promise<ServeStats> {
    return invoke<ServeStats>('lab_model_serve_stats');
  }

  // ========== 训练计划 & 数据配方 ==========

  async trainingPlanCreate(planJson: string): Promise<string> {
    return invoke<string>('lab_training_plan_create', { planJson });
  }

  async trainingPlanValidate(planJson: string): Promise<string> {
    return invoke<string>('lab_training_plan_validate', { planJson });
  }

  async trainingPlanSummarize(planJson: string): Promise<string> {
    return invoke<string>('lab_training_plan_summarize', { planJson });
  }

  async trainingPlanPresets(presetType: string): Promise<string> {
    return invoke<string>('lab_training_plan_presets', { presetType });
  }

  async trainingPlanSave(planJson: string): Promise<string> {
    return invoke<string>('lab_training_plan_save', { planJson });
  }

  async trainingPlanList(): Promise<any[]> {
    return invoke<any[]>('lab_training_plan_list');
  }

  async trainingPlanLoad(planId: string): Promise<string> {
    return invoke<string>('lab_training_plan_load', { planId });
  }

  async trainingPlanDelete(planId: string): Promise<void> {
    return invoke<void>('lab_training_plan_delete', { planId });
  }

  async dataRecipeCreate(recipeJson: string): Promise<string> {
    return invoke<string>('lab_data_recipe_create', { recipeJson });
  }

  async dataRecipeValidate(recipeJson: string): Promise<string> {
    return invoke<string>('lab_data_recipe_validate', { recipeJson });
  }

  async dataRecipeExecute(recipeJson: string, numSamples?: number): Promise<string> {
    return invoke<string>('lab_data_recipe_execute', { recipeJson, numSamples: numSamples ?? null });
  }

  async dataRecipePresets(presetType: string): Promise<string> {
    return invoke<string>('lab_data_recipe_presets', { presetType });
  }

  async lineageGraph(): Promise<any> {
    return invoke<any>('lab_lineage_graph');
  }

  async lineageTrace(graph: any, nodeId: string): Promise<any> {
    return invoke<any>('lab_lineage_trace', { graph, nodeId });
  }

  async lineageImpact(graph: any, nodeId: string): Promise<any> {
    return invoke<any>('lab_lineage_impact', { graph, nodeId });
  }

  async lineageToMermaid(graph: any): Promise<string> {
    return invoke<string>('lab_lineage_to_mermaid', { graph });
  }

  async datasetLineage(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_lineage', { datasetId });
  }

  async modelLineage(modelId: string): Promise<any> {
    return invoke<any>('lab_model_lineage', { modelId });
  }

  async datasetQualityScore(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_quality_score', { datasetId });
  }

  async datasetVersionDiff(datasetId: string, fromVersion: string, toVersion: string): Promise<any> {
    return invoke<any>('lab_dataset_version_diff', { datasetId, fromVersion, toVersion });
  }

  async datasetRecommendForPlan(planJson: string): Promise<any> {
    return invoke<any>('lab_dataset_recommend_for_plan', { planJson });
  }

  async datasetReadinessScore(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_readiness_score', { datasetId });
  }

  async validateDataset(datasetId: string): Promise<any> {
    return invoke<any>('lab_validate_dataset', { datasetId });
  }

  async validateDatasetIntegrity(datasetId: string): Promise<any> {
    return invoke<any>('lab_validate_dataset_integrity', { datasetId });
  }

  async createDatasetSplit(datasetId: string, splitName: string, config: { train_ratio: number; val_ratio: number; test_ratio: number; shuffle: boolean; seed: number }): Promise<any> {
    return invoke<any>('lab_create_dataset_split', { datasetId, splitName, config });
  }

  async listDatasetSplits(datasetId: string): Promise<any[]> {
    return invoke<any[]>('lab_list_dataset_splits', { datasetId });
  }

  async datasetDedup(datasetId: string, config?: { similarity_threshold?: number; num_perm?: number; n_gram?: number }): Promise<any> {
    return invoke<any>('lab_dataset_dedup', { datasetId, config: config ?? null });
  }

  async datasetCheckLeakage(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_check_leakage', { datasetId });
  }

  async datasetCheckSufficiency(datasetId: string, planJson?: string): Promise<any> {
    return invoke<any>('lab_dataset_check_sufficiency', { datasetId, planJson: planJson ?? null });
  }

  async datasetCheckFeatureLeakage(datasetId: string, targetColumn: string): Promise<any> {
    return invoke<any>('lab_dataset_check_feature_leakage', { datasetId, targetColumn });
  }

  async datasetCheckSplitConsistency(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_check_split_consistency', { datasetId });
  }

  async datasetAnalyzeImbalance(datasetId: string, labelColumn: string): Promise<any> {
    return invoke<any>('lab_dataset_analyze_imbalance', { datasetId, labelColumn });
  }

  async datasetAnalyzeDrift(datasetId: string, referenceDatasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_analyze_drift', { datasetId, referenceDatasetId });
  }

  async datasetAnalyzeCorrelation(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_analyze_correlation', { datasetId });
  }

  async datasetLabelQuality(datasetId: string, labelColumn: string): Promise<any> {
    return invoke<any>('lab_dataset_label_quality', { datasetId, labelColumn });
  }

  async datasetConfidentLearning(datasetId: string, labelColumn: string): Promise<any> {
    return invoke<any>('lab_dataset_confident_learning', { datasetId, labelColumn });
  }

  async datasetLabelQualitySummary(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_label_quality_summary', { datasetId });
  }

  async datasetSliceAnalysis(datasetId: string, config: { slice_by: string; conditions: Record<string, any> }): Promise<any> {
    return invoke<any>('lab_dataset_slice_analysis', { datasetId, config });
  }

  async datasetBiasDetection(datasetId: string, config: { sensitive_column: string; label_column: string }): Promise<any> {
    return invoke<any>('lab_dataset_bias_detection', { datasetId, config });
  }

  async datasetInfluenceTracin(datasetId: string, experimentId: string): Promise<any> {
    return invoke<any>('lab_dataset_influence_tracin', { datasetId, experimentId });
  }

  async datasetInfluenceLoo(datasetId: string, experimentId: string): Promise<any> {
    return invoke<any>('lab_dataset_influence_loo', { datasetId, experimentId });
  }

  async datasetInfluenceLossDiff(datasetId: string, experimentId: string): Promise<any> {
    return invoke<any>('lab_dataset_influence_loss_diff', { datasetId, experimentId });
  }

  async datasetSetCard(datasetId: string, card: any): Promise<any> {
    return invoke<any>('lab_dataset_set_card', { datasetId, card });
  }

  async datasetGetCard(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_get_card', { datasetId });
  }

  async datasetDiscoverySearch(query: string, filters?: Record<string, any>): Promise<any> {
    return invoke<any>('lab_dataset_discovery_search', { query, filters: filters ?? null });
  }

  async datasetUsageStats(datasetId: string): Promise<any> {
    return invoke<any>('lab_dataset_usage_stats', { datasetId });
  }

  async datasetMultimodalImages(datasetId: string, offset?: number, limit?: number): Promise<any> {
    return invoke<any>('lab_dataset_multimodal_images', { datasetId, offset: offset ?? null, limit: limit ?? null });
  }

  async datasetMultimodalTexts(datasetId: string, offset?: number, limit?: number): Promise<any> {
    return invoke<any>('lab_dataset_multimodal_texts', { datasetId, offset: offset ?? null, limit: limit ?? null });
  }

  async datasetCreateKfold(datasetId: string, k: number, shuffle: boolean, seed: number): Promise<any> {
    return invoke<any>('lab_dataset_create_kfold', { datasetId, k, shuffle, seed });
  }

  async datasetRowDiff(datasetId: string, fromVersion: string, toVersion: string, offset?: number, limit?: number): Promise<any> {
    return invoke<any>('lab_dataset_row_diff', { datasetId, fromVersion, toVersion, offset: offset ?? null, limit: limit ?? null });
  }

  async datasetListAugmentationPresets(format: string): Promise<any> {
    return invoke<any>('lab_dataset_list_augmentation_presets', { format });
  }

  async datasetLazyInspect(path: string, format: string): Promise<any> {
    return invoke<any>('lab_dataset_lazy_inspect', { path, format });
  }

  async datasetLazyReadChunk(path: string, format: string, offset: number, limit: number): Promise<any> {
    return invoke<any>('lab_dataset_lazy_read_chunk', { path, format, offset, limit });
  }

  async datasetRecommendChunkSize(path: string, format: string): Promise<any> {
    return invoke<any>('lab_dataset_recommend_chunk_size', { path, format });
  }

  async datasetCuration(datasetId: string, config: any): Promise<any> {
    return invoke<any>('lab_dataset_curation', { datasetId, config });
  }

  async curationConfig(): Promise<any> {
    return invoke<any>('lab_curation_config');
  }

  async curationMaskPii(text: string): Promise<any> {
    return invoke<any>('lab_curation_mask_pii', { text });
  }

  async datasetPreview(datasetId: string, offset?: number, limit?: number): Promise<any> {
    return invoke<any>('lab_dataset_preview', { datasetId, offset: offset ?? null, limit: limit ?? null });
  }

  async datasetSample(datasetId: string, n: number, seed?: number): Promise<any> {
    return invoke<any>('lab_dataset_sample', { datasetId, n, seed: seed ?? null });
  }

  async datasetColumnStats(datasetId: string, columnName: string): Promise<any> {
    return invoke<any>('lab_dataset_column_stats', { datasetId, columnName });
  }

  async datasetReadSplit(datasetId: string, splitName: string, offset?: number, limit?: number): Promise<any> {
    return invoke<any>('lab_dataset_read_split', { datasetId, splitName, offset: offset ?? null, limit: limit ?? null });
  }

  async streamingOpenCsv(path: string, config?: any): Promise<any> {
    return invoke<any>('lab_streaming_open_csv', { path, config: config ?? null });
  }

  async streamingOpenJsonl(path: string, config?: any): Promise<any> {
    return invoke<any>('lab_streaming_open_jsonl', { path, config: config ?? null });
  }

  async streamingRecommendChunk(path: string, format: string): Promise<any> {
    return invoke<any>('lab_streaming_recommend_chunk', { path, format });
  }

  async dataCollate(datasetIds: string[], strategy: string): Promise<any> {
    return invoke<any>('lab_data_collate', { datasetIds, strategy });
  }

  async dataLoaderCreate(config: any): Promise<any> {
    return invoke<any>('lab_data_loader_create', { config });
  }

  async dataVersionInit(path: string): Promise<any> {
    return invoke<any>('lab_data_version_init', { path });
  }

  async dataVersionCommit(path: string, message: string): Promise<any> {
    return invoke<any>('lab_data_version_commit', { path, message });
  }

  async dataVersionLog(path: string): Promise<any> {
    return invoke<any>('lab_data_version_log', { path });
  }

  async dataVersionCheckout(path: string, version: string): Promise<any> {
    return invoke<any>('lab_data_version_checkout', { path, version });
  }

  async dataVersionDiff(path: string, fromVersion: string, toVersion: string): Promise<any> {
    return invoke<any>('lab_data_version_diff', { path, fromVersion, toVersion });
  }

  async dataVersionBranches(path: string): Promise<any> {
    return invoke<any>('lab_data_version_branches', { path });
  }

  async dataVersionCreateBranch(path: string, branchName: string): Promise<any> {
    return invoke<any>('lab_data_version_create_branch', { path, branchName });
  }
}
