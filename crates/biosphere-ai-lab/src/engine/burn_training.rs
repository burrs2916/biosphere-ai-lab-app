use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

use burn::{
    backend::{Autodiff, NdArray, Wgpu},
    data::{
        dataloader::{batcher::Batcher, DataLoaderBuilder},
        dataset::Dataset,
    },
    module::Module,
    nn::{
        conv::{Conv2d, Conv2dConfig},
        loss::{CrossEntropyLossConfig, MseLoss, Reduction},
        pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig, MaxPool2d, MaxPool2dConfig},
        BatchNorm, BatchNormConfig,
        Linear, LinearConfig,
        PaddingConfig2d,
        Relu,
    },
    optim::AdamConfig,
    prelude::*,
    record::Recorder,
    tensor::{backend::AutodiffBackend, Int, Tensor},
    train::{
        ClassificationOutput, InferenceStep, Learner,
        MetricEarlyStoppingStrategy, RegressionOutput, StoppingCondition, SupervisedTraining, TrainOutput, TrainStep,
        metric::{AccuracyMetric, LossMetric, NumericEntry},
        renderer::{
            EvaluationName, EvaluationProgress, MetricState, MetricsRenderer,
            MetricsRendererEvaluation, MetricsRendererTraining, TrainingProgress,
        },
        LearningResult, LearnerSummary,
    },
};

use crate::core::config::{LrSchedulerConfig, OptimizerConfig, TrainingConfig};
use crate::core::event::{EventBus, LabEvent};
use crate::types::{ComputeBackend, SessionId, TaskType};

pub struct TrainControl {
    pub paused: AtomicBool,
    pub cancelled: AtomicBool,
}

impl TrainControl {
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn wait_while_paused(&self) {
        while self.is_paused() && !self.is_cancelled() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

type WgpuBackend = Autodiff<Wgpu>;
type CpuBackend = Autodiff<NdArray>;

macro_rules! dispatch_lr_scheduler {
    ($config:expr, $lr:expr, $epochs:expr, $body:expr) => {
        match &$config.lr_scheduler {
            LrSchedulerConfig::Constant => {
                let lr_scheduler = burn::optim::lr_scheduler::constant::ConstantLr::new($lr);
                $body(lr_scheduler)
            }
            LrSchedulerConfig::Step { step_size, gamma } => {
                let lr_scheduler = burn::optim::lr_scheduler::step::StepLrSchedulerConfig::new($lr, *step_size)
                    .with_gamma(*gamma)
                    .init()
                    .map_err(|e| crate::core::LabError::Custom(format!("StepLr scheduler error: {}", e)))?;
                $body(lr_scheduler)
            }
            LrSchedulerConfig::Exponential { gamma } => {
                let clamped_lr = $lr.min(1.0).max(1e-8);
                let lr_scheduler = burn::optim::lr_scheduler::exponential::ExponentialLrSchedulerConfig::new(clamped_lr, *gamma)
                    .init()
                    .map_err(|e| crate::core::LabError::Custom(format!("ExponentialLr scheduler error: {}", e)))?;
                $body(lr_scheduler)
            }
            LrSchedulerConfig::CosineAnnealing { min_lr, num_iters } => {
                let clamped_lr = $lr.min(1.0).max(1e-8);
                let clamped_min = min_lr.min(clamped_lr).max(0.0);
                let lr_scheduler = burn::optim::lr_scheduler::cosine::CosineAnnealingLrSchedulerConfig::new(clamped_lr, *num_iters)
                    .with_min_lr(clamped_min)
                    .init()
                    .map_err(|e| crate::core::LabError::Custom(format!("CosineAnnealing scheduler error: {}", e)))?;
                $body(lr_scheduler)
            }
            LrSchedulerConfig::Linear { final_lr, num_iters } => {
                let clamped_lr = $lr.min(1.0).max(1e-8);
                let clamped_final = final_lr.min(1.0).max(0.0);
                let lr_scheduler = burn::optim::lr_scheduler::linear::LinearLrSchedulerConfig::new(clamped_lr, clamped_final, *num_iters)
                    .init()
                    .map_err(|e| crate::core::LabError::Custom(format!("LinearLr scheduler error: {}", e)))?;
                $body(lr_scheduler)
            }
        }
    };
}

#[derive(Clone, Debug)]
pub struct TabularItem {
    pub features: Vec<f32>,
    pub label: f32,
}

pub struct CsvDataset {
    items: Vec<TabularItem>,
}

impl CsvDataset {
    pub fn from_csv(
        path: &str,
        feature_columns: &[String],
        target_column: &str,
        has_header: bool,
        delimiter: u8,
    ) -> crate::core::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::core::LabError::DataLoadFailed(format!("Cannot read file: {}", e)))?;

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(has_header)
            .from_reader(content.as_bytes());

        let headers = reader
            .headers()
            .map_err(|e| crate::core::LabError::DataLoadFailed(format!("Cannot parse CSV headers: {}", e)))?
            .clone();

        let header_vec: Vec<String> = headers.iter().map(|h| h.to_string()).collect();

        let feature_indices: Vec<usize> = if feature_columns.is_empty() {
            header_vec
                .iter()
                .enumerate()
                .filter(|(_, h)| h.as_str() != target_column)
                .map(|(i, _)| i)
                .collect()
        } else {
            feature_columns
                .iter()
                .map(|fc| {
                    header_vec
                        .iter()
                        .position(|h| h == fc)
                        .ok_or_else(|| {
                            crate::core::LabError::DataLoadFailed(format!(
                                "Feature column '{}' not found",
                                fc
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let target_index = header_vec
            .iter()
            .position(|h| h == target_column)
            .ok_or_else(|| {
                crate::core::LabError::DataLoadFailed(format!(
                    "Target column '{}' not found",
                    target_column
                ))
            })?;

        let mut items = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| {
                crate::core::LabError::DataLoadFailed(format!("CSV parse error: {}", e))
            })?;

            let mut features = Vec::with_capacity(feature_indices.len());
            for &idx in &feature_indices {
                let val: f32 = record
                    .get(idx)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0);
                features.push(val);
            }

            let label: f32 = record
                .get(target_index)
                .unwrap_or("0")
                .trim()
                .parse()
                .unwrap_or(0.0);

            items.push(TabularItem { features, label });
        }

        if items.is_empty() {
            return Err(crate::core::LabError::DataLoadFailed(
                "No data rows found in CSV".to_string(),
            ));
        }

        Ok(Self { items })
    }

    pub fn from_mnist() -> crate::core::Result<Self> {
        use burn::data::dataset::vision::MnistDataset;

        let dataset = MnistDataset::train();
        let mut items = Vec::new();
        for i in 0..dataset.len() {
            if let Some(item) = dataset.get(i) {
                let mut features = Vec::with_capacity(28 * 28);
                for row in item.image.iter() {
                    for &val in row.iter() {
                        features.push(val / 255.0);
                    }
                }
                items.push(TabularItem {
                    features,
                    label: item.label as f32,
                });
            }
        }

        Ok(Self { items })
    }

    pub fn split(&self, train_ratio: f64) -> (Self, Self) {
        let total = self.items.len();
        let train_end = ((total as f64) * train_ratio) as usize;

        let mut indices: Vec<usize> = (0..total).collect();
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        total.hash(&mut hasher);
        42u64.hash(&mut hasher);
        let seed = hasher.finish();
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(seed);
        use rand::Rng;
        for i in (1..indices.len()).rev() {
            let j = rng.gen_range(0..=i);
            indices.swap(i, j);
        }

        let mut train_items = Vec::with_capacity(train_end);
        let mut val_items = Vec::with_capacity(total - train_end);
        for (i, &idx) in indices.iter().enumerate() {
            if i < train_end {
                train_items.push(self.items[idx].clone());
            } else {
                val_items.push(self.items[idx].clone());
            }
        }

        (Self { items: train_items }, Self { items: val_items })
    }

    pub fn num_features(&self) -> usize {
        if self.items.is_empty() {
            0
        } else {
            self.items[0].features.len()
        }
    }

    pub fn num_classes(&self) -> usize {
        let unique_labels: std::collections::HashSet<usize> = self
            .items
            .iter()
            .map(|item| item.label as usize)
            .collect();
        let max_label = unique_labels.iter().copied().max().unwrap_or(0);
        if unique_labels.len() != max_label + 1 || !unique_labels.contains(&0) {
            crate::infrastructure::log("DATA", &format!(
                "Warning: Labels are not contiguous 0..{}. Unique labels: {:?}. Output layer size will be {}.",
                max_label, unique_labels, max_label + 1
            ), None);
        }
        max_label + 1
    }

    pub fn is_classification(&self) -> bool {
        self.items
            .iter()
            .all(|item| item.label == item.label.floor() && item.label >= 0.0)
    }

    pub fn split_by_indices(&self, train_indices: &[usize], val_indices: &[usize]) -> (Self, Self) {
        let train_items: Vec<TabularItem> = train_indices.iter()
            .filter(|&&i| i < self.items.len())
            .map(|&i| self.items[i].clone())
            .collect();
        let val_items: Vec<TabularItem> = val_indices.iter()
            .filter(|&&i| i < self.items.len())
            .map(|&i| self.items[i].clone())
            .collect();
        (Self { items: train_items }, Self { items: val_items })
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn from_json(
        path: &str,
        feature_columns: &[String],
        target_column: &str,
    ) -> crate::core::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::core::LabError::DataLoadFailed(format!("Cannot read file: {}", e)))?;

        let json_value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| crate::core::LabError::DataLoadFailed(format!("JSON parse error: {}", e)))?;

        let array = json_value
            .as_array()
            .ok_or_else(|| crate::core::LabError::DataLoadFailed("JSON root must be an array".to_string()))?;

        if array.is_empty() {
            return Err(crate::core::LabError::DataLoadFailed("JSON array is empty".to_string()));
        }

        let first_obj = array[0]
            .as_object()
            .ok_or_else(|| crate::core::LabError::DataLoadFailed("JSON items must be objects".to_string()))?;

        let header_vec: Vec<String> = first_obj.keys().cloned().collect();

        let feature_indices: Vec<usize> = if feature_columns.is_empty() {
            header_vec
                .iter()
                .enumerate()
                .filter(|(_, h)| h.as_str() != target_column)
                .map(|(i, _)| i)
                .collect()
        } else {
            feature_columns
                .iter()
                .map(|fc| {
                    header_vec
                        .iter()
                        .position(|h| h == fc)
                        .ok_or_else(|| {
                            crate::core::LabError::DataLoadFailed(format!(
                                "Feature column '{}' not found",
                                fc
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let target_index = header_vec
            .iter()
            .position(|h| h == target_column)
            .ok_or_else(|| {
                crate::core::LabError::DataLoadFailed(format!(
                    "Target column '{}' not found",
                    target_column
                ))
            })?;

        let mut items = Vec::new();
        for item_val in array {
            let obj = item_val.as_object().ok_or_else(|| {
                crate::core::LabError::DataLoadFailed("JSON item must be an object".to_string())
            })?;

            let mut features = Vec::with_capacity(feature_indices.len());
            for &idx in &feature_indices {
                let key = &header_vec[idx];
                let val: f32 = obj
                    .get(key)
                    .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
                    .unwrap_or(0.0) as f32;
                features.push(val);
            }

            let target_key = &header_vec[target_index];
            let label: f32 = obj
                .get(target_key)
                .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())))
                .unwrap_or(0.0) as f32;

            items.push(TabularItem { features, label });
        }

        if items.is_empty() {
            return Err(crate::core::LabError::DataLoadFailed(
                "No data rows found in JSON".to_string(),
            ));
        }

        Ok(Self { items })
    }
}

pub struct ImageDataset {
    items: Vec<ImageItem>,
}

impl ImageDataset {
    pub fn new(items: Vec<ImageItem>) -> Self {
        Self { items }
    }

    pub fn from_mnist_images() -> crate::core::Result<Self> {
        use burn::data::dataset::vision::MnistDataset;

        let dataset = MnistDataset::train();
        let mut items = Vec::new();
        for i in 0..dataset.len() {
            if let Some(item) = dataset.get(i) {
                let mut pixels = Vec::with_capacity(28 * 28);
                for row in item.image.iter() {
                    for &val in row.iter() {
                        pixels.push(val / 255.0);
                    }
                }
                items.push(ImageItem {
                    pixels,
                    channels: 1,
                    height: 28,
                    width: 28,
                    label: item.label as f32,
                });
            }
        }

        Ok(Self { items })
    }

    pub fn split(&self, train_ratio: f64) -> (Self, Self) {
        let total = self.items.len();
        let train_end = ((total as f64) * train_ratio) as usize;

        let mut indices: Vec<usize> = (0..total).collect();
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        total.hash(&mut hasher);
        42u64.hash(&mut hasher);
        let seed = hasher.finish();
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::seed_from_u64(seed);
        use rand::Rng;
        for i in (1..indices.len()).rev() {
            let j = rng.gen_range(0..=i);
            indices.swap(i, j);
        }

        let mut train_items = Vec::with_capacity(train_end);
        let mut val_items = Vec::with_capacity(total - train_end);
        for (i, &idx) in indices.iter().enumerate() {
            if i < train_end {
                train_items.push(self.items[idx].clone());
            } else {
                val_items.push(self.items[idx].clone());
            }
        }

        (Self { items: train_items }, Self { items: val_items })
    }

    pub fn num_classes(&self) -> usize {
        let max_label = self
            .items
            .iter()
            .map(|item| item.label as usize)
            .max()
            .unwrap_or(0);
        max_label + 1
    }

    pub fn split_by_indices(&self, train_indices: &[usize], val_indices: &[usize]) -> (Self, Self) {
        let train_items: Vec<ImageItem> = train_indices.iter()
            .filter(|&&i| i < self.items.len())
            .map(|&i| self.items[i].clone())
            .collect();
        let val_items: Vec<ImageItem> = val_indices.iter()
            .filter(|&&i| i < self.items.len())
            .map(|&i| self.items[i].clone())
            .collect();
        (Self { items: train_items }, Self { items: val_items })
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl Dataset<ImageItem> for ImageDataset {
    fn get(&self, index: usize) -> Option<ImageItem> {
        self.items.get(index).cloned()
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

impl Dataset<TabularItem> for CsvDataset {
    fn get(&self, index: usize) -> Option<TabularItem> {
        self.items.get(index).cloned()
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}

#[derive(Clone, Debug)]
pub struct TabularBatch<B: Backend> {
    pub features: Tensor<B, 2>,
    pub targets: Tensor<B, 1>,
    pub targets_int: Tensor<B, 1, Int>,
}

pub struct TabularBatcher<B: Backend> {
    #[allow(dead_code)]
    device: B::Device,
}

impl<B: Backend> TabularBatcher<B> {
    pub fn new(device: B::Device) -> Self {
        Self { device }
    }
}

impl<B: Backend> Batcher<B, TabularItem, TabularBatch<B>> for TabularBatcher<B> {
    fn batch(&self, items: Vec<TabularItem>, device: &B::Device) -> TabularBatch<B> {
        let features = items
            .iter()
            .map(|item| Tensor::<B, 1>::from_floats(item.features.as_slice(), device))
            .collect();
        let features = Tensor::stack::<2>(features, 0);

        let targets_float: Vec<f32> = items.iter().map(|item| item.label).collect();
        let targets = Tensor::<B, 1>::from_floats(targets_float.as_slice(), device);

        let targets_int: Vec<i64> = items.iter().map(|item| item.label as i64).collect();
        let targets_int = Tensor::<B, 1, Int>::from_data(
            burn::tensor::TensorData::from(targets_int.as_slice()),
            device,
        );

        TabularBatch {
            features,
            targets,
            targets_int,
        }
    }
}

#[derive(Module, Debug)]
pub struct DynamicMlp<B: Backend> {
    layers: Vec<Linear<B>>,
    activations: Vec<Relu>,
    output_layer: Linear<B>,
}

impl<B: Backend> DynamicMlp<B> {
    pub fn new(input_size: usize, hidden_sizes: Vec<usize>, output_size: usize, device: &B::Device) -> Self {
        let mut layers = Vec::new();
        let mut activations = Vec::new();

        let mut current_size = input_size;
        for &hidden_size in &hidden_sizes {
            layers.push(LinearConfig::new(current_size, hidden_size).init(device));
            activations.push(Relu::new());
            current_size = hidden_size;
        }

        let output_layer = LinearConfig::new(current_size, output_size).init(device);

        Self {
            layers,
            activations,
            output_layer,
        }
    }

    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let mut x = x;
        for (layer, activation) in self.layers.iter().zip(self.activations.iter()) {
            x = layer.forward(x);
            x = activation.forward(x);
        }
        self.output_layer.forward(x)
    }

    pub fn forward_classification(
        &self,
        features: Tensor<B, 2>,
        targets: Tensor<B, 1, Int>,
    ) -> ClassificationOutput<B> {
        let output = self.forward(features);
        let loss = CrossEntropyLossConfig::new()
            .init(&output.device())
            .forward(output.clone(), targets.clone());
        ClassificationOutput::new(loss, output, targets)
    }

    pub fn forward_regression(
        &self,
        features: Tensor<B, 2>,
        targets: Tensor<B, 2>,
    ) -> RegressionOutput<B> {
        let output = self.forward(features);
        let loss = MseLoss::new().forward(output.clone(), targets.clone(), Reduction::Mean);
        RegressionOutput { loss, output, targets }
    }
}

impl<B: AutodiffBackend> TrainStep for DynamicMlp<B> {
    type Input = TabularBatch<B>;
    type Output = ClassificationOutput<B>;

    fn step(&self, batch: Self::Input) -> TrainOutput<Self::Output> {
        let item = self.forward_classification(batch.features, batch.targets_int);
        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> InferenceStep for DynamicMlp<B> {
    type Input = TabularBatch<B>;
    type Output = ClassificationOutput<B>;

    fn step(&self, batch: Self::Input) -> Self::Output {
        self.forward_classification(batch.features, batch.targets_int)
    }
}

#[derive(Module, Debug)]
pub struct MlpRegressor<B: Backend> {
    mlp: DynamicMlp<B>,
}

impl<B: Backend> MlpRegressor<B> {
    pub fn new(input_size: usize, hidden_sizes: Vec<usize>, output_size: usize, device: &B::Device) -> Self {
        Self {
            mlp: DynamicMlp::new(input_size, hidden_sizes, output_size, device),
        }
    }

    pub fn forward_regression(
        &self,
        features: Tensor<B, 2>,
        targets: Tensor<B, 2>,
    ) -> RegressionOutput<B> {
        self.mlp.forward_regression(features, targets)
    }
}

impl<B: AutodiffBackend> TrainStep for MlpRegressor<B> {
    type Input = TabularBatch<B>;
    type Output = RegressionOutput<B>;

    fn step(&self, batch: Self::Input) -> TrainOutput<Self::Output> {
        let targets_2d = batch.targets.clone().unsqueeze::<2>();
        let targets_2d_t = targets_2d.transpose();
        let item = self.forward_regression(batch.features, targets_2d_t);
        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> InferenceStep for MlpRegressor<B> {
    type Input = TabularBatch<B>;
    type Output = RegressionOutput<B>;

    fn step(&self, batch: Self::Input) -> Self::Output {
        let targets_2d = batch.targets.clone().unsqueeze::<2>();
        let targets_2d_t = targets_2d.transpose();
        self.forward_regression(batch.features, targets_2d_t)
    }
}

#[derive(Clone, Debug)]
pub struct ImageItem {
    pub pixels: Vec<f32>,
    pub channels: usize,
    pub height: usize,
    pub width: usize,
    pub label: f32,
}

#[derive(Clone, Debug)]
pub struct ImageBatch<B: Backend> {
    pub images: Tensor<B, 4>,
    pub targets: Tensor<B, 1>,
    pub targets_int: Tensor<B, 1, Int>,
}

pub struct ImageBatcher<B: Backend> {
    #[allow(dead_code)]
    device: B::Device,
    #[allow(dead_code)]
    channels: usize,
    #[allow(dead_code)]
    height: usize,
    #[allow(dead_code)]
    width: usize,
}

impl<B: Backend> ImageBatcher<B> {
    pub fn new(device: B::Device, channels: usize, height: usize, width: usize) -> Self {
        Self { device, channels, height, width }
    }
}

impl<B: Backend> Batcher<B, ImageItem, ImageBatch<B>> for ImageBatcher<B> {
    fn batch(&self, items: Vec<ImageItem>, device: &B::Device) -> ImageBatch<B> {
        let images = items
            .iter()
            .map(|item| {
                Tensor::<B, 3>::from_floats(item.pixels.as_slice(), device)
                    .reshape([item.channels, item.height, item.width])
            })
            .collect();
        let images = Tensor::stack::<4>(images, 0);

        let targets_float: Vec<f32> = items.iter().map(|item| item.label).collect();
        let targets = Tensor::<B, 1>::from_floats(targets_float.as_slice(), device);

        let targets_int: Vec<i64> = items.iter().map(|item| item.label as i64).collect();
        let targets_int = Tensor::<B, 1, Int>::from_data(
            burn::tensor::TensorData::from(targets_int.as_slice()),
            device,
        );

        ImageBatch { images, targets, targets_int }
    }
}

#[derive(Module, Debug)]
pub struct DynamicCnn<B: Backend> {
    conv1: Conv2d<B>,
    bn1: BatchNorm<B>,
    conv2: Conv2d<B>,
    bn2: BatchNorm<B>,
    pool: MaxPool2d,
    adaptive_pool: AdaptiveAvgPool2d,
    fc1: Linear<B>,
    fc2: Linear<B>,
    activation: Relu,
}

impl<B: Backend> DynamicCnn<B> {
    pub fn new(channels: usize, num_classes: usize, device: &B::Device) -> Self {
        let conv1 = Conv2dConfig::new([channels, 32], [3, 3])
            .with_padding(PaddingConfig2d::Same)
            .init(device);
        let bn1 = BatchNormConfig::new(32).init(device);

        let conv2 = Conv2dConfig::new([32, 64], [3, 3])
            .with_padding(PaddingConfig2d::Same)
            .init(device);
        let bn2 = BatchNormConfig::new(64).init(device);

        let pool = MaxPool2dConfig::new([2, 2]).with_strides([2, 2]).init();
        let adaptive_pool = AdaptiveAvgPool2dConfig::new([1, 1]).init();

        let fc1 = LinearConfig::new(64, 128).init(device);
        let fc2 = LinearConfig::new(128, num_classes).init(device);

        let activation = Relu::new();

        Self { conv1, bn1, conv2, bn2, pool, adaptive_pool, fc1, fc2, activation }
    }

    pub fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 2> {
        let x = self.conv1.forward(x);
        let x = self.bn1.forward(x);
        let x = self.activation.forward(x);
        let x = self.pool.forward(x);

        let x = self.conv2.forward(x);
        let x = self.bn2.forward(x);
        let x = self.activation.forward(x);
        let x = self.pool.forward(x);

        let x = self.adaptive_pool.forward(x);

        let [batch_size, channels, _h, _w] = x.dims();
        let x = x.reshape([batch_size, channels]);

        let x = self.fc1.forward(x);
        let x = self.activation.forward(x);
        self.fc2.forward(x)
    }

    pub fn forward_classification(
        &self,
        images: Tensor<B, 4>,
        targets: Tensor<B, 1, Int>,
    ) -> ClassificationOutput<B> {
        let output = self.forward(images);
        let loss = CrossEntropyLossConfig::new()
            .init(&output.device())
            .forward(output.clone(), targets.clone());
        ClassificationOutput::new(loss, output, targets)
    }
}

impl<B: AutodiffBackend> TrainStep for DynamicCnn<B> {
    type Input = ImageBatch<B>;
    type Output = ClassificationOutput<B>;

    fn step(&self, batch: Self::Input) -> TrainOutput<Self::Output> {
        let item = self.forward_classification(batch.images, batch.targets_int);
        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> InferenceStep for DynamicCnn<B> {
    type Input = ImageBatch<B>;
    type Output = ClassificationOutput<B>;

    fn step(&self, batch: Self::Input) -> Self::Output {
        self.forward_classification(batch.images, batch.targets_int)
    }
}

#[allow(dead_code)]
enum RendererEvent {
    TrainProgress {
        epoch: usize,
        epoch_total: usize,
        iteration: usize,
        items_processed: usize,
        items_total: usize,
    },
    ValidProgress {
        epoch: usize,
        epoch_total: usize,
        iteration: usize,
    },
    TestProgress {
        epoch: usize,
        epoch_total: usize,
        iteration: usize,
    },
    TrainMetric(String, f64),
    ValidMetric(String, f64),
    TestMetric(String, f64),
    TrainEnd,
}

#[derive(Clone)]
struct EventBusRenderer {
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    events: Arc<Mutex<Vec<RendererEvent>>>,
    train_control: Arc<TrainControl>,
    last_emitted_epoch: Arc<Mutex<Option<usize>>>,
    epoch_offset: usize,
    current_epoch: usize,
    total_epochs: usize,
    batch_size: usize,
}

impl EventBusRenderer {
    pub fn new(event_bus: Arc<EventBus>, session_id: SessionId, train_control: Arc<TrainControl>, batch_size: usize) -> Self {
        Self {
            event_bus,
            session_id,
            events: Arc::new(Mutex::new(Vec::new())),
            train_control,
            last_emitted_epoch: Arc::new(Mutex::new(None)),
            epoch_offset: 0,
            current_epoch: 0,
            total_epochs: 0,
            batch_size: batch_size.max(1),
        }
    }

    pub fn with_epoch_offset(mut self, offset: usize) -> Self {
        self.epoch_offset = offset;
        self
    }

    fn extract_metric_value(numeric: &NumericEntry) -> f64 {
        numeric.current()
    }

    fn extract_metric_name(entry: &burn::train::metric::MetricEntry) -> String {
        let formatted = &entry.serialized_entry.formatted;
        formatted
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string()
    }

    fn emit_epoch_if_needed(&self, epoch: usize, epoch_total: usize) {
        let adjusted_epoch = epoch + self.epoch_offset;
        let adjusted_total = epoch_total + self.epoch_offset;
        {
            let last = self.last_emitted_epoch.lock().unwrap();
            if let Some(le) = *last {
                if le == adjusted_epoch {
                    return;
                }
            }
        }

        let events = self.events.lock().unwrap();
        let mut train_loss: Option<f64> = None;
        let mut train_acc: Option<f64> = None;
        let mut val_loss: Option<f64> = None;
        let mut val_acc: Option<f64> = None;
        let mut test_loss: Option<f64> = None;
        let mut test_acc: Option<f64> = None;

        for event in events.iter().rev() {
            match event {
                RendererEvent::TrainMetric(name, value) => {
                    if name.contains("Loss") && train_loss.is_none() {
                        train_loss = Some(*value);
                    }
                    if name.contains("Accuracy") && train_acc.is_none() {
                        train_acc = Some(*value);
                    }
                }
                RendererEvent::ValidMetric(name, value) => {
                    if name.contains("Loss") && val_loss.is_none() {
                        val_loss = Some(*value);
                    }
                    if name.contains("Accuracy") && val_acc.is_none() {
                        val_acc = Some(*value);
                    }
                }
                RendererEvent::TestMetric(name, value) => {
                    if name.contains("Loss") && test_loss.is_none() {
                        test_loss = Some(*value);
                    }
                    if name.contains("Accuracy") && test_acc.is_none() {
                        test_acc = Some(*value);
                    }
                }
                _ => {}
            }
            if train_loss.is_some()
                && train_acc.is_some()
                && val_loss.is_some()
                && val_acc.is_some()
                && test_loss.is_some()
                && test_acc.is_some()
            {
                break;
            }
        }

        let tl = train_loss.unwrap_or(0.0);
        let acc = train_acc.or(val_acc).unwrap_or(0.0);

        let mut metrics = serde_json::Map::new();
        if let Some(a) = train_acc {
            metrics.insert("train_accuracy".to_string(), serde_json::json!(a));
        }
        if let Some(a) = val_acc {
            metrics.insert("val_accuracy".to_string(), serde_json::json!(a));
        }
        if let Some(l) = train_loss {
            metrics.insert("best_loss".to_string(), serde_json::json!(l));
        }
        if let Some(tl_val) = test_loss {
            metrics.insert("test_loss".to_string(), serde_json::json!(tl_val));
        }
        if let Some(ta_val) = test_acc {
            metrics.insert("test_accuracy".to_string(), serde_json::json!(ta_val));
        }

        self.event_bus.emit(LabEvent::EpochCompleted {
            session_id: self.session_id.clone(),
            epoch: adjusted_epoch,
            total_epochs: adjusted_total,
            train_loss: tl,
            val_loss,
            metrics: serde_json::Value::Object(metrics),
        });

        let progress_msg = match val_loss {
            Some(vl) => format!(
                "Epoch {}/{} - loss: {:.4} - val_loss: {:.4} - acc: {:.1}%",
                adjusted_epoch, adjusted_total, tl, vl, acc * 100.0
            ),
            None => format!(
                "Epoch {}/{} - loss: {:.4} - acc: {:.1}%",
                adjusted_epoch, adjusted_total, tl, acc * 100.0
            ),
        };
        self.event_bus.emit(LabEvent::ProgressUpdate {
            session_id: self.session_id.clone(),
            progress: adjusted_epoch as f64 / adjusted_total as f64,
            message: progress_msg.clone(),
        });

        self.event_bus.emit(LabEvent::LogOutput {
            session_id: self.session_id.clone(),
            level: "info".to_string(),
            message: progress_msg,
        });

        {
            let mut last = self.last_emitted_epoch.lock().unwrap();
            *last = Some(adjusted_epoch);
        }
        {
            let mut events = self.events.lock().unwrap();
            events.clear();
        }
    }
}

impl MetricsRendererTraining for EventBusRenderer {
    fn update_train(&mut self, state: MetricState) {
        if self.train_control.is_cancelled() {
            return;
        }
        self.train_control.wait_while_paused();
        if let MetricState::Numeric(entry, numeric) = state {
            let name = Self::extract_metric_name(&entry);
            let value = Self::extract_metric_value(&numeric);
            self.events
                .lock()
                .unwrap()
                .push(RendererEvent::TrainMetric(name, value));
        }
    }

    fn update_valid(&mut self, state: MetricState) {
        if self.train_control.is_cancelled() {
            return;
        }
        if let MetricState::Numeric(entry, numeric) = state {
            let name = Self::extract_metric_name(&entry);
            let value = Self::extract_metric_value(&numeric);
            self.events
                .lock()
                .unwrap()
                .push(RendererEvent::ValidMetric(name, value));
        }
    }

    fn render_train(&mut self, item: TrainingProgress) {
        if self.train_control.is_cancelled() {
            return;
        }
        self.train_control.wait_while_paused();

        self.current_epoch = item.epoch + self.epoch_offset;
        self.total_epochs = item.epoch_total + self.epoch_offset;

        self.events
            .lock()
            .unwrap()
            .push(RendererEvent::TrainProgress {
                epoch: item.epoch,
                epoch_total: item.epoch_total,
                iteration: item.iteration,
                items_processed: item.progress.items_processed,
                items_total: item.progress.items_total,
            });

        let current_loss = {
            let events = self.events.lock().unwrap();
            events.iter().rev().find_map(|e| match e {
                RendererEvent::TrainMetric(name, value) if name.contains("Loss") => Some(*value),
                _ => None,
            }).unwrap_or(0.0)
        };

        self.event_bus.emit(LabEvent::BatchCompleted {
            session_id: self.session_id.clone(),
            batch: item.iteration,
            total_batches: (item.progress.items_total + self.batch_size - 1) / self.batch_size,
            loss: current_loss,
        });

        self.event_bus.emit(LabEvent::LogOutput {
            session_id: self.session_id.clone(),
            level: "info".to_string(),
            message: format!(
                "Batch {}/{} (epoch {}/{}) loss={:.4}",
                item.progress.items_processed,
                item.progress.items_total,
                item.epoch,
                item.epoch_total,
                current_loss
            ),
        });
    }

    fn render_valid(&mut self, item: TrainingProgress) {
        if self.train_control.is_cancelled() {
            return;
        }
        self.train_control.wait_while_paused();

        self.events
            .lock()
            .unwrap()
            .push(RendererEvent::ValidProgress {
                epoch: item.epoch,
                epoch_total: item.epoch_total,
                iteration: item.iteration,
            });

        self.emit_epoch_if_needed(item.epoch, item.epoch_total);
    }

    fn on_train_end(
        &mut self,
        _summary: Option<LearnerSummary>,
    ) -> Result<(), Box<dyn core::error::Error>> {
        self.events.lock().unwrap().push(RendererEvent::TrainEnd);
        Ok(())
    }
}

impl MetricsRendererEvaluation for EventBusRenderer {
    fn update_test(&mut self, _name: EvaluationName, state: MetricState) {
        if self.train_control.is_cancelled() {
            return;
        }
        if let MetricState::Numeric(entry, numeric) = state {
            let metric_name = Self::extract_metric_name(&entry);
            let value = Self::extract_metric_value(&numeric);
            let prefixed_name = format!("test_{}", metric_name);
            self.events
                .lock()
                .unwrap()
                .push(RendererEvent::TestMetric(prefixed_name.clone(), value));
            self.event_bus.emit(LabEvent::LogOutput {
                session_id: self.session_id.clone(),
                level: "debug".to_string(),
                message: format!("Test metric: {} = {:.4}", prefixed_name, value),
            });
        }
    }

    fn render_test(&mut self, item: EvaluationProgress) {
        if self.train_control.is_cancelled() {
            return;
        }
        self.train_control.wait_while_paused();

        self.events
            .lock()
            .unwrap()
            .push(RendererEvent::TestProgress {
                epoch: self.current_epoch,
                epoch_total: self.total_epochs,
                iteration: item.iteration,
            });

        let test_metrics_summary = {
            let events = self.events.lock().unwrap();
            let mut parts = Vec::new();
            for e in events.iter().rev() {
                if let RendererEvent::TestMetric(name, value) = e {
                    if name.starts_with("test_") {
                        parts.push(format!("{}={:.4}", name, value));
                        if parts.len() >= 4 {
                            break;
                        }
                    }
                }
            }
            parts
        };

        if !test_metrics_summary.is_empty() {
            self.event_bus.emit(LabEvent::LogOutput {
                session_id: self.session_id.clone(),
                level: "info".to_string(),
                message: format!(
                    "Evaluation iter {} - {}",
                    item.iteration,
                    test_metrics_summary.join(", ")
                ),
            });
        }

        self.emit_epoch_if_needed(self.current_epoch, self.total_epochs);
    }

    fn on_test_end(&mut self) -> Result<(), Box<dyn core::error::Error>> {
        let test_metrics: Vec<(String, f64)> = {
            let events = self.events.lock().unwrap();
            events.iter()
                .filter_map(|e| match e {
                    RendererEvent::TestMetric(name, value) => Some((name.clone(), *value)),
                    _ => None,
                })
                .collect()
        };

        if !test_metrics.is_empty() {
            let summary: Vec<String> = test_metrics.iter()
                .map(|(n, v)| format!("{}={:.4}", n, v))
                .collect();
            self.event_bus.emit(LabEvent::LogOutput {
                session_id: self.session_id.clone(),
                level: "info".to_string(),
                message: format!("Evaluation complete: {}", summary.join(", ")),
            });
        }

        Ok(())
    }
}

impl MetricsRenderer for EventBusRenderer {
    fn manual_close(&mut self) {}

    fn register_metric(
        &mut self,
        _definition: burn::train::metric::MetricDefinition,
    ) {
    }
}

pub fn run_training(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()> {
    match config.compute_backend {
        ComputeBackend::Wgpu | ComputeBackend::Cuda | ComputeBackend::Metal | ComputeBackend::Rocm => {
            run_training_with_backend::<WgpuBackend>(event_bus, session_id, config, artifact_dir, train_control)
        }
        ComputeBackend::Cpu => {
            run_training_with_backend::<CpuBackend>(event_bus, session_id, config, artifact_dir, train_control)
        }
    }
}

fn run_training_with_backend<B>(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()>
where
    B: AutodiffBackend,
{
    let device = B::Device::default();
    let seed = config.seed.unwrap_or(42);
    B::seed(&device, seed);

    std::fs::create_dir_all(artifact_dir).ok();

    match config.model_id.as_str() {
        "cnn" => run_cnn_training::<B>(event_bus, session_id, config, artifact_dir, device, seed, train_control),
        "mlp" | _ => run_mlp_training::<B>(event_bus, session_id, config, artifact_dir, device, seed, train_control),
    }
}

pub fn run_training_from_checkpoint(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    checkpoint_epoch: usize,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()> {
    let checkpoint_path = find_latest_checkpoint(artifact_dir)
        .ok_or_else(|| crate::core::LabError::InferenceFailed("No checkpoint found for resume".to_string()))?;

    event_bus.emit(LabEvent::LogOutput {
        session_id: session_id.clone(),
        level: "info".to_string(),
        message: format!("Resuming training from checkpoint at epoch {}", checkpoint_epoch),
    });

    let remaining_epochs = config.epochs.saturating_sub(checkpoint_epoch);
    if remaining_epochs == 0 {
        event_bus.emit(LabEvent::LogOutput {
            session_id: session_id.clone(),
            level: "warn".to_string(),
            message: "Checkpoint epoch equals or exceeds total epochs, nothing to train".to_string(),
        });
        return Ok(());
    }

    let mut resume_config = config.clone();
    resume_config.epochs = remaining_epochs;

    match config.compute_backend {
        ComputeBackend::Wgpu | ComputeBackend::Cuda | ComputeBackend::Metal | ComputeBackend::Rocm => {
            run_training_with_checkpoint::<WgpuBackend>(event_bus, session_id, &resume_config, artifact_dir, checkpoint_path, checkpoint_epoch, train_control)
        }
        ComputeBackend::Cpu => {
            run_training_with_checkpoint::<CpuBackend>(event_bus, session_id, &resume_config, artifact_dir, checkpoint_path, checkpoint_epoch, train_control)
        }
    }
}

fn run_training_with_checkpoint<B>(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    checkpoint_path: std::path::PathBuf,
    epoch_offset: usize,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()>
where
    B: AutodiffBackend,
{
    let device = B::Device::default();
    let seed = config.seed.unwrap_or(42);
    B::seed(&device, seed);

    let (dataset_train, dataset_valid) = if config.data_path.is_empty() {
        let dataset = CsvDataset::from_mnist()?;
        let train_ratio = 1.0 - config.validation_split;
        dataset.split(train_ratio)
    } else {
        let target_column = if config.target_columns.is_empty() {
            "label".to_string()
        } else {
            config.target_columns[0].clone()
        };
        let dataset = match config.data_format {
            crate::types::DataFormat::Json => {
                CsvDataset::from_json(&config.data_path, &config.feature_columns, &target_column)?
            }
            _ => {
                let delimiter = config.custom_params.get("delimiter")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.chars().next())
                    .unwrap_or(',') as u8;
                CsvDataset::from_csv(&config.data_path, &config.feature_columns, &target_column, true, delimiter)?
            }
        };
        let train_ratio = 1.0 - config.validation_split;
        if let Some(ref split_indices) = config.split_indices {
            dataset.split_by_indices(&split_indices.train_indices, &split_indices.val_indices)
        } else {
            dataset.split(train_ratio)
        }
    };
    let num_features = dataset_train.num_features();
    let is_classification = matches!(config.task_type, TaskType::Classification)
        || (!matches!(config.task_type, TaskType::Regression) && dataset_train.is_classification());
    let num_outputs = if is_classification { dataset_train.num_classes() } else { 1 };

    let hidden_sizes = if num_features <= 10 {
        vec![64, 32]
    } else if num_features <= 100 {
        vec![128, 64]
    } else {
        vec![256, 128, 64]
    };

    let batcher_train = TabularBatcher::<B>::new(device.clone());
    let batcher_valid = TabularBatcher::<B::InnerBackend>::new(device.clone());

    let mut dataloader_train_builder = DataLoaderBuilder::new(batcher_train)
        .batch_size(config.batch_size)
        .num_workers(2);
    if config.shuffle {
        dataloader_train_builder = dataloader_train_builder.shuffle(seed);
    }
    let dataloader_train = dataloader_train_builder.build(dataset_train);
    let dataloader_valid = DataLoaderBuilder::new(batcher_valid)
        .batch_size(config.batch_size)
        .num_workers(2)
        .build(dataset_valid);

    let recorder = burn::record::DefaultFileRecorder::<burn::record::FullPrecisionSettings>::new();

    if is_classification {
        let model = DynamicMlp::<B>::new(num_features, hidden_sizes.clone(), num_outputs, &device);
        let model_record = recorder.load(checkpoint_path.clone(), &device)
            .map_err(|e| crate::core::LabError::InferenceFailed(format!("Failed to load checkpoint: {:?}", e)))?;
        let model = model.load_record(model_record);
        let optim = create_adam_config(&config.optimizer).init::<B, DynamicMlp<B>>();

        dispatch_lr_scheduler!(config, config.learning_rate, config.epochs, |lr_scheduler| {
            let renderer = EventBusRenderer::new(event_bus.clone(), session_id.clone(), train_control.clone(), config.batch_size)
                .with_epoch_offset(epoch_offset);
            let mut training = SupervisedTraining::new(artifact_dir, dataloader_train.clone(), dataloader_valid.clone())
                .metrics((AccuracyMetric::<NdArray>::new(), LossMetric::<NdArray>::new()))
                .num_epochs(config.epochs)
                .renderer(renderer);

            if let Some(epoch) = config.checkpoint_interval {
                training = training.checkpoint(epoch);
            }

            let learner = Learner::new(model.clone(), optim.clone(), lr_scheduler);
            let _result: LearningResult<_> = training.launch(learner);
            Ok::<(), crate::core::LabError>(())
        })?;
    } else {
        let model = MlpRegressor::<B>::new(num_features, hidden_sizes.clone(), num_outputs, &device);
        let model_record = recorder.load(checkpoint_path, &device)
            .map_err(|e| crate::core::LabError::InferenceFailed(format!("Failed to load checkpoint: {:?}", e)))?;
        let model = model.load_record(model_record);
        let optim = create_adam_config(&config.optimizer).init::<B, MlpRegressor<B>>();

        dispatch_lr_scheduler!(config, config.learning_rate, config.epochs, |lr_scheduler| {
            let renderer = EventBusRenderer::new(event_bus.clone(), session_id.clone(), train_control.clone(), config.batch_size)
                .with_epoch_offset(epoch_offset);
            let mut training = SupervisedTraining::new(artifact_dir, dataloader_train.clone(), dataloader_valid.clone())
                .metrics((LossMetric::<NdArray>::new(),))
                .num_epochs(config.epochs)
                .renderer(renderer);

            if let Some(epoch) = config.checkpoint_interval {
                training = training.checkpoint(epoch);
            }

            let learner = Learner::new(model.clone(), optim.clone(), lr_scheduler);
            let _result: LearningResult<_> = training.launch(learner);
            Ok::<(), crate::core::LabError>(())
        })?;
    }

    Ok(())
}

fn run_mlp_training<B>(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    device: B::Device,
    seed: u64,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()>
where
    B: AutodiffBackend,
{
    let (dataset_train, dataset_valid) = if config.data_path.is_empty() {
        let dataset = CsvDataset::from_mnist()?;
        let train_ratio = 1.0 - config.validation_split;
        dataset.split(train_ratio)
    } else {
        let target_column = if config.target_columns.is_empty() {
            "label".to_string()
        } else {
            config.target_columns[0].clone()
        };

        let dataset = match config.data_format {
            crate::types::DataFormat::Json => {
                CsvDataset::from_json(
                    &config.data_path,
                    &config.feature_columns,
                    &target_column,
                )?
            }
            _ => {
                let delimiter = config
                    .custom_params
                    .get("delimiter")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.chars().next())
                    .unwrap_or(',') as u8;

                CsvDataset::from_csv(
                    &config.data_path,
                    &config.feature_columns,
                    &target_column,
                    true,
                    delimiter,
                )?
            }
        };

        let train_ratio = 1.0 - config.validation_split;
        if let Some(ref split_indices) = config.split_indices {
            dataset.split_by_indices(&split_indices.train_indices, &split_indices.val_indices)
        } else {
            dataset.split(train_ratio)
        }
    };

    let num_features = dataset_train.num_features();
    let is_classification = matches!(config.task_type, TaskType::Classification)
        || (!matches!(config.task_type, TaskType::Regression) && dataset_train.is_classification());
    let num_outputs = if is_classification {
        dataset_train.num_classes()
    } else {
        1
    };

    event_bus.emit(LabEvent::DataLoaded {
        session_id: session_id.clone(),
        rows: dataset_train.len() + dataset_valid.len(),
        columns: num_features,
    });

    let hidden_sizes = if num_features <= 10 {
        vec![64, 32]
    } else if num_features <= 100 {
        vec![128, 64]
    } else {
        vec![256, 128, 64]
    };

    let batcher_train = TabularBatcher::<B>::new(device.clone());
    let batcher_valid = TabularBatcher::<B::InnerBackend>::new(device.clone());

    let mut dataloader_train_builder = DataLoaderBuilder::new(batcher_train)
        .batch_size(config.batch_size)
        .num_workers(2);
    if config.shuffle {
        dataloader_train_builder = dataloader_train_builder.shuffle(seed);
    }
    let dataloader_train = dataloader_train_builder.build(dataset_train);

    let dataloader_valid = DataLoaderBuilder::new(batcher_valid)
        .batch_size(config.batch_size)
        .num_workers(2)
        .build(dataset_valid);

    let renderer = EventBusRenderer::new(event_bus, session_id, train_control, config.batch_size);

    if is_classification {
        let model = DynamicMlp::<B>::new(num_features, hidden_sizes.clone(), num_outputs, &device);
        let optim = create_adam_config(&config.optimizer).init::<B, DynamicMlp<B>>();

        dispatch_lr_scheduler!(config, config.learning_rate, config.epochs, |lr_scheduler| {
            let mut training = SupervisedTraining::new(artifact_dir, dataloader_train.clone(), dataloader_valid.clone())
                .metrics((AccuracyMetric::<NdArray>::new(), LossMetric::<NdArray>::new()))
                .num_epochs(config.epochs)
                .renderer(renderer.clone());

            if let Some(epoch) = config.checkpoint_interval {
                training = training.checkpoint(epoch);
            }

            if let Some(ref es_config) = config.early_stopping {
                let loss_metric = LossMetric::<NdArray>::new();
                let strategy = MetricEarlyStoppingStrategy::new(
                    &loss_metric,
                    burn::train::metric::store::Aggregate::Mean,
                    burn::train::metric::store::Direction::Lowest,
                    burn::train::metric::store::Split::Valid,
                    StoppingCondition::NoImprovementSince { n_epochs: es_config.patience },
                );
                training = training.early_stopping(strategy);
            }

            let learner = Learner::new(model.clone(), optim.clone(), lr_scheduler);
            let _result: LearningResult<_> = training.launch(learner);

            let metadata = serde_json::json!({
                "model_id": config.model_id,
                "num_features": num_features,
                "num_classes": num_outputs,
                "is_classification": is_classification,
                "hidden_sizes": hidden_sizes.clone(),
                "task_type": serde_json::to_string(&config.task_type).unwrap_or_default(),
            });
            let metadata_path = std::path::Path::new(artifact_dir).join("model_metadata.json");
            if let Err(e) = std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap_or_default()) {
                crate::infrastructure::log("BURN", &format!("Failed to write model metadata: {}", e), None);
            }

            Ok::<(), crate::core::LabError>(())
        })?;
    } else {
        let model = MlpRegressor::<B>::new(num_features, hidden_sizes.clone(), num_outputs, &device);
        let optim = create_adam_config(&config.optimizer).init::<B, MlpRegressor<B>>();

        dispatch_lr_scheduler!(config, config.learning_rate, config.epochs, |lr_scheduler| {
            let mut training = SupervisedTraining::new(artifact_dir, dataloader_train.clone(), dataloader_valid.clone())
                .metrics((LossMetric::<NdArray>::new(),))
                .num_epochs(config.epochs)
                .renderer(renderer.clone());

            if let Some(epoch) = config.checkpoint_interval {
                training = training.checkpoint(epoch);
            }

            if let Some(ref es_config) = config.early_stopping {
                let loss_metric = LossMetric::<NdArray>::new();
                let strategy = MetricEarlyStoppingStrategy::new(
                    &loss_metric,
                    burn::train::metric::store::Aggregate::Mean,
                    burn::train::metric::store::Direction::Lowest,
                    burn::train::metric::store::Split::Valid,
                    StoppingCondition::NoImprovementSince { n_epochs: es_config.patience },
                );
                training = training.early_stopping(strategy);
            }

            let learner = Learner::new(model.clone(), optim.clone(), lr_scheduler);
            let _result: LearningResult<_> = training.launch(learner);

            let metadata = serde_json::json!({
                "model_id": config.model_id,
                "num_features": num_features,
                "num_outputs": num_outputs,
                "is_classification": is_classification,
                "hidden_sizes": hidden_sizes.clone(),
                "task_type": serde_json::to_string(&config.task_type).unwrap_or_default(),
            });
            let metadata_path = std::path::Path::new(artifact_dir).join("model_metadata.json");
            if let Err(e) = std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap_or_default()) {
                crate::infrastructure::log("BURN", &format!("Failed to write model metadata: {}", e), None);
            }

            Ok::<(), crate::core::LabError>(())
        })?;
    }

    Ok(())
}

fn run_cnn_training<B>(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    device: B::Device,
    seed: u64,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()>
where
    B: AutodiffBackend,
{
    let (dataset_train, dataset_valid) = if config.data_path.is_empty() {
        let dataset = ImageDataset::from_mnist_images()?;
        let train_ratio = 1.0 - config.validation_split;
        dataset.split(train_ratio)
    } else {
        let dataset = CsvDataset::from_csv(
            &config.data_path,
            &config.feature_columns,
            &config.target_columns.first().unwrap_or(&"label".to_string()),
            true,
            config.custom_params.get("delimiter")
                .and_then(|v| v.as_str())
                .and_then(|s| s.chars().next())
                .unwrap_or(',') as u8,
        )?;

        let image_size: usize = config.custom_params.get("image_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(28) as usize;
        let channels: usize = config.custom_params.get("channels")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        let items: Vec<ImageItem> = dataset.items.iter().map(|item| {
            ImageItem {
                pixels: item.features.clone(),
                channels,
                height: image_size,
                width: image_size,
                label: item.label,
            }
        }).collect();

        let img_dataset = ImageDataset::new(items);
        let train_ratio = 1.0 - config.validation_split;
        if let Some(ref split_indices) = config.split_indices {
            img_dataset.split_by_indices(&split_indices.train_indices, &split_indices.val_indices)
        } else {
            img_dataset.split(train_ratio)
        }
    };

    let num_classes = dataset_train.num_classes();

    event_bus.emit(LabEvent::DataLoaded {
        session_id: session_id.clone(),
        rows: dataset_train.len() + dataset_valid.len(),
        columns: 0,
    });

    let channels = if dataset_train.items.is_empty() { 1 } else { dataset_train.items[0].channels };
    let image_height = if dataset_train.items.is_empty() { 28 } else { dataset_train.items[0].height };
    let image_width = if dataset_train.items.is_empty() { 28 } else { dataset_train.items[0].width };

    let batcher_train = ImageBatcher::<B>::new(device.clone(), channels, image_height, image_width);
    let batcher_valid = ImageBatcher::<B::InnerBackend>::new(device.clone(), channels, image_height, image_width);

    let mut dataloader_train_builder = DataLoaderBuilder::new(batcher_train)
        .batch_size(config.batch_size)
        .num_workers(2);
    if config.shuffle {
        dataloader_train_builder = dataloader_train_builder.shuffle(seed);
    }
    let dataloader_train = dataloader_train_builder.build(dataset_train);

    let dataloader_valid = DataLoaderBuilder::new(batcher_valid)
        .batch_size(config.batch_size)
        .num_workers(2)
        .build(dataset_valid);

    let model = DynamicCnn::<B>::new(channels, num_classes, &device);

    let optim = create_adam_config(&config.optimizer).init::<B, DynamicCnn<B>>();
    let renderer = EventBusRenderer::new(event_bus, session_id, train_control, config.batch_size);

    dispatch_lr_scheduler!(config, config.learning_rate, config.epochs, |lr_scheduler| {
        let mut training = SupervisedTraining::new(artifact_dir, dataloader_train.clone(), dataloader_valid.clone())
            .metrics((AccuracyMetric::<NdArray>::new(), LossMetric::<NdArray>::new()))
            .num_epochs(config.epochs)
            .renderer(renderer.clone());

        if let Some(epoch) = config.checkpoint_interval {
            training = training.checkpoint(epoch);
        }

        if let Some(ref es_config) = config.early_stopping {
            let loss_metric = LossMetric::<NdArray>::new();
            let strategy = MetricEarlyStoppingStrategy::new(
                &loss_metric,
                burn::train::metric::store::Aggregate::Mean,
                burn::train::metric::store::Direction::Lowest,
                burn::train::metric::store::Split::Valid,
                StoppingCondition::NoImprovementSince { n_epochs: es_config.patience },
            );
            training = training.early_stopping(strategy);
        }

        let learner = Learner::new(model.clone(), optim.clone(), lr_scheduler);
        let _result: LearningResult<_> = training.launch(learner);
        Ok::<(), crate::core::LabError>(())
    })
}

fn create_adam_config(config: &OptimizerConfig) -> AdamConfig {
    match config {
        OptimizerConfig::Adam { beta1, beta2, weight_decay } => {
            let mut adam = AdamConfig::new();
            adam = adam.with_beta_1(*beta1 as f32);
            adam = adam.with_beta_2(*beta2 as f32);
            if let Some(wd) = weight_decay {
                adam = adam.with_weight_decay(Some(
                    burn::optim::decay::WeightDecayConfig::new(*wd as f32)
                ));
            }
            adam
        }
        OptimizerConfig::AdamW { beta1, beta2, weight_decay } => {
            let mut adam = AdamConfig::new();
            adam = adam.with_beta_1(*beta1 as f32);
            adam = adam.with_beta_2(*beta2 as f32);
            adam = adam.with_weight_decay(Some(
                burn::optim::decay::WeightDecayConfig::new(*weight_decay as f32)
            ));
            crate::infrastructure::log("BURN", "AdamW requested but Burn only supports Adam; applying weight_decay from AdamW config", None);
            adam
        }
        OptimizerConfig::Sgd { momentum, weight_decay } => {
            crate::infrastructure::log("BURN", "SGD requested but Burn only supports Adam; using Adam as fallback", None);
            let mut adam = AdamConfig::new();
            if let Some(wd) = weight_decay {
                adam = adam.with_weight_decay(Some(
                    burn::optim::decay::WeightDecayConfig::new(*wd as f32)
                ));
            }
            let _ = momentum;
            adam
        }
        OptimizerConfig::Rmsprop { alpha, weight_decay } => {
            crate::infrastructure::log("BURN", "Rmsprop requested but Burn only supports Adam; using Adam as fallback", None);
            let mut adam = AdamConfig::new();
            if let Some(wd) = weight_decay {
                adam = adam.with_weight_decay(Some(
                    burn::optim::decay::WeightDecayConfig::new(*wd as f32)
                ));
            }
            let _ = alpha;
            adam
        }
        OptimizerConfig::Custom { name, .. } => {
            crate::infrastructure::log("BURN", &format!("Custom optimizer '{}' not supported by Burn; using Adam as fallback", name), None);
            AdamConfig::new()
        }
    }
}

pub fn run_mnist_training(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    epochs: usize,
    batch_size: usize,
    learning_rate: f64,
    seed: u64,
    artifact_dir: &str,
) -> crate::core::Result<()> {
    let config = TrainingConfig {
        session_name: "mnist".to_string(),
        task_type: TaskType::Classification,
        engine_id: "burn".to_string(),
        model_id: "mlp".to_string(),
        data_source_id: "csv".to_string(),
        data_path: String::new(),
        dataset_id: None,
        dataset_version: None,
        epochs,
        batch_size,
        learning_rate,
        optimizer: OptimizerConfig::default(),
        loss_function: "cross_entropy".to_string(),
        compute_backend: ComputeBackend::Wgpu,
        data_format: crate::types::DataFormat::Csv,
        validation_split: 0.2,
        test_split: 0.0,
        shuffle: true,
        seed: Some(seed),
        split_name: None,
        split_indices: None,
        checkpoint_interval: None,
        early_stopping: None,
        lr_scheduler: LrSchedulerConfig::default(),
        target_columns: vec!["label".to_string()],
        feature_columns: vec![],
        custom_params: std::collections::HashMap::new(),
    };

    run_training(event_bus, session_id, &config, artifact_dir, Arc::new(TrainControl::new()))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InferenceResult {
    pub predictions: Vec<f32>,
    pub predicted_classes: Vec<usize>,
    pub probabilities: Vec<Vec<f32>>,
}

pub fn run_inference(
    config: &TrainingConfig,
    artifact_dir: &str,
    input_data: &[Vec<f32>],
) -> crate::core::Result<InferenceResult> {
    match config.compute_backend {
        ComputeBackend::Wgpu | ComputeBackend::Cuda | ComputeBackend::Metal | ComputeBackend::Rocm => {
            run_inference_with_backend::<Wgpu>(config, artifact_dir, input_data)
        }
        ComputeBackend::Cpu => {
            run_inference_with_backend::<NdArray>(config, artifact_dir, input_data)
        }
    }
}

fn run_inference_with_backend<B>(
    config: &TrainingConfig,
    artifact_dir: &str,
    input_data: &[Vec<f32>],
) -> crate::core::Result<InferenceResult>
where
    B: burn::tensor::backend::Backend,
{
    let device = B::Device::default();
    let is_classification = matches!(config.task_type, TaskType::Classification);
    let model_id = &config.model_id;

    if model_id.starts_with("cnn") {
        let channels: usize = config.custom_params.get("input_channels")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;
        let num_classes: usize = load_model_metadata(artifact_dir)
            .as_ref()
            .and_then(|m| m.get("num_classes"))
            .and_then(|v| v.as_u64())
            .or_else(|| config.custom_params.get("num_classes").and_then(|v| v.as_u64()))
            .unwrap_or(10) as usize;
        let height: usize = config.custom_params.get("input_height")
            .and_then(|v| v.as_u64())
            .unwrap_or(28) as usize;
        let width: usize = config.custom_params.get("input_width")
            .and_then(|v| v.as_u64())
            .unwrap_or(28) as usize;

        let model = DynamicCnn::<B>::new(channels, num_classes, &device);

        let checkpoint_path = find_latest_checkpoint(artifact_dir);
        let checkpoint_path = match checkpoint_path {
            Some(p) => p,
            None => return Err(crate::core::LabError::InferenceFailed("No checkpoint found".to_string())),
        };

        let recorder = burn::record::DefaultFileRecorder::<burn::record::FullPrecisionSettings>::new();
        let model_record = recorder.load(checkpoint_path, &device)
            .map_err(|e| crate::core::LabError::InferenceFailed(format!("Failed to load checkpoint: {:?}", e)))?;
        let model = model.load_record(model_record);

        let image_size = height * width;
        let batch_size = input_data.len();
        let mut all_features = Vec::with_capacity(batch_size * channels * image_size);
        for row in input_data {
            let expected = channels * image_size;
            if row.len() < expected {
                let mut padded = row.clone();
                padded.resize(expected, 0.0);
                all_features.extend_from_slice(&padded);
            } else {
                all_features.extend_from_slice(&row[..expected]);
            }
        }

        let input_data = TensorData::new(all_features, vec![batch_size, channels * image_size]);
        let input_tensor = Tensor::<B, 2>::from_floats(input_data, &device);
        let input_tensor = input_tensor.reshape([batch_size, channels, height, width]);
        let output = model.forward(input_tensor);

        let output_data: Vec<f32> = output.into_data().to_vec()
            .map_err(|e| crate::core::LabError::InferenceFailed(format!("Failed to read output: {:?}", e)))?;

        let mut predictions = Vec::new();
        let mut predicted_classes = Vec::new();
        let mut probabilities = Vec::new();

        if is_classification {
            for i in 0..batch_size {
                let mut max_val = f32::NEG_INFINITY;
                let mut max_idx = 0;
                let mut probs = Vec::with_capacity(num_classes);
                for j in 0..num_classes {
                    let val = output_data[i * num_classes + j];
                    if val > max_val {
                        max_val = val;
                        max_idx = j;
                    }
                }
                let exp_sum: f32 = (0..num_classes)
                    .map(|j| (output_data[i * num_classes + j] - max_val).exp())
                    .sum();
                for j in 0..num_classes {
                    let val = output_data[i * num_classes + j];
                    let prob = (val - max_val).exp() / exp_sum;
                    probs.push(prob.max(0.0).min(1.0));
                }
                predictions.push(max_val);
                predicted_classes.push(max_idx);
                probabilities.push(probs);
            }
        } else {
            for i in 0..batch_size {
                predictions.push(output_data[i]);
                predicted_classes.push(0);
                probabilities.push(vec![output_data[i]]);
            }
        }

        Ok(InferenceResult { predictions, predicted_classes, probabilities })
    } else {
        let num_features = input_data.first().map(|r| r.len()).unwrap_or(1);
        let num_outputs: usize = load_model_metadata(artifact_dir)
            .as_ref()
            .and_then(|m| m.get("num_classes").or(m.get("num_outputs")))
            .and_then(|v| v.as_u64())
            .or_else(|| config.custom_params.get("num_classes").and_then(|v| v.as_u64()))
            .unwrap_or(1) as usize;
        let hidden_sizes: Vec<usize> = load_model_metadata(artifact_dir)
            .as_ref()
            .and_then(|m| m.get("hidden_sizes"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect())
            .or_else(|| config.custom_params.get("hidden_sizes")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect()))
            .unwrap_or_else(|| {
                if num_features <= 10 { vec![64, 32] }
                else if num_features <= 100 { vec![128, 64] }
                else { vec![256, 128, 64] }
            });

        let model = DynamicMlp::<B>::new(num_features, hidden_sizes, num_outputs, &device);

        let checkpoint_path = find_latest_checkpoint(artifact_dir);
        let checkpoint_path = match checkpoint_path {
            Some(p) => p,
            None => return Err(crate::core::LabError::InferenceFailed("No checkpoint found".to_string())),
        };

        let recorder = burn::record::DefaultFileRecorder::<burn::record::FullPrecisionSettings>::new();
        let model_record = recorder.load(checkpoint_path, &device)
            .map_err(|e| crate::core::LabError::InferenceFailed(format!("Failed to load checkpoint: {:?}", e)))?;
        let model = model.load_record(model_record);

        let batch_size = input_data.len();
        let mut all_features = Vec::with_capacity(batch_size * num_features);
        for row in input_data {
            if row.len() < num_features {
                let mut padded = row.clone();
                padded.resize(num_features, 0.0);
                all_features.extend_from_slice(&padded);
            } else {
                all_features.extend_from_slice(&row[..num_features]);
            }
        }

        let input_data = TensorData::new(all_features, vec![batch_size, num_features]);
        let input_tensor = Tensor::<B, 2>::from_floats(input_data, &device);
        let output = model.forward(input_tensor);

        let output_data: Vec<f32> = output.into_data().to_vec()
            .map_err(|e| crate::core::LabError::InferenceFailed(format!("Failed to read output: {:?}", e)))?;

        let mut predictions = Vec::new();
        let mut predicted_classes = Vec::new();
        let mut probabilities = Vec::new();

        if is_classification {
            for i in 0..batch_size {
                let mut max_val = f32::NEG_INFINITY;
                let mut max_idx = 0;
                let mut probs = Vec::with_capacity(num_outputs);
                for j in 0..num_outputs {
                    let val = output_data[i * num_outputs + j];
                    if val > max_val {
                        max_val = val;
                        max_idx = j;
                    }
                }
                let exp_sum: f32 = (0..num_outputs)
                    .map(|j| (output_data[i * num_outputs + j] - max_val).exp())
                    .sum();
                for j in 0..num_outputs {
                    let val = output_data[i * num_outputs + j];
                    let prob = (val - max_val).exp() / exp_sum;
                    probs.push(prob.max(0.0).min(1.0));
                }
                predictions.push(max_val);
                predicted_classes.push(max_idx);
                probabilities.push(probs);
            }
        } else {
            for i in 0..batch_size {
                predictions.push(output_data[i]);
                predicted_classes.push(0);
                probabilities.push(vec![output_data[i]]);
            }
        }

        Ok(InferenceResult { predictions, predicted_classes, probabilities })
    }
}

fn find_latest_checkpoint(artifact_dir: &str) -> Option<std::path::PathBuf> {
    let checkpoint_dir = std::path::Path::new(artifact_dir);
    if !checkpoint_dir.exists() {
        return None;
    }

    let mut latest_epoch: usize = 0;
    let mut latest_path: Option<std::path::PathBuf> = None;

    if let Ok(entries) = std::fs::read_dir(checkpoint_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy();
            if name.starts_with("checkpoint-") || name.starts_with("model-") {
                if path.is_dir() {
                    if let Some(checkpoint_file) = find_checkpoint_in_dir(&path) {
                        if let Some(epoch_str) = name.split('-').last() {
                            if let Ok(epoch) = epoch_str.parse::<usize>() {
                                if epoch >= latest_epoch {
                                    latest_epoch = epoch;
                                    latest_path = Some(checkpoint_file);
                                }
                            }
                        }
                    }
                } else {
                    let is_checkpoint = name.ends_with(".mpk.gz")
                        || name.ends_with(".mpk")
                        || name.ends_with(".bin");
                    if is_checkpoint {
                        if let Some(epoch_str) = name.split('-').last() {
                            let epoch_result = epoch_str
                                .strip_suffix(".mpk.gz")
                                .or_else(|| epoch_str.strip_suffix(".mpk"))
                                .or_else(|| epoch_str.strip_suffix(".bin"))
                                .and_then(|s| s.parse::<usize>().ok())
                                .or_else(|| epoch_str.parse::<usize>().ok());
                            if let Some(epoch) = epoch_result {
                                if epoch >= latest_epoch {
                                    latest_epoch = epoch;
                                    latest_path = Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if latest_path.is_none() {
        if let Some(cp_file) = find_checkpoint_in_dir(checkpoint_dir) {
            return Some(cp_file);
        }
        let mpk_path = checkpoint_dir.join("checkpoint.mpk.gz");
        if mpk_path.exists() {
            return Some(mpk_path);
        }
        let json_path = checkpoint_dir.join("checkpoint.json");
        if json_path.exists() {
            return Some(json_path);
        }
    }

    latest_path
}

pub fn find_checkpoint_in_dir(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name()?.to_string_lossy();
                let is_checkpoint = name.ends_with(".mpk.gz")
                    || name.ends_with(".mpk")
                    || name.ends_with(".bin");
                if is_checkpoint {
                    if name.starts_with("model") || name.starts_with("checkpoint") || name == "model.mpk.gz" || name == "model.mpk" || name == "model.bin" {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

fn load_model_metadata(artifact_dir: &str) -> Option<serde_json::Value> {
    let metadata_path = std::path::Path::new(artifact_dir).join("model_metadata.json");
    if metadata_path.exists() {
        let content = std::fs::read_to_string(&metadata_path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}
