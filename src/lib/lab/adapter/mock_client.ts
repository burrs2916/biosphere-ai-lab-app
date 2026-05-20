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
  LrSchedulerConfig,
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
  DatasetVersionRecord,
  TuneConfig,
  TuneResult,
  HparamSpace,
  TuneStrategy,
  HparamValue,
  ExportResult,
  ExportFormat,
  DataExportFormat,
  DataExportResult,
  ServeResponse,
  ServeEndpoint,
  ServeStats,
  LogEntry,
} from './types';

const mockPlugins: Record<string, PluginInfo[]> = {
  engine: [
    { id: 'burn', name: 'Burn Engine', version: '0.1.0', description: 'Burn deep learning framework', plugin_kind: 'engine' },
    { id: 'tch', name: 'PyTorch Engine (tch-rs)', version: '0.1.0', description: 'PyTorch C++ API engine via tch-rs bindings, supports CUDA and CPU backends', plugin_kind: 'engine' },
  ],
  task: [
    { id: 'classification', name: 'Classification', version: '0.1.0', description: 'Classification task', plugin_kind: 'task' },
  ],
  model: [
    { id: 'mlp', name: 'MLP', version: '0.1.0', description: 'Multi-layer perceptron', plugin_kind: 'model' },
  ],
  data_source: [
    { id: 'csv', name: 'CSV Loader', version: '0.1.0', description: 'CSV data source', plugin_kind: 'data_source' },
  ],
};

const mockHardware: HardwareInfo = {
  cpu_cores: 8,
  cpu_model: 'Apple M1',
  total_memory_mb: 16384,
  available_memory_mb: 8192,
  cpu_usage_percent: 25.0,
  gpu_devices: [],
  os_name: 'macos',
  os_version: '14.0',
};

function generateLossCurve(steps: number, start: number, end: number, noise: number = 0.02): { step: number; value: number; timestamp: string; epoch: number | null }[] {
  const points: { step: number; value: number; timestamp: string; epoch: number | null }[] = [];
  for (let i = 0; i < steps; i++) {
    const progress = i / steps;
    const value = start * Math.exp(-3 * progress) + end + (Math.random() - 0.5) * noise;
    points.push({
      step: i,
      value: Math.max(value, end * 0.8),
      timestamp: new Date(Date.now() - (steps - i) * 60000).toISOString(),
      epoch: i % 10 === 0 ? Math.floor(i / 10) : null,
    });
  }
  return points;
}

function generateAccuracyCurve(steps: number): { step: number; value: number; timestamp: string; epoch: number | null }[] {
  const points: { step: number; value: number; timestamp: string; epoch: number | null }[] = [];
  for (let i = 0; i < steps; i++) {
    const progress = i / steps;
    const value = 0.1 + 0.87 * (1 - Math.exp(-4 * progress)) + (Math.random() - 0.5) * 0.02;
    points.push({
      step: i,
      value: Math.min(value, 0.99),
      timestamp: new Date(Date.now() - (steps - i) * 60000).toISOString(),
      epoch: i % 10 === 0 ? Math.floor(i / 10) : null,
    });
  }
  return points;
}

const mockTrainLoss = generateLossCurve(50, 2.3, 0.05);
const mockValLoss = generateLossCurve(50, 2.5, 0.08, 0.03);
const mockAccuracy = generateAccuracyCurve(50);

const mockMetricsTimeline: MetricsTimeline = {
  series: {
    train_loss: { name: 'train_loss', values: mockTrainLoss },
    val_loss: { name: 'val_loss', values: mockValLoss },
    accuracy: { name: 'accuracy', values: mockAccuracy },
  },
};

const mockExperiments: ExperimentSummary[] = [
  {
    id: 'exp-001',
    name: 'MNIST Classification',
    status: 'completed',
    task_type: 'classification',
    tags: ['mnist', 'baseline'],
    dataset_id: null,
    dataset_version: null,
    group: null,
    created_at: new Date(Date.now() - 86400000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString(),
    metric_names: ['train_loss', 'val_loss', 'accuracy'],
    best_metrics: { train_loss: 0.05, val_loss: 0.08, accuracy: 0.97 },
  },
  {
    id: 'exp-002',
    name: 'CIFAR-10 ResNet',
    status: 'running',
    task_type: 'classification',
    tags: ['cifar10', 'resnet'],
    dataset_id: null,
    dataset_version: null,
    group: null,
    created_at: new Date(Date.now() - 7200000).toISOString(),
    updated_at: new Date().toISOString(),
    metric_names: ['train_loss', 'val_loss'],
    best_metrics: { train_loss: 0.3, val_loss: 0.5 },
  },
  {
    id: 'exp-003',
    name: 'Sentiment Analysis BERT',
    status: 'failed',
    task_type: 'nlp',
    tags: ['bert', 'sentiment'],
    dataset_id: null,
    dataset_version: null,
    group: null,
    created_at: new Date(Date.now() - 172800000).toISOString(),
    updated_at: new Date(Date.now() - 86400000).toISOString(),
    metric_names: ['train_loss'],
    best_metrics: { train_loss: 1.2 },
  },
  {
    id: 'exp-004',
    name: 'Image Segmentation U-Net',
    status: 'paused',
    task_type: 'segmentation',
    tags: ['unet', 'medical'],
    dataset_id: null,
    dataset_version: null,
    group: null,
    created_at: new Date(Date.now() - 259200000).toISOString(),
    updated_at: new Date(Date.now() - 172800000).toISOString(),
    metric_names: ['train_loss', 'val_loss', 'iou'],
    best_metrics: { train_loss: 0.15, val_loss: 0.22, iou: 0.78 },
  },
];

const mockExperimentDetails: Record<string, ExperimentDetail> = {
  'exp-001': {
    id: 'exp-001',
    name: 'MNIST Classification',
    description: 'MNIST手写数字分类实验，使用多层感知机',
    status: 'completed',
    task_type: 'classification',
    config: {
      session_name: 'MNIST Classification',
      task_type: 'classification',
      engine_id: 'burn',
      model_id: 'mlp',
      data_source_id: 'csv',
      data_path: '/data/mnist/train.csv',
      epochs: 10,
      batch_size: 32,
      learning_rate: 0.001,
      optimizer: { Adam: { beta1: 0.9, beta2: 0.999, weight_decay: null } },
      loss_function: 'cross_entropy',
      compute_backend: 'metal',
      data_format: 'csv',
      validation_split: 0.2,
      test_split: 0.1,
      shuffle: true,
      seed: 42,
      checkpoint_interval: null,
      early_stopping: null,
      lr_scheduler: 'Constant' as LrSchedulerConfig,
      target_columns: ['label'],
      feature_columns: [],
      custom_params: {},
    },
    metrics: mockMetricsTimeline,
    params: {
      optimizer: 'adam',
      weight_decay: 0.0001,
      scheduler: 'cosine',
    },
    tags: ['mnist', 'baseline'],
    artifacts: [
      { artifact_type: 'checkpoint', path: '/models/exp-001/final.bin', size_bytes: 438144, created_at: new Date().toISOString(), version: null, description: null, checksum: null, metadata: null },
    ],
    model_id: null,
    dataset_id: null,
    dataset_version: null,
    environment: null,
    final_metrics: { train_loss: 0.05, val_loss: 0.08, accuracy: 0.97 },
    created_at: new Date(Date.now() - 86400000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString(),
    completed_at: new Date(Date.now() - 3600000).toISOString(),
    group: null,
    error_message: null,
  },
  'exp-002': {
    id: 'exp-002',
    name: 'CIFAR-10 ResNet',
    description: 'CIFAR-10图像分类实验，使用ResNet18架构',
    status: 'running',
    task_type: 'classification',
    config: {
      session_name: 'CIFAR-10 ResNet',
      task_type: 'classification',
      engine_id: 'burn',
      model_id: 'resnet18',
      data_source_id: 'csv',
      data_path: '/data/cifar10/train.csv',
      epochs: 50,
      batch_size: 64,
      learning_rate: 0.01,
      optimizer: { Sgd: { momentum: 0.9, weight_decay: 0.0005 } },
      loss_function: 'cross_entropy',
      compute_backend: 'metal',
      data_format: 'csv',
      validation_split: 0.1,
      test_split: 0.1,
      shuffle: true,
      seed: 42,
      checkpoint_interval: 5,
      early_stopping: { patience: 10, min_delta: 0.001, metric: 'loss', mode: 'min' },
      lr_scheduler: 'Constant' as LrSchedulerConfig,
      target_columns: ['label'],
      feature_columns: [],
      custom_params: {},
    },
    metrics: {
      series: {
        train_loss: { name: 'train_loss', values: generateLossCurve(25, 2.3, 0.3) },
        val_loss: { name: 'val_loss', values: generateLossCurve(25, 2.5, 0.5, 0.05) },
      },
    },
    params: {
      optimizer: 'sgd',
      momentum: 0.9,
      weight_decay: 0.0005,
    },
    tags: ['cifar10', 'resnet'],
    artifacts: [],
    model_id: null,
    dataset_id: null,
    dataset_version: null,
    environment: null,
    final_metrics: null,
    created_at: new Date(Date.now() - 7200000).toISOString(),
    updated_at: new Date().toISOString(),
    completed_at: null,
    group: null,
    error_message: null,
  },
};

export class MockClient implements LabClient {
  async getState(): Promise<LabStateSnapshot> {
    return {
      active_experiments: 2,
      registered_engines: 1,
      registered_tasks: 1,
      registered_models: 1,
      registered_data_sources: 1,
    };
  }

  async getDashboardStats(): Promise<DashboardStats> {
    return {
      total_experiments: 5,
      running_experiments: 2,
      completed_experiments: 2,
      failed_experiments: 1,
      total_models: 3,
      production_models: 1,
      status_counts: { running: 2, completed: 2, failed: 1 },
      task_type_counts: { classification: 3, regression: 2 },
    };
  }

  async getResourceSnapshot(): Promise<ResourceSnapshot> {
    return {
      cpu_usage_percent: 35.2,
      memory_total_mb: 16384,
      memory_available_mb: 8192,
      memory_usage_percent: 50.0,
      disk_total_gb: 512,
      disk_available_gb: 256,
      disk_usage_percent: 50.0,
      gpu_usage_percent: null,
      gpu_memory_used_mb: null,
      gpu_memory_total_mb: null,
      timestamp: Date.now(),
    };
  }

  async listEngines(): Promise<PluginInfo[]> {
    return mockPlugins.engine;
  }

  async listTasks(): Promise<PluginInfo[]> {
    return mockPlugins.task;
  }

  async listModels(): Promise<PluginInfo[]> {
    return mockPlugins.model;
  }

  async listDataSources(): Promise<PluginInfo[]> {
    return mockPlugins.data_source;
  }

  async getHardwareInfo(): Promise<HardwareInfo> {
    return mockHardware;
  }

  async getRecommendations(hardware: HardwareInfo, taskType: string, dataSize: number): Promise<TrainingRecommendation> {
    return {
      recommended_batch_size: 32,
      recommended_epochs: 10,
      recommended_backend: 'metal',
      recommended_learning_rate: 0.001,
      estimated_training_time_minutes: 5,
      can_train_locally: true,
      should_use_remote: false,
      warnings: [],
    };
  }

  async loadData(config: DataLoadConfig): Promise<DatasetInfo> {
    return {
      name: config.path.split('/').pop() || 'dataset',
      format: config.format,
      rows: 1000,
      columns: 10,
      column_names: ['col1', 'col2', 'col3', 'col4', 'col5', 'col6', 'col7', 'col8', 'col9', 'col10'],
      column_types: ['number', 'number', 'number', 'number', 'number', 'number', 'number', 'number', 'number', 'number'],
      has_missing_values: false,
      memory_size_mb: 4.2,
    };
  }

  async previewData(config: DataLoadConfig, rows: number): Promise<DataPreview> {
    const columns = ['col1', 'col2', 'col3'];
    const sampleRows = Array.from({ length: Math.min(rows, 5) }, () => [Math.random(), Math.random(), Math.random()]);
    return {
      columns,
      column_types: ['float', 'float', 'float'],
      rows: sampleRows,
      total_rows: 1000,
      offset: 0,
    };
  }

  async getModelArch(modelId: string): Promise<ModelArchDef> {
    return {
      id: modelId,
      arch_type: 'mlp',
      layers: [
        { layer_type: 'Linear', name: 'input', input_shape: { dims: [784] }, output_shape: { dims: [128] }, params: 100480, config: {} },
        { layer_type: 'ReLU', name: 'relu1', input_shape: { dims: [128] }, output_shape: { dims: [128] }, params: 0, config: {} },
        { layer_type: 'Linear', name: 'hidden', input_shape: { dims: [128] }, output_shape: { dims: [64] }, params: 8256, config: {} },
        { layer_type: 'ReLU', name: 'relu2', input_shape: { dims: [64] }, output_shape: { dims: [64] }, params: 0, config: {} },
        { layer_type: 'Linear', name: 'output', input_shape: { dims: [64] }, output_shape: { dims: [10] }, params: 650, config: {} },
      ],
      total_params: 109386,
      input_shape: { dims: [784] },
      output_shape: { dims: [10] },
    };
  }

  async createExperiment(name: string, taskType: string, config: TrainingConfig): Promise<string> {
    console.log('[MockClient] createExperiment:', name, taskType);
    return 'exp-mock-' + Date.now();
  }

  async listExperiments(): Promise<ExperimentSummary[]> {
    return mockExperiments;
  }

  async getExperimentDetail(experimentId: string): Promise<ExperimentDetail> {
    const detail = mockExperimentDetails[experimentId];
    if (detail) return detail;
    return {
      id: experimentId,
      name: 'Unknown Experiment',
      description: null,
      status: 'created',
      task_type: 'custom',
      config: {
        session_name: 'Unknown',
        task_type: 'custom',
        engine_id: 'burn',
        model_id: 'mlp',
        data_source_id: 'csv',
        data_path: '',
        epochs: 10,
        batch_size: 32,
        learning_rate: 0.001,
        optimizer: { Adam: { beta1: 0.9, beta2: 0.999, weight_decay: null } },
        loss_function: 'cross_entropy',
        compute_backend: 'cpu',
        data_format: 'csv',
        validation_split: 0.2,
        test_split: 0.1,
        shuffle: true,
        seed: null,
        checkpoint_interval: null,
        early_stopping: null,
        lr_scheduler: 'Constant' as LrSchedulerConfig,
        target_columns: [],
        feature_columns: [],
        custom_params: {},
      },
      metrics: { series: {} },
      params: {},
      tags: [],
      artifacts: [],
      model_id: null,
      dataset_id: null,
      dataset_version: null,
      environment: null,
      final_metrics: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      completed_at: null,
      group: null,
      error_message: null,
    };
  }

  async queryMetrics(experimentId: string, metricNames: string[]): Promise<MetricsTimeline> {
    const detail = mockExperimentDetails[experimentId];
    if (detail) {
      const filtered: Record<string, { name: string; values: { step: number; value: number; timestamp: string; epoch: number | null }[] }> = {};
      for (const name of metricNames) {
        if (detail.metrics.series[name]) {
          filtered[name] = detail.metrics.series[name];
        }
      }
      return { series: filtered };
    }
    return mockMetricsTimeline;
  }

  async queryMetricsDownsampled(experimentId: string, metricNames: string[], _maxPoints?: number, _smoothAlpha?: number): Promise<MetricsTimeline> {
    return this.queryMetrics(experimentId, metricNames);
  }

  async loadLogs(_experimentId: string, _limit?: number): Promise<LogEntry[]> {
    return [];
  }

  async trackMetric(experimentId: string, metricName: string, value: number, step: number): Promise<void> {
    console.log('[MockClient] trackMetric:', experimentId, metricName, value, step);
  }

  async registerModel(name: string, version: string, framework: string): Promise<string> {
    console.log('[MockClient] registerModel:', name, version, framework);
    return 'model-mock-' + Date.now();
  }

  async startTraining(name: string, taskType: string, config: TrainingConfig): Promise<string> {
    const id = await this.createExperiment(name, taskType, config);
    console.log('[MockClient] startTraining:', id);
    return id;
  }

  async stopTraining(experimentId: string): Promise<void> {
    console.log('[MockClient] stopTraining:', experimentId);
  }

  async pauseTraining(experimentId: string): Promise<void> {
    console.log('[MockClient] pauseTraining:', experimentId);
  }

  async resumeTraining(experimentId: string): Promise<void> {
    console.log('[MockClient] resumeTraining:', experimentId);
  }

  async resumeFromCheckpoint(experimentId: string, checkpointEpoch: number): Promise<void> {
    console.log('[MockClient] resumeFromCheckpoint:', experimentId, checkpointEpoch);
  }

  async runInference(experimentId: string, input_data: number[][]): Promise<InferenceResult> {
    console.log('[MockClient] runInference:', experimentId, input_data);
    return {
      predictions: [0.8],
      predicted_classes: [1],
      probabilities: [[0.2, 0.8]],
    };
  }

  async preprocessData(dataPath: string, dataFormat: string, steps: PipelineStep[]): Promise<DataPreview> {
    console.log('[MockClient] preprocessData:', dataPath, dataFormat, steps);
    return {
      columns: ['col1', 'col2'],
      column_types: ['float', 'float'],
      rows: [[0.5, 0.3]],
      total_rows: 1,
      offset: 0,
    };
  }

  async experimentAddTag(experimentId: string, tag: string): Promise<void> {
    console.log('[MockClient] experimentAddTag:', experimentId, tag);
  }

  async experimentSetParam(experimentId: string, key: string, value: unknown): Promise<void> {
    console.log('[MockClient] experimentSetParam:', experimentId, key, value);
  }

  async listArtifacts(experimentId: string): Promise<ArtifactRef[]> {
    console.log('[MockClient] listArtifacts:', experimentId);
    return [];
  }

  async getArtifactContent(experimentId: string, artifactPath: string): Promise<number[]> {
    console.log('[MockClient] getArtifactContent:', experimentId, artifactPath);
    return [];
  }

  async scanArtifacts(experimentId: string): Promise<ArtifactRef[]> {
    console.log('[MockClient] scanArtifacts:', experimentId);
    return [];
  }

  async openArtifactDir(experimentId: string): Promise<void> {
    console.log('[MockClient] openArtifactDir:', experimentId);
  }

  async selectFile(filters?: FileFilter[]): Promise<string | null> {
    console.log('[MockClient] selectFile:', filters);
    return '/mock/path/to/file.csv';
  }

  async selectDirectory(): Promise<string | null> {
    console.log('[MockClient] selectDirectory');
    return '/mock/path/to/directory';
  }

  async listModelRegistrations(statusFilter?: string): Promise<ModelRegistration[]> {
    console.log('[MockClient] listModelRegistrations:', statusFilter);
    return [
      {
        id: 'model-001',
        name: 'MLP Classifier',
        version: '1.0.0',
        status: 'production',
        framework: 'burn',
        path: '/models/mlp_v1',
        signature: null,
        structured_signature: null,
        lineage: null,
        metadata: { accuracy: 0.95 },
        description: 'MNIST分类器 - 多层感知机',
        tags: ['mnist', 'production', 'classifier'],
        aliases: ['champion'],
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
      {
        id: 'model-002',
        name: 'CNN ImageNet',
        version: '0.3.0',
        status: 'staging',
        framework: 'burn',
        path: '/models/cnn_v03',
        signature: null,
        structured_signature: null,
        lineage: null,
        metadata: { accuracy: 0.89 },
        description: 'CIFAR-10 CNN模型',
        tags: ['cifar10', 'staging'],
        aliases: [],
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
    ];
  }

  async getModelRegistration(modelId: string): Promise<ModelRegistration> {
    console.log('[MockClient] getModelRegistration:', modelId);
    return {
      id: modelId,
      name: 'Mock Model',
      version: '1.0.0',
      status: 'none',
      framework: 'burn',
      path: null,
      signature: null,
      structured_signature: null,
      lineage: null,
      metadata: {},
      description: null,
      tags: [],
      aliases: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  }

  async promoteModelStaging(modelId: string): Promise<void> {
    console.log('[MockClient] promoteModelStaging:', modelId);
  }

  async promoteModelProduction(modelId: string): Promise<void> {
    console.log('[MockClient] promoteModelProduction:', modelId);
  }

  async archiveModel(modelId: string): Promise<void> {
    console.log('[MockClient] archiveModel:', modelId);
  }

  async demoteModelStaging(modelId: string): Promise<void> {
    console.log('[MockClient] demoteModelStaging:', modelId);
  }

  async addModelAlias(modelId: string, alias: string): Promise<void> {
    console.log('[MockClient] addModelAlias:', modelId, alias);
  }

  async removeModelAlias(modelId: string, alias: string): Promise<void> {
    console.log('[MockClient] removeModelAlias:', modelId, alias);
  }

  async deleteModelRegistration(modelId: string): Promise<void> {
    console.log('[MockClient] deleteModelRegistration:', modelId);
  }

  async setModelPath(modelId: string, path: string): Promise<void> {
    console.log('[MockClient] setModelPath:', modelId, path);
  }

  async registerModelFromExperiment(experimentId: string, name: string, version: string): Promise<void> {
    console.log('[MockClient] registerModelFromExperiment:', experimentId, name, version);
  }

  async addModelVersion(modelId: string, path: string, description?: string): Promise<void> {
    console.log('[MockClient] addModelVersion:', modelId, path, description);
  }

  async listModelVersions(modelId: string): Promise<ModelVersion[]> {
    console.log('[MockClient] listModelVersions:', modelId);
    return [
      { version: '1.0.0', path: '/models/v1', description: '初始版本', created_at: new Date().toISOString(), size_bytes: 438144 },
    ];
  }

  async setModelDescription(modelId: string, description: string): Promise<void> {
    console.log('[MockClient] setModelDescription:', modelId, description);
  }

  async addModelTag(modelId: string, tag: string): Promise<void> {
    console.log('[MockClient] addModelTag:', modelId, tag);
  }

  async removeModelTag(modelId: string, tag: string): Promise<void> {
    console.log('[MockClient] removeModelTag:', modelId, tag);
  }

  async experimentSetDescription(experimentId: string, description: string): Promise<void> {
    console.log('[MockClient] experimentSetDescription:', experimentId, description);
  }

  async listCheckpoints(experimentId: string): Promise<CheckpointInfo[]> {
    console.log('[MockClient] listCheckpoints:', experimentId);
    return [
      { name: 'checkpoint-epoch-10', epoch: 10, path: '/artifacts/checkpoint-10', checkpoint_file: '/artifacts/checkpoint-10/model.bin', size_bytes: 438144, modified_timestamp: Date.now() / 1000 },
    ];
  }

  async deleteCheckpoint(experimentId: string, checkpointName: string): Promise<void> {
    console.log('[MockClient] deleteCheckpoint:', experimentId, checkpointName);
  }

  async evaluateModel(experimentId: string, testDataPath: string): Promise<EvaluationResult> {
    console.log('[MockClient] evaluateModel:', experimentId, testDataPath);
    return {
      task_type: 'classification',
      total_samples: 100,
      accuracy: 0.95,
      confusion_matrix: [[45, 5], [3, 47]],
      class_metrics: [
        { class: 0, precision: 0.9375, recall: 0.9, f1_score: 0.9184, support: 50 },
        { class: 1, precision: 0.9038, recall: 0.94, f1_score: 0.9216, support: 50 },
      ],
    };
  }

  async deleteExperiment(experimentId: string): Promise<void> {
    console.log('[MockClient] deleteExperiment:', experimentId);
  }

  async archiveExperiment(experimentId: string): Promise<void> {
    console.log('[MockClient] archiveExperiment:', experimentId);
  }

  async restoreExperiment(experimentId: string): Promise<void> {
    console.log('[MockClient] restoreExperiment:', experimentId);
  }

  async setExperimentGroup(experimentId: string, group: string): Promise<void> {
    console.log('[MockClient] setExperimentGroup:', experimentId, group);
  }

  async listExperimentGroups(): Promise<string[]> {
    console.log('[MockClient] listExperimentGroups');
    return ['nlp-experiments', 'vision-experiments'];
  }

  async batchDeleteExperiments(experimentIds: string[]): Promise<number> {
    console.log('[MockClient] batchDeleteExperiments:', experimentIds);
    return experimentIds.length;
  }

  async saveEvaluation(experimentId: string, evaluationResult: any, testDataPath: string): Promise<string> {
    console.log('[MockClient] saveEvaluation:', experimentId, testDataPath);
    return `/eval/${experimentId}/eval_${Date.now()}.json`;
  }

  async listEvaluations(experimentId: string): Promise<any[]> {
    console.log('[MockClient] listEvaluations:', experimentId);
    return [];
  }

  async cloneExperiment(experimentId: string, newName: string): Promise<string> {
    console.log('[MockClient] cloneExperiment:', experimentId, newName);
    return `exp_cloned_${Date.now()}`;
  }

  async registerDataset(name: string, format: string, path: string): Promise<DatasetRegistration> {
    console.log('[MockClient] registerDataset:', name, format, path);
    return {
      id: `ds_${Date.now()}`,
      name,
      version: 'v1',
      status: 'active',
      format: format as any,
      path,
      digest: 'abc123',
      rows: 1000,
      columns: 10,
      column_profiles: [],
      memory_size_mb: 5.2,
      tags: [],
      description: null,
      source_type: null,
      source_uri: null,
      experiment_ids: [],
      metadata: {},
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
  }

  async listDatasets(statusFilter?: string, formatFilter?: string, nameContains?: string): Promise<DatasetSummary[]> {
    console.log('[MockClient] listDatasets:', statusFilter, formatFilter, nameContains);
    return [];
  }

  async getDataset(datasetId: string): Promise<DatasetRegistration> {
    console.log('[MockClient] getDataset:', datasetId);
    throw new Error('Not found');
  }

  async deleteDataset(datasetId: string): Promise<void> {
    console.log('[MockClient] deleteDataset:', datasetId);
  }

  async archiveDataset(datasetId: string): Promise<void> {
    console.log('[MockClient] archiveDataset:', datasetId);
  }

  async restoreDataset(datasetId: string): Promise<void> {
    console.log('[MockClient] restoreDataset:', datasetId);
  }

  async datasetAddTag(datasetId: string, tag: string): Promise<void> {
    console.log('[MockClient] datasetAddTag:', datasetId, tag);
  }

  async datasetRemoveTag(datasetId: string, tag: string): Promise<void> {
    console.log('[MockClient] datasetRemoveTag:', datasetId, tag);
  }

  async datasetSetDescription(datasetId: string, description: string): Promise<void> {
    console.log('[MockClient] datasetSetDescription:', datasetId, description);
  }

  async datasetLinkExperiment(datasetId: string, experimentId: string): Promise<void> {
    console.log('[MockClient] datasetLinkExperiment:', datasetId, experimentId);
  }

  async datasetNewVersion(datasetId: string): Promise<DatasetRegistration> {
    console.log('[MockClient] datasetNewVersion:', datasetId);
    throw new Error('Not found');
  }

  async datasetVersionHistory(datasetId: string): Promise<DatasetVersionRecord[]> {
    console.log('[MockClient] datasetVersionHistory:', datasetId);
    return [];
  }

  async datasetNewVersionWithNote(datasetId: string, changeNote: string): Promise<DatasetRegistration> {
    console.log('[MockClient] datasetNewVersionWithNote:', datasetId, changeNote);
    throw new Error('Not found');
  }

  async listConnectors(): Promise<import('./types').ConnectorInfo[]> {
    console.log('[MockClient] listConnectors');
    return [
      { id: 'local_fs', name: 'Local Filesystem', description: 'Scan and discover datasets from local filesystem directories', connector_type: 'local', supported_formats: ['csv', 'json', 'parquet', 'image', 'text', 'binary'], requires_auth: false },
      { id: 'http', name: 'HTTP/HTTPS', description: 'Connect to remote datasets via HTTP/HTTPS URLs', connector_type: 'http', supported_formats: ['csv', 'json', 'parquet', 'image', 'text', 'binary'], requires_auth: false },
    ];
  }

  async scanDataSources(uri: string, options?: { recursive?: boolean; max_depth?: number; extensions?: string[]; max_results?: number }): Promise<import('./types').DiscoveredItem[]> {
    console.log('[MockClient] scanDataSources:', uri, options);
    return [];
  }

  async testDataConnection(uri: string): Promise<boolean> {
    console.log('[MockClient] testDataConnection:', uri);
    return true;
  }

  async resolveDataItem(item: import('./types').DiscoveredItem): Promise<import('./types').ResolvedDataSource> {
    console.log('[MockClient] resolveDataItem:', item.name);
    return {
      name: item.name,
      format: item.format,
      path: item.path,
      source_type: item.connector_type,
      source_uri: item.path,
      size_bytes: item.size_bytes,
      load_config_params: {},
    };
  }

  async quickRegisterDataset(item: import('./types').DiscoveredItem): Promise<DatasetRegistration> {
    console.log('[MockClient] quickRegisterDataset:', item.name);
    throw new Error('Not implemented in mock');
  }

  async batchInference(experimentId: string, inputData: number[][]): Promise<InferenceResult> {
    console.log('[MockClient] batchInference:', experimentId, inputData.length);
    return {
      predictions: inputData.map(() => Math.random()),
      predicted_classes: inputData.map(() => Math.floor(Math.random() * 10)),
      probabilities: inputData.map(() => Array.from({ length: 10 }, () => Math.random())),
    };
  }

  async getSettings(): Promise<AppSettings> {
    return {
      general: { language: 'zh-CN', theme: 'dark', log_level: 'info', auto_refresh_interval: 5000 },
      training: { default_compute_backend: 'metal', default_engine: 'burn', max_concurrent_experiments: 2, auto_checkpoint: true, checkpoint_interval: 5 },
      storage: { data_directory: './data', model_directory: './models', checkpoint_directory: './checkpoints', max_storage_gb: 50 },
    };
  }

  async saveSettings(settings: AppSettings): Promise<void> {
    console.log('[MockClient] saveSettings:', settings);
  }

  async startHyperparameterTuning(tuneConfig: TuneConfig): Promise<TuneResult> {
    console.log('[MockClient] startHyperparameterTuning:', tuneConfig);
    return { tune_id: 'mock-tune', strategy: 'Grid', trials: [], best_trial: null, best_params: null };
  }

  async generateHparamCombinations(hparamSpace: HparamSpace, strategy: TuneStrategy): Promise<Record<string, HparamValue>[]> {
    console.log('[MockClient] generateHparamCombinations:', hparamSpace, strategy);
    return [];
  }

  async exportModel(experimentId: string, format: string, outputPath: string | null, opsetVersion: number | null, inputShapes: number[][]): Promise<ExportResult> {
    console.log('[MockClient] exportModel:', experimentId, format);
    return { success: true, output_path: outputPath || '/tmp/model.onnx', format: 'Onnx' as ExportFormat, file_size_bytes: 1024, message: 'ok' };
  }

  async exportDataset(datasetId: string, targetFormat: DataExportFormat, outputPath: string | null): Promise<DataExportResult> {
    console.log('[MockClient] exportDataset:', datasetId, targetFormat);
    return {
      success: true,
      source_format: 'csv',
      target_format: targetFormat,
      output_path: outputPath || `/tmp/dataset_${datasetId}.${targetFormat}`,
      file_size_bytes: 2048000,
      rows_exported: 10000,
      message: `成功导出 10000 行数据为 ${targetFormat.toUpperCase()} 格式`,
    };
  }

  async listExportFormats(engineId: string): Promise<string[]> {
    console.log('[MockClient] listExportFormats:', engineId);
    return ['onnx', 'torchscript'];
  }

  async modelDeploy(modelId: string): Promise<void> {
    console.log('[MockClient] modelDeploy:', modelId);
  }

  async modelUndeploy(modelId: string): Promise<void> {
    console.log('[MockClient] modelUndeploy:', modelId);
  }

  async modelPredict(modelId: string, inputs: number[][]): Promise<ServeResponse> {
    console.log('[MockClient] modelPredict:', modelId, inputs.length);
    return {
      model_id: modelId,
      model_name: 'mock-model',
      model_version: 'v1',
      predictions: inputs.map(() => Math.random()),
      predicted_classes: inputs.map(() => 0),
      probabilities: inputs.map(() => [0.5, 0.5]),
      latency_ms: 1.0,
    };
  }

  async modelListEndpoints(): Promise<ServeEndpoint[]> {
    console.log('[MockClient] modelListEndpoints');
    return [];
  }

  async modelServeStats(): Promise<ServeStats> {
    console.log('[MockClient] modelServeStats');
    return { total_endpoints: 0, active_endpoints: 0, total_requests: 0, avg_latency_ms: 0 };
  }

  async onLabEvent(handler: (event: LabEvent) => void): Promise<() => void> {
    console.log('[MockClient] onLabEvent registered');
    return () => {};
  }

  async onSessionEvent(sessionId: SessionId, handler: (event: LabEvent) => void): Promise<() => void> {
    console.log('[MockClient] onSessionEvent registered:', sessionId);
    return () => {};
  }

  async trainingPlanCreate(planJson: string): Promise<string> {
    console.log('[MockClient] trainingPlanCreate');
    return JSON.stringify({ created: true, plan_id: 'mock-plan-1' });
  }

  async trainingPlanValidate(planJson: string): Promise<string> {
    console.log('[MockClient] trainingPlanValidate');
    return JSON.stringify({ is_valid: true, errors: [], warnings: [] });
  }

  async trainingPlanSummarize(planJson: string): Promise<string> {
    console.log('[MockClient] trainingPlanSummarize');
    return JSON.stringify({
      total_estimated_tokens: 1000000000,
      total_estimated_samples: 5000000,
      total_estimated_cost_usd: 5000,
      datasets_count: 3,
    });
  }

  async trainingPlanPresets(presetType: string): Promise<string> {
    console.log('[MockClient] trainingPlanPresets:', presetType);
    return JSON.stringify({ name: presetType, version: '1.0' });
  }

  async trainingPlanSave(planJson: string): Promise<string> {
    console.log('[MockClient] trainingPlanSave');
    return 'mock-plan-saved-1';
  }

  async trainingPlanList(): Promise<any[]> {
    console.log('[MockClient] trainingPlanList');
    return [
      {
        id: 'mock-plan-1',
        name: 'LLaMA-3 Pretraining',
        version: '1.0',
        description: 'Standard LLM pretraining plan',
        plan_type: 'Pretraining',
        phases_count: 3,
        datasets_count: 5,
        total_estimated_tokens: 2000000000000,
        total_estimated_steps: 500000,
        estimated_gpu_hours: 1200.5,
        modified_at: '2026-05-01T10:00:00+00:00',
      },
      {
        id: 'mock-plan-2',
        name: 'SFT Fine-tuning',
        version: '1.0',
        description: 'Supervised fine-tuning plan',
        plan_type: 'SFT',
        phases_count: 1,
        datasets_count: 2,
        total_estimated_tokens: 50000000000,
        total_estimated_steps: 10000,
        estimated_gpu_hours: 48.0,
        modified_at: '2026-04-28T15:30:00+00:00',
      },
    ];
  }

  async trainingPlanLoad(planId: string): Promise<string> {
    console.log('[MockClient] trainingPlanLoad:', planId);
    return JSON.stringify({ name: planId, version: '1.0', plan_type: 'Pretraining', phases: [] });
  }

  async trainingPlanDelete(planId: string): Promise<void> {
    console.log('[MockClient] trainingPlanDelete:', planId);
  }

  async dataRecipeCreate(recipeJson: string): Promise<string> {
    console.log('[MockClient] dataRecipeCreate');
    return JSON.stringify({ created: true, recipe_id: 'mock-recipe-1' });
  }

  async dataRecipeValidate(recipeJson: string): Promise<string> {
    console.log('[MockClient] dataRecipeValidate');
    return JSON.stringify({ valid: true });
  }

  async dataRecipeExecute(recipeJson: string, numSamples?: number): Promise<string> {
    console.log('[MockClient] dataRecipeExecute:', numSamples);
    return JSON.stringify({ executed: true, samples: numSamples || 1000 });
  }

  async dataRecipePresets(presetType: string): Promise<string> {
    console.log('[MockClient] dataRecipePresets:', presetType);
    return JSON.stringify({ name: presetType, version: '1.0', datasets: [] });
  }

  async lineageGraph(): Promise<any> {
    console.log('[MockClient] lineageGraph');
    return {
      nodes: [
        { id: 'ds-1', name: 'MNIST', node_type: 'dataset', version: 'v1', digest: null, created_at: new Date().toISOString(), metadata: {} },
        { id: 'exp-1', name: 'MNIST-CNN-v1', node_type: 'experiment', version: 'v1', digest: null, created_at: new Date().toISOString(), metadata: {} },
        { id: 'model-1', name: 'CNN-MNIST', node_type: 'model', version: 'v1', digest: null, created_at: new Date().toISOString(), metadata: {} },
      ],
      edges: [
        { from: 'exp-1', to: 'ds-1', relation: 'trained_on', transform: 'training', params: null, created_at: new Date().toISOString() },
        { from: 'model-1', to: 'exp-1', relation: 'derived_from', transform: 'checkpoint', params: null, created_at: new Date().toISOString() },
      ],
      metadata: { total_nodes: 3, total_edges: 2, max_depth: 2, has_cycles: false, root_nodes: ['ds-1'], leaf_nodes: ['model-1'] },
    };
  }

  async lineageTrace(graph: any, nodeId: string): Promise<any> {
    console.log('[MockClient] lineageTrace:', nodeId);
    return { target_id: nodeId, target_name: 'Node', upstream: [], downstream: [], full_path: [nodeId], depth: 0 };
  }

  async lineageImpact(graph: any, nodeId: string): Promise<any> {
    console.log('[MockClient] lineageImpact:', nodeId);
    return { changed_node: nodeId, changed_node_name: 'Node', directly_affected: [], indirectly_affected: [], total_affected: 0, severity: 'low', recommendations: [] };
  }

  async lineageToMermaid(graph: any): Promise<string> {
    console.log('[MockClient] lineageToMermaid');
    return 'graph LR\n    ds-1["📊 MNIST<br/>v1"]\n    exp-1["🔬 MNIST-CNN-v1<br/>v1"]\n    model-1["🧠 CNN-MNIST<br/>v1"]\n    exp-1 -->|"trains on"| ds-1\n    model-1 -->|"derives"| exp-1';
  }

  async datasetLineage(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetLineage:', datasetId);
    return { dataset_id: datasetId, dataset_name: 'MNIST', dataset_version: 'v1', experiments: [], models: [], experiment_count: 0, model_count: 0 };
  }

  async modelLineage(modelId: string): Promise<any> {
    console.log('[MockClient] modelLineage:', modelId);
    return { model_id: modelId, model_name: 'CNN-MNIST', version: 'v1', experiment: null, datasets: [] };
  }

  async datasetQualityScore(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetQualityScore:', datasetId);
    return {
      dataset_id: datasetId,
      dataset_name: 'MNIST',
      overall_score: 82.0,
      grade: 'good',
      dimensions: [
        {
          name: 'completeness',
          label: '完整性',
          score: 95.0,
          weight: 0.25,
          details: '数据缺失率评估',
          sub_metrics: [
            { name: 'column_completeness', label: '列完整性', score: 98.0, description: '所有列均无大量缺失' },
            { name: 'row_completeness', label: '行完整性', score: 92.0, description: '5% 的行存在部分字段缺失' },
          ],
          improvement: null,
        },
        {
          name: 'consistency',
          label: '一致性',
          score: 88.0,
          weight: 0.20,
          details: '数据类型与格式一致性',
          sub_metrics: [
            { name: 'type_consistency', label: '类型一致性', score: 95.0, description: '数据类型完全一致' },
            { name: 'format_consistency', label: '格式一致性', score: 81.0, description: '部分日期格式不统一' },
          ],
          improvement: '建议统一日期格式为 ISO 8601 标准',
        },
        {
          name: 'distribution',
          label: '分布质量',
          score: 72.0,
          weight: 0.20,
          details: '数据分布均衡性与偏度',
          sub_metrics: [
            { name: 'class_balance', label: '类别均衡', score: 68.0, description: '类别分布存在轻微不均衡' },
            { name: 'feature_distribution', label: '特征分布', score: 76.0, description: '数值特征分布基本正常' },
            { name: 'outlier_ratio', label: '异常值比例', score: 72.0, description: '约3%的数据为潜在异常值' },
          ],
          improvement: '建议对少数类进行过采样或使用类别权重，并检查异常值',
        },
        {
          name: 'label_quality',
          label: '标签质量',
          score: 78.0,
          weight: 0.20,
          details: '标签准确性与一致性',
          sub_metrics: [
            { name: 'label_accuracy', label: '标签准确性', score: 82.0, description: '约1.2%的标签疑似错误' },
            { name: 'label_consistency', label: '标签一致性', score: 74.0, description: '相似样本标签存在不一致' },
          ],
          improvement: '建议使用 Confident Learning 方法清洗标签，预计可提升模型精度2-5%',
        },
        {
          name: 'information',
          label: '信息密度',
          score: 75.0,
          weight: 0.15,
          details: '数据信息量与冗余度',
          sub_metrics: [
            { name: 'feature_redundancy', label: '特征冗余', score: 70.0, description: '部分特征高度相关(>0.7)' },
            { name: 'entropy', label: '信息熵', score: 80.0, description: '整体信息熵较高' },
          ],
          improvement: '建议移除高度相关的冗余特征，可减少训练时间并避免过拟合',
        },
      ],
      issues: [
        { severity: 'warning', dimension: 'distribution', description: '类别 "class_c" 样本数仅为其他类的60%', suggestion: '考虑使用 SMOTE 过采样或调整类别权重' },
        { severity: 'info', dimension: 'label_quality', description: '检测到120个疑似错误标签', suggestion: '使用 Confident Learning 自动识别并修正' },
        { severity: 'warning', dimension: 'information', description: '特征 income 和 education 相关系数达0.71', suggestion: '考虑移除其中一个以避免多重共线性' },
      ],
      recommendations: [
        '🔧 修复标签质量：运行标签清洗流程，预计可提升模型精度2-5%',
        '📊 处理类别不均衡：对 class_c 进行过采样，或使用 focal loss',
        '🔍 移除冗余特征：income 与 education 高度相关，建议保留其一',
        '📅 统一日期格式：将所有日期字段转为 ISO 8601 格式',
        '🧹 清理异常值：3%的潜在异常值可能影响模型训练稳定性',
      ],
      scored_at: new Date().toISOString(),
    };
  }

  async datasetVersionDiff(datasetId: string, fromVersion: string, toVersion: string): Promise<any> {
    console.log('[MockClient] datasetVersionDiff:', datasetId, fromVersion, toVersion);
    return {
      from_version: fromVersion,
      to_version: toVersion,
      rows_added: 5000,
      rows_removed: 0,
      columns_added: ['new_feature'],
      columns_removed: [],
      columns_type_changed: [],
      schema_compatible: true,
    };
  }

  async datasetRecommendForPlan(planJson: string): Promise<any> {
    console.log('[MockClient] datasetRecommendForPlan:', planJson.substring(0, 100));
    return {
      plan_name: 'standard_llm_pretraining',
      plan_type: 'Pretraining',
      recommendations: [
        {
          dataset_id: '1',
          dataset_name: 'web_corpus_pretrain',
          version: 'v1',
          score: 92.0,
          reasons: ['数据集类型与训练计划 预训练 高度匹配', '数据规模 10000000 行满足训练预算需求', '数据质量评分 85 分，质量优秀'],
          suitability: 'excellent',
          match_details: { type_match: 90.0, size_match: 95.0, quality_match: 85.0, feature_match: 80.0 },
        },
        {
          dataset_id: '2',
          dataset_name: 'instruction_sft_data',
          version: 'v1',
          score: 55.0,
          reasons: ['数据集类型与训练计划部分匹配', '数据规模基本满足需求', '数据质量评分 90 分，质量优秀'],
          suitability: 'fair',
          match_details: { type_match: 50.0, size_match: 40.0, quality_match: 90.0, feature_match: 60.0 },
        },
      ],
      total_datasets_available: 5,
      generated_at: new Date().toISOString(),
    };
  }

  async datasetReadinessScore(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetReadinessScore:', datasetId);
    return {
      dataset_id: datasetId,
      overall_score: 78,
      readiness_level: 'ready',
      dimensions: {
        completeness: { score: 85, status: 'pass', message: '数据完整，无缺失列' },
        consistency: { score: 72, status: 'warn', message: '部分列存在格式不一致' },
        balance: { score: 80, status: 'pass', message: '类别分布基本均衡' },
        sufficiency: { score: 75, status: 'pass', message: '数据量满足基本训练需求' },
        quality: { score: 78, status: 'pass', message: '数据质量良好' },
      },
      issues: [
        { severity: 'warning', dimension: 'consistency', message: '列 "age" 包含异常值', suggestion: '建议检查并清理异常值' },
      ],
      recommendations: ['数据基本就绪，建议修复 consistency 警告后开始训练'],
    };
  }

  async validateDataset(datasetId: string): Promise<any> {
    console.log('[MockClient] validateDataset:', datasetId);
    return {
      dataset_id: datasetId,
      is_valid: true,
      errors: [],
      warnings: [
        { field: 'column:age', message: '检测到可能的异常值', severity: 'warning' },
      ],
      schema_check: { valid: true, issues: [] },
      type_check: { valid: true, issues: [] },
      range_check: { valid: false, issues: [{ column: 'age', message: '值域超出预期范围 [0, 120]' }] },
    };
  }

  async validateDatasetIntegrity(datasetId: string): Promise<any> {
    console.log('[MockClient] validateDatasetIntegrity:', datasetId);
    return {
      dataset_id: datasetId,
      integrity_ok: true,
      checksum_match: true,
      row_count_consistent: true,
      schema_consistent: true,
      issues: [],
    };
  }

  async createDatasetSplit(datasetId: string, splitName: string, config: { train_ratio: number; val_ratio: number; test_ratio: number; shuffle: boolean; seed: number }): Promise<any> {
    console.log('[MockClient] createDatasetSplit:', datasetId, splitName, config);
    const total = 10000;
    return {
      split_id: `split_${Date.now()}`,
      dataset_id: datasetId,
      name: splitName,
      config,
      splits: {
        train: { rows: Math.round(total * config.train_ratio), path: `/data/splits/${splitName}/train` },
        val: { rows: Math.round(total * config.val_ratio), path: `/data/splits/${splitName}/val` },
        test: { rows: Math.round(total * config.test_ratio), path: `/data/splits/${splitName}/test` },
      },
      created_at: new Date().toISOString(),
    };
  }

  async listDatasetSplits(datasetId: string): Promise<any[]> {
    console.log('[MockClient] listDatasetSplits:', datasetId);
    return [
      {
        split_id: 'split_1',
        dataset_id: datasetId,
        name: 'default_split',
        config: { train_ratio: 0.7, val_ratio: 0.15, test_ratio: 0.15, shuffle: true, seed: 42 },
        splits: {
          train: { rows: 7000, path: '/data/splits/default_split/train' },
          val: { rows: 1500, path: '/data/splits/default_split/val' },
          test: { rows: 1500, path: '/data/splits/default_split/test' },
        },
        created_at: new Date().toISOString(),
      },
    ];
  }

  async datasetDedup(datasetId: string, config?: { similarity_threshold?: number; num_perm?: number; n_gram?: number }): Promise<any> {
    console.log('[MockClient] datasetDedup:', datasetId, config);
    return {
      dataset_id: datasetId,
      original_rows: 10000,
      duplicates_found: 234,
      duplicates_removed: 234,
      remaining_rows: 9766,
      dedup_ratio: 0.0234,
      clusters: 120,
      config: config || { similarity_threshold: 0.8, num_perm: 128, n_gram: 5 },
    };
  }

  async datasetCheckLeakage(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetCheckLeakage:', datasetId);
    return {
      dataset_id: datasetId,
      leakage_detected: false,
      risk_level: 'low',
      checks: {
        target_leakage: { detected: false, score: 0.05, message: '未检测到目标泄露' },
        feature_leakage: { detected: false, score: 0.02, message: '未检测到特征泄露' },
        split_leakage: { detected: false, score: 0.0, message: '数据划分无重叠' },
      },
      recommendations: [],
    };
  }

  async datasetCheckSufficiency(datasetId: string, planJson?: string): Promise<any> {
    console.log('[MockClient] datasetCheckSufficiency:', datasetId);
    return {
      dataset_id: datasetId,
      is_sufficient: true,
      current_rows: 10000,
      estimated_required: 8000,
      margin: 0.25,
      per_class: { class_a: { count: 5000, sufficient: true }, class_b: { count: 5000, sufficient: true } },
      recommendations: ['数据量充足，可以开始训练'],
    };
  }

  async datasetCheckFeatureLeakage(datasetId: string, targetColumn: string): Promise<any> {
    console.log('[MockClient] datasetCheckFeatureLeakage:', datasetId, targetColumn);
    return {
      dataset_id: datasetId,
      target_column: targetColumn,
      leakage_detected: false,
      suspicious_features: [],
      correlation_scores: { feature_a: 0.12, feature_b: 0.08, feature_c: 0.15 },
      risk_level: 'low',
    };
  }

  async datasetCheckSplitConsistency(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetCheckSplitConsistency:', datasetId);
    return {
      dataset_id: datasetId,
      consistent: true,
      issues: [],
      label_distribution_match: 0.98,
      feature_distribution_match: 0.95,
    };
  }

  async datasetAnalyzeImbalance(datasetId: string, labelColumn: string): Promise<any> {
    console.log('[MockClient] datasetAnalyzeImbalance:', datasetId, labelColumn);
    return {
      dataset_id: datasetId,
      label_column: labelColumn,
      total_samples: 10000,
      class_counts: { class_a: 4000, class_b: 3500, class_c: 2500 },
      imbalance_ratio: 1.6,
      is_imbalanced: false,
      entropy: 1.05,
      recommendations: ['数据分布基本均衡，无需特殊处理'],
    };
  }

  async datasetAnalyzeDrift(datasetId: string, referenceDatasetId: string): Promise<any> {
    console.log('[MockClient] datasetAnalyzeDrift:', datasetId, referenceDatasetId);
    return {
      dataset_id: datasetId,
      reference_dataset_id: referenceDatasetId,
      drift_detected: false,
      drift_score: 0.08,
      feature_drifts: [
        { feature: 'age', drift_score: 0.05, drift_type: 'none' },
        { feature: 'income', drift_score: 0.12, drift_type: 'mild' },
      ],
      overall_severity: 'low',
      recommendations: [],
    };
  }

  async datasetAnalyzeCorrelation(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetAnalyzeCorrelation:', datasetId);
    return {
      dataset_id: datasetId,
      correlation_matrix: {
        age_income: 0.65, age_education: 0.42, income_education: 0.71,
      },
      highly_correlated_pairs: [
        { feature_a: 'income', feature_b: 'education', correlation: 0.71 },
      ],
      recommendations: ['income 和 education 高度相关，考虑移除其中一个以避免多重共线性'],
    };
  }

  async datasetLabelQuality(datasetId: string, labelColumn: string): Promise<any> {
    console.log('[MockClient] datasetLabelQuality:', datasetId, labelColumn);
    return {
      dataset_id: datasetId,
      label_column: labelColumn,
      total_labels: 10000,
      unique_labels: 5,
      suspected_mislabels: 120,
      mislabel_rate: 0.012,
      label_consistency: 0.988,
      per_class_quality: {
        class_a: { count: 2000, suspected_mislabels: 15, consistency: 0.992 },
        class_b: { count: 2000, suspected_mislabels: 25, consistency: 0.988 },
        class_c: { count: 2000, suspected_mislabels: 30, consistency: 0.985 },
        class_d: { count: 2000, suspected_mislabels: 20, consistency: 0.990 },
        class_e: { count: 2000, suspected_mislabels: 30, consistency: 0.985 },
      },
    };
  }

  async datasetConfidentLearning(datasetId: string, labelColumn: string): Promise<any> {
    console.log('[MockClient] datasetConfidentLearning:', datasetId, labelColumn);
    return {
      dataset_id: datasetId,
      label_column: labelColumn,
      noise_matrix: { class_a: [0.98, 0.01, 0.01], class_b: [0.02, 0.95, 0.03] },
      confident_joint: [[1950, 20, 30], [40, 1900, 60], [10, 30, 1960]],
      identified_label_issues: 150,
      suggested_corrections: [
        { index: 42, original_label: 'class_a', suggested_label: 'class_b', confidence: 0.89 },
        { index: 187, original_label: 'class_c', suggested_label: 'class_a', confidence: 0.92 },
      ],
    };
  }

  async datasetLabelQualitySummary(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetLabelQualitySummary:', datasetId);
    return {
      dataset_id: datasetId,
      overall_quality: 'good',
      total_label_columns: 2,
      summaries: [
        { column: 'category', quality: 0.988, mislabel_rate: 0.012 },
        { column: 'sentiment', quality: 0.965, mislabel_rate: 0.035 },
      ],
    };
  }

  async datasetSliceAnalysis(datasetId: string, config: { slice_by: string; conditions: Record<string, any> }): Promise<any> {
    console.log('[MockClient] datasetSliceAnalysis:', datasetId, config);
    return {
      dataset_id: datasetId,
      total_rows: 10000,
      slices: [
        { slice_name: 'age<30', row_count: 3500, row_ratio: 0.35, avg_quality: 0.92, label_distribution: { a: 0.4, b: 0.35, c: 0.25 } },
        { slice_name: 'age>=30', row_count: 6500, row_ratio: 0.65, avg_quality: 0.88, label_distribution: { a: 0.38, b: 0.37, c: 0.25 } },
      ],
      cross_slice_comparison: { max_quality_gap: 0.04, worst_slice: 'age>=30' },
      recommendations: ['切片间质量差异较小，模型性能应较为均衡'],
    };
  }

  async datasetBiasDetection(datasetId: string, config: { sensitive_column: string; label_column: string }): Promise<any> {
    console.log('[MockClient] datasetBiasDetection:', datasetId, config);
    return {
      dataset_id: datasetId,
      sensitive_column: config.sensitive_column,
      label_column: config.label_column,
      bias_detected: false,
      demographic_parity: 0.92,
      equalized_odds: 0.89,
      predictive_parity: 0.91,
      group_metrics: [
        { group: 'male', positive_rate: 0.35, tpr: 0.82, fpr: 0.12 },
        { group: 'female', positive_rate: 0.33, tpr: 0.80, fpr: 0.11 },
      ],
      recommendations: ['未检测到显著偏差，各群体指标基本均衡'],
    };
  }

  async datasetInfluenceTracin(datasetId: string, experimentId: string): Promise<any> {
    console.log('[MockClient] datasetInfluenceTracin:', datasetId, experimentId);
    return {
      dataset_id: datasetId,
      experiment_id: experimentId,
      method: 'tracin',
      total_samples: 10000,
      most_helpful: [
        { index: 42, influence_score: 0.89, label: 'class_a' },
        { index: 187, influence_score: 0.85, label: 'class_b' },
      ],
      most_harmful: [
        { index: 312, influence_score: -0.72, label: 'class_c' },
        { index: 891, influence_score: -0.65, label: 'class_a' },
      ],
      influence_distribution: { mean: 0.02, std: 0.15, median: 0.01 },
    };
  }

  async datasetInfluenceLoo(datasetId: string, experimentId: string): Promise<any> {
    console.log('[MockClient] datasetInfluenceLoo:', datasetId, experimentId);
    return {
      dataset_id: datasetId,
      experiment_id: experimentId,
      method: 'loo',
      total_samples: 10000,
      most_helpful: [{ index: 42, score_change: 0.015 }, { index: 187, score_change: 0.012 }],
      most_harmful: [{ index: 312, score_change: -0.018 }, { index: 891, score_change: -0.011 }],
    };
  }

  async datasetInfluenceLossDiff(datasetId: string, experimentId: string): Promise<any> {
    console.log('[MockClient] datasetInfluenceLossDiff:', datasetId, experimentId);
    return {
      dataset_id: datasetId,
      experiment_id: experimentId,
      method: 'loss_diff',
      total_samples: 10000,
      most_helpful: [{ index: 42, loss_diff: -0.023 }, { index: 187, loss_diff: -0.019 }],
      most_harmful: [{ index: 312, loss_diff: 0.031 }, { index: 891, loss_diff: 0.022 }],
    };
  }

  async datasetSetCard(datasetId: string, card: any): Promise<any> {
    console.log('[MockClient] datasetSetCard:', datasetId);
    return { dataset_id: datasetId, card, updated: true };
  }

  async datasetGetCard(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetGetCard:', datasetId);
    return {
      dataset_id: datasetId,
      name: '示例数据集',
      description: '这是一个示例数据集卡片',
      homepage: 'https://example.com/dataset',
      license: 'MIT',
      citation: '@inproceedings{example2024}',
      task_categories: ['text-classification', 'sentiment-analysis'],
      languages: ['zh', 'en'],
      size_categories: ['10K<n<100K'],
      creation_date: '2024-01-15',
      update_date: '2024-06-01',
      usage_notes: '适用于文本分类和情感分析任务',
      quality_score: 85,
      known_issues: ['部分样本标签可能有误'],
    };
  }

  async datasetDiscoverySearch(query: string, filters?: Record<string, any>): Promise<any> {
    console.log('[MockClient] datasetDiscoverySearch:', query, filters);
    return {
      query,
      total_results: 3,
      results: [
        { id: '1', name: 'web_corpus_pretrain', format: 'jsonl', rows: 10000000, relevance: 0.95 },
        { id: '2', name: 'instruction_sft_data', format: 'json', rows: 50000, relevance: 0.82 },
        { id: '3', name: 'code_dataset', format: 'csv', rows: 200000, relevance: 0.75 },
      ],
    };
  }

  async datasetUsageStats(datasetId: string): Promise<any> {
    console.log('[MockClient] datasetUsageStats:', datasetId);
    return {
      dataset_id: datasetId,
      total_experiments: 12,
      active_experiments: 3,
      completed_experiments: 8,
      failed_experiments: 1,
      avg_accuracy: 0.87,
      last_used: new Date().toISOString(),
      usage_timeline: [
        { date: '2024-01', experiments: 2 },
        { date: '2024-02', experiments: 3 },
        { date: '2024-03', experiments: 4 },
        { date: '2024-04', experiments: 3 },
      ],
    };
  }

  async datasetMultimodalImages(datasetId: string, offset?: number, limit?: number): Promise<any> {
    console.log('[MockClient] datasetMultimodalImages:', datasetId, offset, limit);
    return { dataset_id: datasetId, images: [], total: 0, offset: offset || 0 };
  }

  async datasetMultimodalTexts(datasetId: string, offset?: number, limit?: number): Promise<any> {
    console.log('[MockClient] datasetMultimodalTexts:', datasetId, offset, limit);
    return { dataset_id: datasetId, texts: [{ id: 1, content: '示例文本', metadata: {} }], total: 1, offset: offset || 0 };
  }

  async datasetCreateKfold(datasetId: string, k: number, shuffle: boolean, seed: number): Promise<any> {
    console.log('[MockClient] datasetCreateKfold:', datasetId, k, shuffle, seed);
    return {
      dataset_id: datasetId,
      k,
      folds: Array.from({ length: k }, (_, i) => ({
        fold_id: i,
        train_indices: Array.from({ length: 8000 }, (_, j) => j + i * 200),
        val_indices: Array.from({ length: 2000 }, (_, j) => j + i * 200),
      })),
    };
  }

  async datasetRowDiff(datasetId: string, fromVersion: string, toVersion: string, offset?: number, limit?: number): Promise<any> {
    console.log('[MockClient] datasetRowDiff:', datasetId, fromVersion, toVersion);
    return {
      dataset_id: datasetId,
      from_version: fromVersion,
      to_version: toVersion,
      total_changes: 15,
      changes: [
        { row_index: 3, change_type: 'Modified', column: 'age', old_value: '25', new_value: '26' },
        { row_index: 7, change_type: 'Added', column: null, old_value: null, new_value: '{"name":"new_user"}' },
      ],
    };
  }

  async datasetListAugmentationPresets(format: string): Promise<any> {
    console.log('[MockClient] datasetListAugmentationPresets:', format);
    return {
      format,
      presets: [
        { name: 'text_augmentation', description: '文本数据增强', operations: ['synonym_replacement', 'random_insertion', 'random_deletion'] },
        { name: 'image_augmentation', description: '图像数据增强', operations: ['rotate', 'flip', 'crop', 'color_jitter'] },
      ],
    };
  }

  async datasetLazyInspect(path: string, format: string): Promise<any> {
    console.log('[MockClient] datasetLazyInspect:', path, format);
    return { path, format, rows: 100000, columns: 15, size_mb: 250.5, schema: [{ name: 'id', type: 'integer' }, { name: 'text', type: 'string' }] };
  }

  async datasetLazyReadChunk(path: string, format: string, offset: number, limit: number): Promise<any> {
    console.log('[MockClient] datasetLazyReadChunk:', path, format, offset, limit);
    return { path, format, offset, limit, rows: [['1', 'sample text']], columns: ['id', 'text'] };
  }

  async datasetRecommendChunkSize(path: string, format: string): Promise<any> {
    console.log('[MockClient] datasetRecommendChunkSize:', path, format);
    return { path, format, recommended_chunk_size: 10000, estimated_memory_mb: 50 };
  }

  async datasetCuration(datasetId: string, config: any): Promise<any> {
    console.log('[MockClient] datasetCuration:', datasetId, config);
    return {
      dataset_id: datasetId,
      original_rows: 10000,
      curated_rows: 9500,
      removed_rows: 500,
      steps_applied: ['remove_duplicates', 'filter_low_quality', 'mask_pii'],
      curation_report: { duplicates_removed: 200, low_quality_removed: 250, pii_masked: 50 },
    };
  }

  async curationConfig(): Promise<any> {
    console.log('[MockClient] curationConfig');
    return {
      available_steps: [
        { id: 'remove_duplicates', name: '去重', description: '移除重复数据' },
        { id: 'filter_low_quality', name: '低质量过滤', description: '过滤低质量数据' },
        { id: 'mask_pii', name: 'PII脱敏', description: '对个人身份信息进行脱敏' },
        { id: 'normalize_text', name: '文本规范化', description: '统一文本格式' },
      ],
    };
  }

  async curationMaskPii(text: string): Promise<any> {
    console.log('[MockClient] curationMaskPii:', text.substring(0, 50));
    return { original: text, masked: text.replace(/\d{11}/g, '[PHONE]').replace(/\d{18}/g, '[ID_CARD]'), pii_found: 2 };
  }

  async datasetPreview(datasetId: string, offset?: number, limit?: number): Promise<any> {
    console.log('[MockClient] datasetPreview:', datasetId, offset, limit);
    return { dataset_id: datasetId, rows: [['1', 'sample']], columns: ['id', 'text'], offset: offset || 0, total_rows: 10000 };
  }

  async datasetSample(datasetId: string, n: number, seed?: number): Promise<any> {
    console.log('[MockClient] datasetSample:', datasetId, n, seed);
    return { dataset_id: datasetId, sample_size: n, rows: Array.from({ length: Math.min(n, 5) }, (_, i) => [String(i), `sample_${i}`]), columns: ['id', 'text'] };
  }

  async datasetColumnStats(datasetId: string, columnName: string): Promise<any> {
    console.log('[MockClient] datasetColumnStats:', datasetId, columnName);
    return {
      dataset_id: datasetId, column: columnName,
      mean: 45.2, std: 12.3, min: 18, max: 85,
      median: 43, q1: 32, q3: 56,
      null_count: 5, unique_count: 67,
    };
  }

  async datasetReadSplit(datasetId: string, splitName: string, offset?: number, limit?: number): Promise<any> {
    console.log('[MockClient] datasetReadSplit:', datasetId, splitName, offset, limit);
    return { dataset_id: datasetId, split_name: splitName, rows: [['1', 'train_sample']], columns: ['id', 'text'], offset: offset || 0, total_rows: 7000 };
  }

  async streamingOpenCsv(path: string, config?: any): Promise<any> {
    console.log('[MockClient] streamingOpenCsv:', path);
    return { path, handle: 'csv_handle_1', total_rows: 100000, columns: ['id', 'name', 'value'] };
  }

  async streamingOpenJsonl(path: string, config?: any): Promise<any> {
    console.log('[MockClient] streamingOpenJsonl:', path);
    return { path, handle: 'jsonl_handle_1', total_rows: 50000, columns: ['id', 'text', 'label'] };
  }

  async streamingRecommendChunk(path: string, format: string): Promise<any> {
    console.log('[MockClient] streamingRecommendChunk:', path, format);
    return { path, format, recommended_chunk: 5000, estimated_memory_mb: 25 };
  }

  async dataCollate(datasetIds: string[], strategy: string): Promise<any> {
    console.log('[MockClient] dataCollate:', datasetIds, strategy);
    return { collated_id: 'collated_1', source_datasets: datasetIds, strategy, total_rows: datasetIds.length * 10000 };
  }

  async dataLoaderCreate(config: any): Promise<any> {
    console.log('[MockClient] dataLoaderCreate:', config);
    return { loader_id: 'loader_1', config, batch_size: config.batch_size || 32, num_workers: config.num_workers || 4 };
  }

  async dataVersionInit(path: string): Promise<any> {
    console.log('[MockClient] dataVersionInit:', path);
    return { path, initialized: true, current_version: 'v0', branch: 'main' };
  }

  async dataVersionCommit(path: string, message: string): Promise<any> {
    console.log('[MockClient] dataVersionCommit:', path, message);
    return { path, version: 'v1', message, hash: 'abc123', timestamp: new Date().toISOString() };
  }

  async dataVersionLog(path: string): Promise<any> {
    console.log('[MockClient] dataVersionLog:', path);
    return {
      path,
      commits: [
        { version: 'v3', message: '添加新数据', hash: 'def456', timestamp: '2024-06-01T10:00:00Z' },
        { version: 'v2', message: '清洗数据', hash: 'bcd345', timestamp: '2024-05-15T08:00:00Z' },
        { version: 'v1', message: '初始提交', hash: 'abc123', timestamp: '2024-01-15T12:00:00Z' },
      ],
    };
  }

  async dataVersionCheckout(path: string, version: string): Promise<any> {
    console.log('[MockClient] dataVersionCheckout:', path, version);
    return { path, checkout_version: version, previous_version: 'v3', success: true };
  }

  async dataVersionDiff(path: string, fromVersion: string, toVersion: string): Promise<any> {
    console.log('[MockClient] dataVersionDiff:', path, fromVersion, toVersion);
    return { path, from: fromVersion, to: toVersion, files_added: ['new_file.csv'], files_removed: [], files_modified: ['data.csv'], row_change: 500 };
  }

  async dataVersionBranches(path: string): Promise<any> {
    console.log('[MockClient] dataVersionBranches:', path);
    return {
      path,
      current_branch: 'main',
      branches: [
        { name: 'main', head: 'v3', created_at: '2024-01-15T12:00:00Z' },
        { name: 'experiment-a', head: 'v2', created_at: '2024-04-01T09:00:00Z' },
      ],
    };
  }

  async dataVersionCreateBranch(path: string, branchName: string): Promise<any> {
    console.log('[MockClient] dataVersionCreateBranch:', path, branchName);
    return { path, branch: branchName, head: 'v3', created_at: new Date().toISOString() };
  }
}
