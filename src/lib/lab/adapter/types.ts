export type SessionId = string;

export type PluginId = string;

export interface PluginInfo {
  id: PluginId;
  name: string;
  version: string;
  description: string;
  plugin_kind: PluginKind;
}

export type PluginKind = 'engine' | 'task' | 'model' | 'data_source' | 'remote';

export interface SessionInfo {
  id: SessionId;
  name: string;
  status: SessionStatus;
  config: TrainingConfig;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
  current_epoch: number;
  total_epochs: number;
  error_message: string | null;
}

export type SessionStatus =
  | 'created'
  | 'configuring'
  | 'loading_data'
  | 'ready'
  | 'training'
  | 'paused'
  | 'evaluating'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type ComputeBackend = 'cpu' | 'cuda' | 'wgpu' | 'metal' | 'rocm';

export type TaskType =
  | 'classification'
  | 'regression'
  | 'clustering'
  | 'detection'
  | 'segmentation'
  | 'generation'
  | 'nlp'
  | 'custom';

export type DataFormat = 'csv' | 'json' | 'image' | 'text' | 'binary' | 'parquet' | 'excel' | 'tfrecord' | 'huggingface' | 'database';

export type MetricType =
  | 'loss' | 'accuracy' | 'precision' | 'recall' | 'f1_score'
  | 'mse' | 'rmse' | 'mae' | 'r2' | 'auc' | 'custom';

export type ArchType =
  | 'mlp' | 'cnn' | 'rnn' | 'lstm' | 'gru'
  | 'transformer' | 'autoencoder' | 'gan' | 'custom';

export interface TrainingConfig {
  session_name: string;
  task_type: TaskType;
  engine_id: string;
  model_id: string;
  data_source_id: string;
  data_path: string;
  epochs: number;
  batch_size: number;
  learning_rate: number;
  optimizer: OptimizerConfig;
  loss_function: string;
  compute_backend: ComputeBackend;
  data_format: DataFormat;
  validation_split: number;
  test_split: number;
  shuffle: boolean;
  seed: number | null;
  checkpoint_interval: number | null;
  early_stopping: EarlyStoppingConfig | null;
  lr_scheduler: LrSchedulerConfig;
  target_columns: string[];
  feature_columns: string[];
  custom_params: Record<string, unknown>;
}

export type OptimizerConfig =
  | { Sgd: { momentum: number | null; weight_decay: number | null } }
  | { Adam: { beta1: number; beta2: number; weight_decay: number | null } }
  | { AdamW: { beta1: number; beta2: number; weight_decay: number } }
  | { Rmsprop: { alpha: number; weight_decay: number | null } }
  | { Custom: { name: string; params: Record<string, unknown> } };

export interface EarlyStoppingConfig {
  metric: MetricType;
  patience: number;
  min_delta: number;
  mode: 'min' | 'max';
}

export type LrSchedulerConfig =
  | 'Constant'
  | { Step: { step_size: number; gamma: number } }
  | { Exponential: { gamma: number } }
  | { CosineAnnealing: { min_lr: number; num_iters: number } }
  | { Linear: { final_lr: number; num_iters: number } };

export interface HardwareInfo {
  cpu_cores: number;
  cpu_model: string;
  total_memory_mb: number;
  available_memory_mb: number;
  cpu_usage_percent: number;
  gpu_devices: GpuInfo[];
  os_name: string;
  os_version: string;
}

export interface GpuInfo {
  name: string;
  vram_mb: number;
  compute_backend: ComputeBackend;
  driver_version: string | null;
}

export interface TrainingRecommendation {
  recommended_batch_size: number;
  recommended_epochs: number;
  recommended_backend: ComputeBackend;
  recommended_learning_rate: number;
  estimated_training_time_minutes: number;
  can_train_locally: boolean;
  should_use_remote: boolean;
  warnings: string[];
}

export interface DatasetInfo {
  name: string;
  format: DataFormat;
  rows: number;
  columns: number;
  column_names: string[];
  column_types: string[];
  has_missing_values: boolean;
  memory_size_mb: number;
}

export type ColumnType = 'integer' | 'float' | 'boolean' | 'string' | 'datetime' | 'categorical' | 'unknown';

export interface ColumnProfile {
  name: string;
  column_type: ColumnType;
  null_count: number;
  distinct_count: number;
  total_count: number;
  min_value: string | null;
  max_value: string | null;
  mean_value: number | null;
  std_value: number | null;
  median_value: number | null;
  top_values: [string, number][];
}

export type DatasetStatus = 'active' | 'archived' | 'deleted';

export interface DatasetRegistration {
  id: string;
  name: string;
  version: string;
  status: DatasetStatus;
  format: DataFormat;
  path: string;
  digest: string;
  rows: number;
  columns: number;
  column_profiles: ColumnProfile[];
  memory_size_mb: number;
  tags: string[];
  description: string | null;
  source_type: string | null;
  source_uri: string | null;
  experiment_ids: string[];
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface DatasetSummary {
  id: string;
  name: string;
  version: string;
  status: DatasetStatus;
  format: DataFormat;
  rows: number;
  columns: number;
  has_missing_values: boolean;
  memory_size_mb: number;
  tags: string[];
  experiment_count: number;
  created_at: string;
  updated_at: string;
}

export interface DatasetVersionRecord {
  version: string;
  digest: string;
  rows: number;
  columns: number;
  memory_size_mb: number;
  created_at: string;
  change_note: string | null;
}

export interface DataPreview {
  columns: string[];
  column_types: string[];
  rows: unknown[][];
  total_rows: number;
  offset: number;
}

export interface DataLoadConfig {
  path: string;
  format: DataFormat;
  has_header: boolean;
  delimiter: string | null;
  encoding: string | null;
  max_rows: number | null;
  custom_params: Record<string, unknown>;
}

export interface ConnectorInfo {
  id: string;
  name: string;
  description: string;
  connector_type: string;
  supported_formats: DataFormat[];
  requires_auth: boolean;
}

export interface DiscoveredItem {
  name: string;
  path: string;
  format: DataFormat;
  size_bytes: number;
  connector_type: string;
  is_directory: boolean;
  children_count: number | null;
  metadata: Record<string, unknown>;
}

export interface ResolvedDataSource {
  name: string;
  format: DataFormat;
  path: string;
  source_type: string;
  source_uri: string;
  size_bytes: number;
  load_config_params: Record<string, unknown>;
}

export interface ScanOptions {
  recursive?: boolean;
  max_depth?: number;
  extensions?: string[];
  max_results?: number;
}

export interface ModelArchDef {
  id: PluginId;
  arch_type: ArchType;
  layers: LayerDescription[];
  total_params: number;
  input_shape: TensorShape;
  output_shape: TensorShape;
}

export interface LayerDescription {
  layer_type: string;
  name: string;
  input_shape: TensorShape;
  output_shape: TensorShape;
  params: number;
  config: unknown;
}

export interface TensorShape {
  dims: number[];
}

export interface EpochMetrics {
  epoch: number;
  train_loss: number;
  val_loss: number | null;
  metrics: Record<string, number>;
  learning_rate: number;
  epoch_time_ms: number;
}

export interface InferenceResult {
  predictions: number[];
  predicted_classes: number[];
  probabilities: number[][];
}

export interface PipelineStep {
  step_type: PreprocessType;
  params: Record<string, unknown>;
}

export type PreprocessType =
  | 'Normalize'
  | 'Standardize'
  | 'OneHotEncode'
  | 'LabelEncode'
  | 'FillMissing'
  | 'DropMissing'
  | 'Tokenize'
  | 'PadSequence'
  | 'AugmentImage'
  | 'ResizeImage'
  | { Custom: string };

export type LabEvent =
  | { type: 'SessionCreated'; payload: { session_id: SessionId } }
  | { type: 'SessionStarted'; payload: { session_id: SessionId } }
  | { type: 'SessionPaused'; payload: { session_id: SessionId } }
  | { type: 'SessionResumed'; payload: { session_id: SessionId } }
  | { type: 'SessionCompleted'; payload: { session_id: SessionId; final_metrics: unknown } }
  | { type: 'SessionFailed'; payload: { session_id: SessionId; error: string } }
  | { type: 'SessionCancelled'; payload: { session_id: SessionId } }
  | { type: 'EpochCompleted'; payload: { session_id: SessionId; epoch: number; total_epochs: number; train_loss: number; val_loss: number | null; metrics: unknown } }
  | { type: 'BatchCompleted'; payload: { session_id: SessionId; batch: number; total_batches: number; loss: number } }
  | { type: 'CheckpointSaved'; payload: { session_id: SessionId; path: string; epoch: number } }
  | { type: 'DataLoaded'; payload: { session_id: SessionId; rows: number; columns: number } }
  | { type: 'HardwareAlert'; payload: { session_id: string | null; cpu_usage: number; memory_usage: number; memory_total_mb: number; memory_available_mb: number; disk_total_gb: number; disk_available_gb: number; disk_usage_percent: number; gpu_usage: number | null; gpu_memory_used_mb: number | null; gpu_memory_total_mb: number | null; message: string } }
  | { type: 'ProgressUpdate'; payload: { session_id: SessionId; progress: number; message: string } }
  | { type: 'LogOutput'; payload: { session_id: SessionId; level: string; message: string } }
  | { type: 'Heartbeat'; payload: { session_id: SessionId; epoch: number; total_epochs: number; elapsed_secs: number } }
  | { type: 'DatasetRegistered'; payload: { dataset_id: string; name: string; format: string; rows: number; columns: number } }
  | { type: 'DatasetDeleted'; payload: { dataset_id: string } }
  | { type: 'DatasetArchived'; payload: { dataset_id: string } }
  | { type: 'DatasetRestored'; payload: { dataset_id: string } }
  | { type: 'DownloadProgress'; payload: { task_id: string; progress: number; downloaded_bytes: number; total_bytes: number | null; speed_mbps: number; message: string } }
  | { type: 'DedupProgress'; payload: { task_id: string; progress: number; processed: number; total: number; duplicates_found: number; message: string } }
  | { type: 'CurationProgress'; payload: { task_id: string; step: string; progress: number; message: string } }
  | { type: 'OperationCompleted'; payload: { task_id: string; operation: string; result: unknown } }
  | { type: 'OperationFailed'; payload: { task_id: string; operation: string; error: string } }
  | { type: 'Custom'; payload: unknown };

export interface LabStateSnapshot {
  active_experiments: number;
  registered_engines: number;
  registered_tasks: number;
  registered_models: number;
  registered_data_sources: number;
}

export interface DashboardStats {
  total_experiments: number;
  running_experiments: number;
  completed_experiments: number;
  failed_experiments: number;
  total_models: number;
  production_models: number;
  status_counts: Record<string, number>;
  task_type_counts: Record<string, number>;
}

export interface ResourceSnapshot {
  cpu_usage_percent: number;
  memory_total_mb: number;
  memory_available_mb: number;
  memory_usage_percent: number;
  disk_total_gb: number;
  disk_available_gb: number;
  disk_usage_percent: number;
  gpu_usage_percent: number | null;
  gpu_memory_used_mb: number | null;
  gpu_memory_total_mb: number | null;
  timestamp: number;
}

export type ExperimentStatus =
  | 'created'
  | 'running'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'archived';

export type ExperimentId = string;

export interface ExperimentSummary {
  id: ExperimentId;
  name: string;
  status: ExperimentStatus;
  task_type: TaskType;
  tags: string[];
  dataset_id: string | null;
  dataset_version: string | null;
  group: string | null;
  created_at: string;
  updated_at: string;
  metric_names: string[];
  best_metrics: Record<string, number>;
}

export interface ExperimentDetail {
  id: ExperimentId;
  name: string;
  description: string | null;
  status: ExperimentStatus;
  task_type: TaskType;
  config: TrainingConfig;
  metrics: MetricsTimeline;
  params: Record<string, unknown>;
  tags: string[];
  artifacts: ArtifactRef[];
  model_id: string | null;
  dataset_id: string | null;
  dataset_version: string | null;
  group: string | null;
  environment: EnvironmentInfo | null;
  final_metrics: Record<string, number> | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  error_message: string | null;
}

export interface GitInfo {
  commit_hash: string | null;
  branch: string | null;
  commit_message: string | null;
  is_dirty: boolean | null;
  remote_url: string | null;
}

export interface DependencyInfo {
  rust_version: string | null;
  burn_version: string | null;
  crates: Record<string, string>;
}

export interface SystemInfo {
  os: string;
  os_version: string;
  cpu_cores: number;
  total_memory_mb: number;
  hostname: string;
}

export interface EnvironmentInfo {
  git: GitInfo | null;
  dependencies: DependencyInfo | null;
  system: SystemInfo | null;
  captured_at: string;
}

export interface ArtifactRef {
  artifact_type: string;
  path: string;
  size_bytes: number;
  created_at: string;
  version: string | null;
  description: string | null;
  checksum: string | null;
  metadata: unknown | null;
}

export interface MetricPoint {
  step: number;
  value: number;
  timestamp: string;
  epoch: number | null;
}

export interface MetricSeries {
  name: string;
  values: MetricPoint[];
}

export interface MetricsTimeline {
  series: Record<string, MetricSeries>;
}

export interface LogEntry {
  level: string;
  message: string;
  timestamp: string;
}

export interface FileFilter {
  name: string;
  extensions: string[];
}

export type ModelRegistrationStatus = 'none' | 'staging' | 'production' | 'archived';

export type ModelRegistrationId = string;

export interface ModelRegistration {
  id: ModelRegistrationId;
  name: string;
  version: string;
  status: ModelRegistrationStatus;
  framework: string;
  path: string | null;
  signature: unknown | null;
  structured_signature: ModelSignature | null;
  lineage: ModelLineage | null;
  metadata: Record<string, unknown>;
  description: string | null;
  tags: string[];
  aliases: string[];
  created_at: string;
  updated_at: string;
}

export interface TensorSpec {
  name: string;
  dtype: string;
  shape: number[];
}

export interface ModelSignature {
  inputs: TensorSpec[];
  outputs: TensorSpec[];
}

export interface ModelLineage {
  experiment_id: string | null;
  experiment_name: string | null;
  training_config: unknown | null;
  parent_model_id: string | null;
  dataset: string | null;
}

export interface AppSettings {
  general: GeneralSettings;
  training: TrainingSettings;
  storage: StorageSettings;
}

export interface GeneralSettings {
  language: string;
  theme: string;
  log_level: string;
  auto_refresh_interval: number;
}

export interface TrainingSettings {
  default_compute_backend: string;
  default_engine: string;
  max_concurrent_experiments: number;
  auto_checkpoint: boolean;
  checkpoint_interval: number;
}

export interface StorageSettings {
  data_directory: string;
  model_directory: string;
  checkpoint_directory: string;
  max_storage_gb: number;
}

export interface ModelVersion {
  version: string;
  path: string;
  description: string | null;
  created_at: string;
  size_bytes: number;
}

export interface CheckpointInfo {
  name: string;
  epoch: number;
  path: string;
  checkpoint_file: string | null;
  size_bytes: number;
  modified_timestamp: number | null;
}

export interface ClassificationEvaluation {
  task_type: 'classification';
  total_samples: number;
  accuracy: number;
  confusion_matrix: number[][];
  class_metrics: ClassMetric[];
  macro_precision?: number;
  macro_recall?: number;
  macro_f1?: number;
}

export interface ClassMetric {
  class: number;
  precision: number;
  recall: number;
  f1_score: number;
  support: number;
}

export interface RegressionEvaluation {
  task_type: 'regression';
  total_samples: number;
  mse: number;
  rmse: number;
  mae: number;
  r_squared?: number;
}

export type EvaluationResult = ClassificationEvaluation | RegressionEvaluation;

export type HparamValue =
  | { Float: number }
  | { Int: number }
  | { String: string };

export type HparamRange =
  | { FloatRange: { min: number; max: number } }
  | { IntRange: { min: number; max: number } }
  | { Choice: HparamValue[] };

export interface HparamSpace {
  params: Record<string, HparamRange>;
}

export type TuneStrategy =
  | 'Grid'
  | { Random: { n_trials: number } };

export interface TuneConfig {
  base_config: TrainingConfig;
  hparam_space: HparamSpace;
  strategy: TuneStrategy;
  metric_to_optimize: string;
  maximize: boolean;
  max_concurrent: number;
}

export type TrialStatus = 'Pending' | 'Running' | 'Completed' | 'Failed';

export interface TrialResult {
  experiment_id: string;
  params: Record<string, HparamValue>;
  metric_value: number | null;
  status: TrialStatus;
}

export interface TuneResult {
  tune_id: string;
  strategy: TuneStrategy;
  trials: TrialResult[];
  best_trial: TrialResult | null;
  best_params: Record<string, HparamValue> | null;
}

export type ExportFormat = 'TorchScript' | 'Onnx' | 'BurnRecord';

export interface ExportRequest {
  experiment_id: string;
  format: ExportFormat;
  output_path: string | null;
  opset_version: number | null;
  input_shapes: number[][];
}

export interface ExportResult {
  success: boolean;
  format: ExportFormat;
  output_path: string;
  file_size_bytes: number;
  message: string;
}

export type DataExportFormat = 'csv' | 'json' | 'parquet';

export interface DataExportResult {
  success: boolean;
  source_format: DataFormat;
  target_format: DataExportFormat;
  output_path: string;
  file_size_bytes: number;
  rows_exported: number;
  message: string;
}

export interface ServeRequest {
  model_id: string;
  inputs: number[][];
}

export interface ServeResponse {
  model_id: string;
  model_name: string;
  model_version: string;
  predictions: number[];
  predicted_classes: number[];
  probabilities: number[][];
  latency_ms: number;
}

export interface ServeEndpoint {
  model_id: string;
  model_name: string;
  model_version: string;
  status: string;
  url: string;
  input_shape: number[];
  output_shape: number[];
  request_count: number;
  avg_latency_ms: number;
}

export interface ServeStats {
  total_endpoints: number;
  active_endpoints: number;
  total_requests: number;
  avg_latency_ms: number;
}

// ========== 训练计划 & 数据配方 ==========

export type PlanType = 'Pretraining' | 'FineTuning' | 'InstructionTuning' | 'RLHF' | 'Custom';

export type MixingStrategy =
  | 'Proportional'
  | 'Interleaved'
  | { Curriculum: { stages: CurriculumStage[] } }
  | { DynamicRatio: { metric: string; update_interval: number } };

export interface CurriculumStage {
  name: string;
  datasets: string[];
  samples: number;
  difficulty: number;
}

export interface RecipeDataset {
  name: string;
  weight: number;
  source: string;
  split: string;
  local_path: string | null;
  max_samples: number | null;
  filters: Record<string, string> | null;
}

export interface QualityThresholds {
  min_text_length: number;
  max_text_length: number;
  min_perplexity: number | null;
  max_perplexity: number | null;
  language_whitelist: string[] | null;
  min_quality_score: number | null;
  remove_duplicates: boolean;
  remove_pii: boolean;
}

export interface DataRecipe {
  name: string;
  version: string;
  description: string | null;
  datasets: RecipeDataset[];
  mixing_strategy: MixingStrategy;
  curriculum: { stages: CurriculumStage[] } | null;
  dynamic_ratio: { metric: string; update_interval: number } | null;
  quality_thresholds: QualityThresholds | null;
  total_samples_target: number | null;
  seed: number;
}

export interface DataBudget {
  max_tokens: number;
  max_samples: number;
  max_cost_usd: number;
  token_budget_per_phase: Record<string, number>;
}

export interface QualityGate {
  name: string;
  metric: string;
  threshold: number;
  action: 'warn' | 'skip' | 'abort';
  phase: string;
}

export interface PreprocessingConfig {
  tokenizer_path: string;
  max_seq_length: number;
  packing: boolean;
  add_special_tokens: boolean;
  truncation: 'left' | 'right' | 'none';
  padding: 'max_length' | 'longest' | 'none';
}

export interface ValidationConfig {
  val_split: number;
  val_datasets: string[];
  metrics: string[];
  eval_interval_steps: number;
}

export interface ExperimentTracking {
  project_name: string;
  tags: string[];
  notes: string | null;
  log_interval_steps: number;
}

export interface PlanPhase {
  name: string;
  recipe: DataRecipe;
  steps: number;
  batch_size: number;
  learning_rate: number;
  warmup_steps: number;
  weight_decay: number;
  max_seq_length: number;
}

export interface TrainingPlan {
  name: string;
  version: string;
  description: string | null;
  plan_type: PlanType;
  phases: PlanPhase[];
  data_budget: DataBudget;
  quality_gates: QualityGate[];
  dedup_config: DedupConfig | null;
  preprocessing: PreprocessingConfig | null;
  validation: ValidationConfig | null;
  experiment_tracking: ExperimentTracking | null;
  output_dir: string;
  seed: number;
  metadata: Record<string, unknown>;
}

export interface DedupConfig {
  similarity_threshold: number;
  num_perm: number;
  n_gram: number;
  minhash_seed: number;
  num_bands: number;
}

export interface PlanValidationResult {
  is_valid: boolean;
  errors: string[];
  warnings: string[];
  checks: PlanCheck[];
}

export interface PlanCheck {
  name: string;
  status: 'Passed' | 'Warning' | 'Failed';
  message: string;
}

export interface PlanSummary {
  name: string;
  version: string;
  plan_type: PlanType;
  phases: PhaseSummary[];
  total_estimated_tokens: number;
  total_estimated_samples: number;
  total_estimated_cost_usd: number;
  quality_gates_count: number;
  datasets_count: number;
}

export interface PhaseSummary {
  name: string;
  steps: number;
  batch_size: number;
  learning_rate: number;
  datasets: string[];
  estimated_tokens: number;
  estimated_samples: number;
}

export interface RecipeValidationResult {
  valid: boolean;
  name?: string;
  datasets?: number;
  total_weight?: number;
  error?: string;
}

export interface RecipeExecutionResult {
  sequence: string[];
  stats: RecipeStats;
}

export interface RecipeStats {
  total_samples: number;
  dataset_counts: Record<string, number>;
  mixing_strategy: string;
}
