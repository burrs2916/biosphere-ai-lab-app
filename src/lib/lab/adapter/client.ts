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
  AppSettings,
  SessionId,
  ExperimentSummary,
  ExperimentDetail,
  MetricsTimeline,
  TrainingConfig,
  FileFilter,
  ModelRegistration,
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

export interface LabClient {
  getState(): Promise<LabStateSnapshot>;
  getDashboardStats(): Promise<DashboardStats>;
  getResourceSnapshot(): Promise<ResourceSnapshot>;
  listEngines(): Promise<PluginInfo[]>;
  listTasks(): Promise<PluginInfo[]>;
  listModels(): Promise<PluginInfo[]>;
  listDataSources(): Promise<PluginInfo[]>;
  getHardwareInfo(): Promise<HardwareInfo>;
  getRecommendations(hardware: HardwareInfo, taskType: string, dataSize: number): Promise<TrainingRecommendation>;
  loadData(config: DataLoadConfig): Promise<DatasetInfo>;
  previewData(config: DataLoadConfig, offset?: number, limit?: number): Promise<DataPreview>;
  getModelArch(modelId: string): Promise<ModelArchDef>;

  createExperiment(name: string, taskType: string, config: TrainingConfig): Promise<string>;
  listExperiments(group?: string): Promise<ExperimentSummary[]>;
  getExperimentDetail(experimentId: string): Promise<ExperimentDetail>;
  queryMetrics(experimentId: string, metricNames: string[]): Promise<MetricsTimeline>;
  queryMetricsDownsampled(experimentId: string, metricNames: string[], maxPoints?: number, smoothAlpha?: number): Promise<MetricsTimeline>;
  loadLogs(experimentId: string, limit?: number): Promise<LogEntry[]>;
  trackMetric(experimentId: string, metricName: string, value: number, step: number): Promise<void>;
  registerModel(name: string, version: string, framework: string): Promise<string>;

  listModelRegistrations(statusFilter?: string): Promise<ModelRegistration[]>;
  getModelRegistration(modelId: string): Promise<ModelRegistration>;
  promoteModelStaging(modelId: string): Promise<void>;
  promoteModelProduction(modelId: string): Promise<void>;
  demoteModelStaging(modelId: string): Promise<void>;
  archiveModel(modelId: string): Promise<void>;
  addModelAlias(modelId: string, alias: string): Promise<void>;
  removeModelAlias(modelId: string, alias: string): Promise<void>;
  deleteModelRegistration(modelId: string): Promise<void>;
  setModelPath(modelId: string, path: string): Promise<void>;
  registerModelFromExperiment(experimentId: string, name: string, version: string): Promise<void>;
  addModelVersion(modelId: string, path: string, description?: string): Promise<void>;
  listModelVersions(modelId: string): Promise<ModelVersion[]>;
  setModelDescription(modelId: string, description: string): Promise<void>;
  addModelTag(modelId: string, tag: string): Promise<void>;
  removeModelTag(modelId: string, tag: string): Promise<void>;

  startTraining(name: string, taskType: string, config: TrainingConfig): Promise<string>;
  stopTraining(experimentId: string): Promise<void>;
  pauseTraining(experimentId: string): Promise<void>;
  resumeTraining(experimentId: string): Promise<void>;
  resumeFromCheckpoint(experimentId: string, checkpointEpoch: number): Promise<void>;
  runInference(experimentId: string, input_data: number[][]): Promise<InferenceResult>;
  batchInference(experimentId: string, inputData: number[][]): Promise<InferenceResult>;
  preprocessData(dataPath: string, dataFormat: string, steps: PipelineStep[]): Promise<DataPreview>;
  experimentAddTag(experimentId: string, tag: string): Promise<void>;
  experimentSetParam(experimentId: string, key: string, value: unknown): Promise<void>;
  experimentSetDescription(experimentId: string, description: string): Promise<void>;

  listArtifacts(experimentId: string): Promise<ArtifactRef[]>;
  getArtifactContent(experimentId: string, artifactPath: string): Promise<number[]>;
  scanArtifacts(experimentId: string): Promise<ArtifactRef[]>;
  openArtifactDir(experimentId: string): Promise<void>;
  listCheckpoints(experimentId: string): Promise<CheckpointInfo[]>;
  deleteCheckpoint(experimentId: string, checkpointName: string): Promise<void>;
  evaluateModel(experimentId: string, testDataPath: string): Promise<EvaluationResult>;
  saveEvaluation(experimentId: string, evaluationResult: any, testDataPath: string): Promise<string>;
  listEvaluations(experimentId: string): Promise<any[]>;
  deleteExperiment(experimentId: string): Promise<void>;
  archiveExperiment(experimentId: string): Promise<void>;
  restoreExperiment(experimentId: string): Promise<void>;
  batchDeleteExperiments(experimentIds: string[]): Promise<number>;
  cloneExperiment(experimentId: string, newName: string): Promise<string>;
  setExperimentGroup(experimentId: string, group: string): Promise<void>;
  listExperimentGroups(): Promise<string[]>;

  registerDataset(name: string, format: string, path: string): Promise<DatasetRegistration>;
  listDatasets(statusFilter?: string, formatFilter?: string, nameContains?: string): Promise<DatasetSummary[]>;
  getDataset(datasetId: string): Promise<DatasetRegistration>;
  deleteDataset(datasetId: string): Promise<void>;
  archiveDataset(datasetId: string): Promise<void>;
  restoreDataset(datasetId: string): Promise<void>;
  datasetAddTag(datasetId: string, tag: string): Promise<void>;
  datasetRemoveTag(datasetId: string, tag: string): Promise<void>;
  datasetSetDescription(datasetId: string, description: string): Promise<void>;
  datasetLinkExperiment(datasetId: string, experimentId: string): Promise<void>;
  datasetNewVersion(datasetId: string): Promise<DatasetRegistration>;
  datasetVersionHistory(datasetId: string): Promise<DatasetVersionRecord[]>;
  datasetNewVersionWithNote(datasetId: string, changeNote: string): Promise<DatasetRegistration>;

  listConnectors(): Promise<ConnectorInfo[]>;
  scanDataSources(uri: string, options?: { recursive?: boolean; max_depth?: number; extensions?: string[]; max_results?: number }): Promise<DiscoveredItem[]>;
  testDataConnection(uri: string): Promise<boolean>;
  resolveDataItem(item: DiscoveredItem): Promise<ResolvedDataSource>;
  quickRegisterDataset(item: DiscoveredItem): Promise<DatasetRegistration>;

  selectFile(filters?: FileFilter[]): Promise<string | null>;
  selectDirectory(): Promise<string | null>;

  getSettings(): Promise<AppSettings>;
  saveSettings(settings: AppSettings): Promise<void>;

  onLabEvent(handler: (event: LabEvent) => void): Promise<() => void>;
  onSessionEvent(sessionId: SessionId, handler: (event: LabEvent) => void): Promise<() => void>;

  startHyperparameterTuning(tuneConfig: TuneConfig): Promise<TuneResult>;
  generateHparamCombinations(hparamSpace: HparamSpace, strategy: TuneStrategy): Promise<Record<string, HparamValue>[]>;
  exportModel(experimentId: string, format: string, outputPath: string | null, opsetVersion: number | null, inputShapes: number[][]): Promise<ExportResult>;
  listExportFormats(engineId: string): Promise<string[]>;
  exportDataset(datasetId: string, targetFormat: DataExportFormat, outputPath: string | null): Promise<DataExportResult>;
  modelDeploy(modelId: string): Promise<void>;
  modelUndeploy(modelId: string): Promise<void>;
  modelPredict(modelId: string, inputs: number[][]): Promise<ServeResponse>;
  modelListEndpoints(): Promise<ServeEndpoint[]>;
  modelServeStats(): Promise<ServeStats>;

  trainingPlanCreate(planJson: string): Promise<string>;
  trainingPlanValidate(planJson: string): Promise<string>;
  trainingPlanSummarize(planJson: string): Promise<string>;
  trainingPlanPresets(presetType: string): Promise<string>;
  trainingPlanSave(planJson: string): Promise<string>;
  trainingPlanList(): Promise<any[]>;
  trainingPlanLoad(planId: string): Promise<string>;
  trainingPlanDelete(planId: string): Promise<void>;
  dataRecipeCreate(recipeJson: string): Promise<string>;
  dataRecipeValidate(recipeJson: string): Promise<string>;
  dataRecipeExecute(recipeJson: string, numSamples?: number): Promise<string>;
  dataRecipePresets(presetType: string): Promise<string>;

  lineageGraph(): Promise<any>;
  lineageTrace(graph: any, nodeId: string): Promise<any>;
  lineageImpact(graph: any, nodeId: string): Promise<any>;
  lineageToMermaid(graph: any): Promise<string>;
  datasetLineage(datasetId: string): Promise<any>;
  modelLineage(modelId: string): Promise<any>;
  datasetQualityScore(datasetId: string): Promise<any>;
  datasetVersionDiff(datasetId: string, fromVersion: string, toVersion: string): Promise<any>;
  datasetRecommendForPlan(planJson: string): Promise<any>;
  datasetReadinessScore(datasetId: string): Promise<any>;
  validateDataset(datasetId: string): Promise<any>;
  validateDatasetIntegrity(datasetId: string): Promise<any>;
  createDatasetSplit(datasetId: string, splitName: string, config: { train_ratio: number; val_ratio: number; test_ratio: number; shuffle: boolean; seed: number }): Promise<any>;
  listDatasetSplits(datasetId: string): Promise<any[]>;
  datasetDedup(datasetId: string, config?: { similarity_threshold?: number; num_perm?: number; n_gram?: number }): Promise<any>;
  datasetCheckLeakage(datasetId: string): Promise<any>;
  datasetCheckSufficiency(datasetId: string, planJson?: string): Promise<any>;
  datasetCheckFeatureLeakage(datasetId: string, targetColumn: string): Promise<any>;
  datasetCheckSplitConsistency(datasetId: string): Promise<any>;
  datasetAnalyzeImbalance(datasetId: string, labelColumn: string): Promise<any>;
  datasetAnalyzeDrift(datasetId: string, referenceDatasetId: string): Promise<any>;
  datasetAnalyzeCorrelation(datasetId: string): Promise<any>;
  datasetLabelQuality(datasetId: string, labelColumn: string): Promise<any>;
  datasetConfidentLearning(datasetId: string, labelColumn: string): Promise<any>;
  datasetLabelQualitySummary(datasetId: string): Promise<any>;
  datasetSliceAnalysis(datasetId: string, config: { slice_by: string; conditions: Record<string, any> }): Promise<any>;
  datasetBiasDetection(datasetId: string, config: { sensitive_column: string; label_column: string }): Promise<any>;
  datasetInfluenceTracin(datasetId: string, experimentId: string): Promise<any>;
  datasetInfluenceLoo(datasetId: string, experimentId: string): Promise<any>;
  datasetInfluenceLossDiff(datasetId: string, experimentId: string): Promise<any>;
  datasetSetCard(datasetId: string, card: any): Promise<any>;
  datasetGetCard(datasetId: string): Promise<any>;
  datasetDiscoverySearch(query: string, filters?: Record<string, any>): Promise<any>;
  datasetUsageStats(datasetId: string): Promise<any>;
  datasetMultimodalImages(datasetId: string, offset?: number, limit?: number): Promise<any>;
  datasetMultimodalTexts(datasetId: string, offset?: number, limit?: number): Promise<any>;
  datasetCreateKfold(datasetId: string, k: number, shuffle: boolean, seed: number): Promise<any>;
  datasetRowDiff(datasetId: string, fromVersion: string, toVersion: string, offset?: number, limit?: number): Promise<any>;
  datasetListAugmentationPresets(format: string): Promise<any>;
  datasetLazyInspect(path: string, format: string): Promise<any>;
  datasetLazyReadChunk(path: string, format: string, offset: number, limit: number): Promise<any>;
  datasetRecommendChunkSize(path: string, format: string): Promise<any>;
  datasetCuration(datasetId: string, config: any): Promise<any>;
  curationConfig(): Promise<any>;
  curationMaskPii(text: string): Promise<any>;
  datasetPreview(datasetId: string, offset?: number, limit?: number): Promise<any>;
  datasetSample(datasetId: string, n: number, seed?: number): Promise<any>;
  datasetColumnStats(datasetId: string, columnName: string): Promise<any>;
  datasetReadSplit(datasetId: string, splitName: string, offset?: number, limit?: number): Promise<any>;
  streamingOpenCsv(path: string, config?: any): Promise<any>;
  streamingOpenJsonl(path: string, config?: any): Promise<any>;
  streamingRecommendChunk(path: string, format: string): Promise<any>;
  dataCollate(datasetIds: string[], strategy: string): Promise<any>;
  dataLoaderCreate(config: any): Promise<any>;
  dataVersionInit(path: string): Promise<any>;
  dataVersionCommit(path: string, message: string): Promise<any>;
  dataVersionLog(path: string): Promise<any>;
  dataVersionCheckout(path: string, version: string): Promise<any>;
  dataVersionDiff(path: string, fromVersion: string, toVersion: string): Promise<any>;
  dataVersionBranches(path: string): Promise<any>;
  dataVersionCreateBranch(path: string, branchName: string): Promise<any>;
}
