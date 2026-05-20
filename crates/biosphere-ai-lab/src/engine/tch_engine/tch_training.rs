use std::sync::Arc;

use tch::{nn, nn::{Module, OptimizerConfig}, Device, Kind, Tensor, Reduction};

use crate::core::config::TrainingConfig;
use crate::core::event::{EventBus, LabEvent};
use crate::core::LabError;
use crate::engine::burn_training::TrainControl;
use crate::types::{ComputeBackend, SessionId, TaskType};

fn select_device(config: &TrainingConfig) -> Device {
    match config.compute_backend {
        ComputeBackend::Cuda => {
            if tch::Cuda::is_available() {
                Device::Cuda(0)
            } else {
                Device::Cpu
            }
        }
        _ => Device::Cpu,
    }
}

fn load_csv_data(
    path: &str,
    feature_columns: &[String],
    target_column: &str,
    has_header: bool,
    delimiter: u8,
) -> crate::core::Result<(Vec<Vec<f32>>, Vec<f32>)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file: {}", e)))?;

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(has_header)
        .from_reader(content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| LabError::DataLoadFailed(format!("Cannot parse CSV headers: {}", e)))?
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
                    .ok_or_else(|| LabError::DataLoadFailed(format!("Feature column '{}' not found", fc)))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    let target_index = header_vec
        .iter()
        .position(|h| h == target_column)
        .ok_or_else(|| LabError::DataLoadFailed(format!("Target column '{}' not found", target_column)))?;

    let mut features_all = Vec::new();
    let mut labels_all = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| LabError::DataLoadFailed(format!("CSV parse error: {}", e)))?;

        let mut features = Vec::with_capacity(feature_indices.len());
        for &idx in &feature_indices {
            let val: f32 = record.get(idx).unwrap_or("0").trim().parse().unwrap_or(0.0);
            features.push(val);
        }

        let label: f32 = record.get(target_index).unwrap_or("0").trim().parse().unwrap_or(0.0);
        features_all.push(features);
        labels_all.push(label);
    }

    if features_all.is_empty() {
        return Err(LabError::DataLoadFailed("No data rows found in CSV".to_string()));
    }

    Ok((features_all, labels_all))
}

fn load_json_data(
    path: &str,
    feature_columns: &[String],
    target_column: &str,
) -> crate::core::Result<(Vec<Vec<f32>>, Vec<f32>)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file: {}", e)))?;

    let json_value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| LabError::DataLoadFailed(format!("JSON parse error: {}", e)))?;

    let array = json_value
        .as_array()
        .ok_or_else(|| LabError::DataLoadFailed("JSON root must be an array".to_string()))?;

    if array.is_empty() {
        return Err(LabError::DataLoadFailed("JSON array is empty".to_string()));
    }

    let first_obj = array[0]
        .as_object()
        .ok_or_else(|| LabError::DataLoadFailed("JSON items must be objects".to_string()))?;

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
                    .ok_or_else(|| LabError::DataLoadFailed(format!("Feature column '{}' not found", fc)))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    let target_index = header_vec
        .iter()
        .position(|h| h == target_column)
        .ok_or_else(|| LabError::DataLoadFailed(format!("Target column '{}' not found", target_column)))?;

    let mut features_all = Vec::new();
    let mut labels_all = Vec::new();

    for item_val in array {
        let obj = item_val.as_object()
            .ok_or_else(|| LabError::DataLoadFailed("JSON item must be an object".to_string()))?;

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

        features_all.push(features);
        labels_all.push(label);
    }

    if features_all.is_empty() {
        return Err(LabError::DataLoadFailed("No data rows found in JSON".to_string()));
    }

    Ok((features_all, labels_all))
}

#[derive(Debug)]
struct TchMlp {
    layers: Vec<nn::Linear>,
    output_layer: nn::Linear,
}

impl TchMlp {
    fn new(vs: &nn::Path, input_size: usize, hidden_sizes: Vec<usize>, output_size: usize) -> Self {
        let mut layers = Vec::new();
        let mut current_size = input_size;

        for (i, &hidden_size) in hidden_sizes.iter().enumerate() {
            layers.push(nn::linear(
                vs / format!("hidden_{}", i),
                current_size as i64,
                hidden_size as i64,
                Default::default(),
            ));
            current_size = hidden_size;
        }

        let output_layer = nn::linear(
            vs / "output",
            current_size as i64,
            output_size as i64,
            Default::default(),
        );

        Self { layers, output_layer }
    }

    fn forward_train(&self, xs: &Tensor) -> Tensor {
        let mut x = xs.shallow_clone();
        for layer in &self.layers {
            x = layer.forward(&x).relu();
        }
        self.output_layer.forward(&x)
    }

    fn forward_infer(&self, xs: &Tensor) -> Tensor {
        let mut x = xs.shallow_clone();
        for layer in &self.layers {
            x = layer.forward(&x).relu().detach();
        }
        self.output_layer.forward(&x)
    }
}

impl tch::nn::ModuleT for TchMlp {
    fn forward_t(&self, xs: &Tensor, _train: bool) -> Tensor {
        self.forward_train(xs)
    }
}

#[derive(Debug)]
struct TchCnn {
    conv1: nn::Conv2D,
    conv2: nn::Conv2D,
    fc1: nn::Linear,
    fc2: nn::Linear,
    fc3: nn::Linear,
}

impl TchCnn {
    fn new(vs: &nn::Path, channels: i64, height: i64, width: i64, num_classes: i64) -> Self {
        let conv1 = nn::conv2d(vs / "conv1", channels, 32, 3, nn::ConvConfig { padding: 1, ..Default::default() });
        let conv2 = nn::conv2d(vs / "conv2", 32, 64, 3, nn::ConvConfig { padding: 1, ..Default::default() });

        let h = height / 4;
        let w = width / 4;
        let fc_input = 64 * h * w;

        let fc1 = nn::linear(vs / "fc1", fc_input, 128, Default::default());
        let fc2 = nn::linear(vs / "fc2", 128, 64, Default::default());
        let fc3 = nn::linear(vs / "fc3", 64, num_classes, Default::default());

        Self { conv1, conv2, fc1, fc2, fc3 }
    }

    fn forward_train(&self, xs: &Tensor) -> Tensor {
        xs.apply(&self.conv1)
            .relu()
            .max_pool2d_default(2)
            .apply(&self.conv2)
            .relu()
            .max_pool2d_default(2)
            .flat_view()
            .apply(&self.fc1)
            .relu()
            .apply(&self.fc2)
            .relu()
            .apply(&self.fc3)
    }

    fn forward_infer(&self, xs: &Tensor) -> Tensor {
        self.forward_train(xs).detach()
    }
}

impl tch::nn::ModuleT for TchCnn {
    fn forward_t(&self, xs: &Tensor, _train: bool) -> Tensor {
        self.forward_train(xs)
    }
}

fn build_optimizer(
    vs: &nn::VarStore,
    config: &TrainingConfig,
) -> crate::core::Result<tch::nn::Optimizer> {
    match &config.optimizer {
        crate::core::config::OptimizerConfig::Adam { beta1, beta2, weight_decay } => {
            let mut opt = nn::Adam {
                beta1: *beta1,
                beta2: *beta2,
                ..Default::default()
            };
            if let Some(wd) = weight_decay {
                opt.wd = *wd;
            }
            opt.build(vs, config.learning_rate)
                .map_err(|e| LabError::Custom(format!("Optimizer build error: {}", e)))
        }
        crate::core::config::OptimizerConfig::AdamW { beta1, beta2, weight_decay } => {
            let opt = nn::Adam {
                beta1: *beta1,
                beta2: *beta2,
                wd: *weight_decay,
                ..Default::default()
            };
            opt.build(vs, config.learning_rate)
                .map_err(|e| LabError::Custom(format!("Optimizer build error: {}", e)))
        }
        crate::core::config::OptimizerConfig::Sgd { momentum, weight_decay } => {
            let mut opt = nn::Sgd {
                momentum: momentum.unwrap_or(0.0),
                ..Default::default()
            };
            if let Some(wd) = weight_decay {
                opt.wd = *wd;
            }
            opt.build(vs, config.learning_rate)
                .map_err(|e| LabError::Custom(format!("Optimizer build error: {}", e)))
        }
        crate::core::config::OptimizerConfig::Rmsprop { .. } => {
            nn::RmsProp::default().build(vs, config.learning_rate)
                .map_err(|e| LabError::Custom(format!("Optimizer build error: {}", e)))
        }
        crate::core::config::OptimizerConfig::Custom { name, .. } => {
            Err(LabError::Custom(format!("Unsupported optimizer: {}", name)))
        }
    }
}

fn get_lr_for_epoch(config: &TrainingConfig, epoch: usize) -> f64 {
    let base_lr = config.learning_rate;
    match &config.lr_scheduler {
        crate::core::config::LrSchedulerConfig::Constant => base_lr,
        crate::core::config::LrSchedulerConfig::Step { step_size, gamma } => {
            let num_steps = epoch / step_size;
            base_lr * gamma.powi(num_steps as i32)
        }
        crate::core::config::LrSchedulerConfig::Exponential { gamma } => {
            base_lr * gamma.powi(epoch as i32)
        }
        crate::core::config::LrSchedulerConfig::CosineAnnealing { min_lr, num_iters } => {
            if epoch >= *num_iters {
                return *min_lr;
            }
            let progress = epoch as f64 / *num_iters as f64;
            min_lr + 0.5 * (base_lr - min_lr) * (1.0 + (std::f64::consts::PI * progress).cos())
        }
        crate::core::config::LrSchedulerConfig::Linear { final_lr, num_iters } => {
            if epoch >= *num_iters {
                return *final_lr;
            }
            let progress = epoch as f64 / *num_iters as f64;
            base_lr + (final_lr - base_lr) * progress
        }
    }
}

pub fn run_tch_training(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()> {
    std::fs::create_dir_all(artifact_dir).ok();

    let device = select_device(config);

    event_bus.emit(LabEvent::LogOutput {
        session_id: session_id.clone(),
        level: "info".to_string(),
        message: format!("tch-rs engine: using device {:?}", device),
    });

    match config.model_id.as_str() {
        "cnn" => run_cnn_training(event_bus, session_id, config, artifact_dir, device, train_control),
        "mlp" | _ => run_mlp_training(event_bus, session_id, config, artifact_dir, device, train_control),
    }
}

fn run_mlp_training(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    device: Device,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()> {
    let (features_all, labels_all) = if config.data_path.is_empty() {
        return Err(LabError::DataLoadFailed("No data path specified for tch-rs engine".to_string()));
    } else {
        let target_column = if config.target_columns.is_empty() {
            "label".to_string()
        } else {
            config.target_columns[0].clone()
        };

        match config.data_format {
            crate::types::DataFormat::Json => {
                load_json_data(&config.data_path, &config.feature_columns, &target_column)?
            }
            _ => {
                let delimiter = config.custom_params.get("delimiter")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.chars().next())
                    .unwrap_or(',') as u8;
                load_csv_data(&config.data_path, &config.feature_columns, &target_column, true, delimiter)?
            }
        }
    };

    let num_features = features_all[0].len();
    let is_classification = matches!(config.task_type, TaskType::Classification)
        || (!matches!(config.task_type, TaskType::Regression)
            && labels_all.iter().all(|l| l.floor() == *l && *l >= 0.0));
    let num_classes = if is_classification {
        labels_all.iter().map(|l| *l as usize).max().unwrap_or(0) + 1
    } else {
        1
    };
    let num_outputs = if is_classification { num_classes } else { 1 };

    event_bus.emit(LabEvent::DataLoaded {
        session_id: session_id.clone(),
        rows: features_all.len(),
        columns: num_features,
    });

    let hidden_sizes = if num_features <= 10 {
        vec![64, 32]
    } else if num_features <= 100 {
        vec![128, 64]
    } else {
        vec![256, 128, 64]
    };

    let total = features_all.len();
    let train_end = ((total as f64) * (1.0 - config.validation_split)) as usize;

    let flat_features: Vec<f32> = features_all.iter().flatten().copied().collect();

    let all_features = Tensor::from_slice(&flat_features)
        .reshape(&[total as i64, num_features as i64])
        .to(device);

    let labels_tensor = if is_classification {
        let labels_i64: Vec<i64> = labels_all.iter().map(|l| *l as i64).collect();
        Tensor::from_slice(&labels_i64).to(device)
    } else {
        Tensor::from_slice(&labels_all).reshape(&[total as i64, 1]).to(device)
    };

    let perm = Tensor::randperm(total as i64, (Kind::Int64, device));
    let all_features = all_features.index_select(0, &perm);
    let labels_tensor = labels_tensor.index_select(0, &perm);

    let train_features = all_features.narrow(0, 0, train_end as i64);
    let train_labels = labels_tensor.narrow(0, 0, train_end as i64);
    let val_features = all_features.narrow(0, train_end as i64, (total - train_end) as i64);
    let val_labels = labels_tensor.narrow(0, train_end as i64, (total - train_end) as i64);

    let vs = nn::VarStore::new(device);
    let model = TchMlp::new(&vs.root(), num_features, hidden_sizes.clone(), num_outputs);

    let mut optimizer = build_optimizer(&vs, config)?;

    let batch_size = config.batch_size as i64;
    let num_train = train_end as i64;
    let num_batches = (num_train + batch_size - 1) / batch_size;

    let mut best_val_loss = f64::MAX;
    let mut patience_counter = 0usize;

    for epoch in 0..config.epochs {
        if train_control.is_cancelled() {
            return Err(LabError::TrainingFailed("Training cancelled".to_string()));
        }
        train_control.wait_while_paused();

        let lr = get_lr_for_epoch(config, epoch);
        optimizer.set_lr(lr);

        let mut epoch_loss = 0.0f64;
        let mut epoch_correct = 0i64;
        let mut epoch_total = 0i64;

        let perm = if config.shuffle {
            Tensor::randperm(num_train, (Kind::Int64, device))
        } else {
            Tensor::arange(num_train, (Kind::Int64, device))
        };

        let shuffled_features = train_features.index_select(0, &perm);
        let shuffled_labels = train_labels.index_select(0, &perm);

        for batch_idx in 0..num_batches {
            if train_control.is_cancelled() {
                return Err(LabError::TrainingFailed("Training cancelled".to_string()));
            }
            train_control.wait_while_paused();

            let start = batch_idx * batch_size;
            let end = std::cmp::min(start + batch_size, num_train);

            let batch_x = shuffled_features.narrow(0, start, end - start);
            let batch_y = shuffled_labels.narrow(0, start, end - start);

            let output = model.forward_train(&batch_x);

            let loss = if is_classification {
                output.cross_entropy_for_logits(&batch_y)
            } else {
                output.mse_loss(&batch_y, Reduction::Mean)
            };

            epoch_loss += loss.double_value(&[]) * (end - start) as f64;

            if is_classification {
                epoch_correct += output.argmax(-1, false).eq_tensor(&batch_y).sum(tch::Kind::Int64).double_value(&[]) as i64;
            }
            epoch_total += end - start;

            optimizer.backward_step(&loss);

            if batch_idx % 10 == 0 || batch_idx == num_batches - 1 {
                let batch_loss = loss.double_value(&[]);
                event_bus.emit(LabEvent::BatchCompleted {
                    session_id: session_id.clone(),
                    batch: (batch_idx + 1) as usize,
                    total_batches: num_batches as usize,
                    loss: batch_loss,
                });
            }
        }

        let avg_train_loss = epoch_loss / epoch_total as f64;
        let train_acc = if is_classification {
            epoch_correct as f64 / epoch_total as f64
        } else {
            0.0
        };

        let val_output = model.forward_infer(&val_features);
        let val_loss_tensor = if is_classification {
            val_output.cross_entropy_for_logits(&val_labels)
        } else {
            val_output.mse_loss(&val_labels, Reduction::Mean)
        };
        let val_loss = val_loss_tensor.double_value(&[]);

        let val_acc = if is_classification {
            val_output.argmax(-1, false).eq_tensor(&val_labels).sum(tch::Kind::Float).double_value(&[])
                / val_labels.size()[0] as f64
        } else {
            0.0
        };

        let progress_msg = if is_classification {
            format!(
                "Epoch {}/{} - train_loss: {:.4} - train_acc: {:.1}% - val_loss: {:.4} - val_acc: {:.1}%",
                epoch + 1, config.epochs, avg_train_loss, train_acc * 100.0, val_loss, val_acc * 100.0
            )
        } else {
            format!(
                "Epoch {}/{} - train_loss: {:.4} - val_loss: {:.4}",
                epoch + 1, config.epochs, avg_train_loss, val_loss
            )
        };

        event_bus.emit(LabEvent::EpochCompleted {
            session_id: session_id.clone(),
            epoch: epoch + 1,
            total_epochs: config.epochs,
            train_loss: avg_train_loss,
            val_loss: Some(val_loss),
            metrics: serde_json::json!({
                "train_acc": train_acc,
                "val_acc": val_acc,
                "val_loss": val_loss,
            }),
        });

        event_bus.emit(LabEvent::ProgressUpdate {
            session_id: session_id.clone(),
            progress: (epoch + 1) as f64 / config.epochs as f64,
            message: progress_msg.clone(),
        });

        event_bus.emit(LabEvent::LogOutput {
            session_id: session_id.clone(),
            level: "info".to_string(),
            message: progress_msg,
        });

        if let Some(interval) = config.checkpoint_interval {
            if (epoch + 1) % interval == 0 {
                let ckpt_path = format!("{}/checkpoint_epoch_{}.ot", artifact_dir, epoch + 1);
                vs.save(&ckpt_path)
                    .map_err(|e| LabError::Custom(format!("Failed to save checkpoint: {}", e)))?;
                event_bus.emit(LabEvent::CheckpointSaved {
                    session_id: session_id.clone(),
                    path: ckpt_path,
                    epoch: epoch + 1,
                });
            }
        }

        if let Some(ref es_config) = config.early_stopping {
            if val_loss < best_val_loss - es_config.min_delta {
                best_val_loss = val_loss;
                patience_counter = 0;
            } else {
                patience_counter += 1;
                if patience_counter >= es_config.patience {
                    event_bus.emit(LabEvent::LogOutput {
                        session_id: session_id.clone(),
                        level: "info".to_string(),
                        message: format!("Early stopping triggered at epoch {} (patience={})", epoch + 1, es_config.patience),
                    });
                    break;
                }
            }
        }
    }

    let final_path = format!("{}/model_final.ot", artifact_dir);
    vs.save(&final_path)
        .map_err(|e| LabError::Custom(format!("Failed to save final model: {}", e)))?;

    let metadata = serde_json::json!({
        "engine": "tch-rs",
        "model_id": config.model_id,
        "num_features": num_features,
        "num_classes": num_outputs,
        "is_classification": is_classification,
        "hidden_sizes": hidden_sizes,
        "task_type": config.task_type,
        "model_path": final_path,
    });
    let metadata_path = std::path::Path::new(artifact_dir).join("model_metadata.json");
    if let Err(e) = std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap_or_default()) {
        crate::infrastructure::log("TCH", &format!("Failed to write model metadata: {}", e), None);
    }

    Ok(())
}

pub fn create_model_for_export(
    model_id: &str,
    vs_root: &nn::Path,
    num_features: usize,
    num_outputs: usize,
    _batch_size: i64,
) -> Box<dyn tch::nn::ModuleT + Send> {
    if model_id == "cnn" {
        let channels = 1;
        let height = 28;
        let width = 28;
        Box::new(TchCnn::new(vs_root, channels, height, width, num_outputs as i64))
    } else {
        let hidden_sizes = if num_features <= 10 {
            vec![64, 32]
        } else if num_features <= 100 {
            vec![128, 64]
        } else {
            vec![256, 128, 64]
        };
        Box::new(TchMlp::new(vs_root, num_features, hidden_sizes, num_outputs))
    }
}

pub fn run_tch_training_from_checkpoint(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    checkpoint_epoch: usize,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()> {
    let metadata = load_tch_metadata(artifact_dir)
        .ok_or_else(|| LabError::InferenceFailed("No model_metadata.json found for checkpoint resume".to_string()))?;

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

    let device = select_device(config);

    event_bus.emit(LabEvent::LogOutput {
        session_id: session_id.clone(),
        level: "info".to_string(),
        message: format!("tch-rs: Resuming training from epoch {}, {} epochs remaining", checkpoint_epoch, remaining_epochs),
    });

    let model_id = metadata.get("model_id")
        .and_then(|v| v.as_str())
        .unwrap_or("mlp");

    let checkpoint_path = find_tch_checkpoint(artifact_dir, checkpoint_epoch)
        .ok_or_else(|| LabError::InferenceFailed(format!("No checkpoint found at or near epoch {}", checkpoint_epoch)))?;

    if model_id == "cnn" {
        run_cnn_training_from_checkpoint(event_bus, session_id, &resume_config, artifact_dir, device, train_control, &checkpoint_path, checkpoint_epoch, &metadata)
    } else {
        run_mlp_training_from_checkpoint(event_bus, session_id, &resume_config, artifact_dir, device, train_control, &checkpoint_path, checkpoint_epoch, &metadata)
    }
}

fn find_tch_checkpoint(artifact_dir: &str, target_epoch: usize) -> Option<String> {
    let dir = std::path::Path::new(artifact_dir);
    if !dir.exists() {
        return None;
    }

    let mut best_path: Option<String> = None;
    let mut best_epoch: usize = 0;

    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("checkpoint_epoch_") && name.ends_with(".ot") {
            if let Some(epoch_str) = name.strip_prefix("checkpoint_epoch_").and_then(|s| s.strip_suffix(".ot")) {
                if let Ok(epoch) = epoch_str.parse::<usize>() {
                    if epoch <= target_epoch && epoch > best_epoch {
                        best_epoch = epoch;
                        best_path = Some(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    if best_path.is_none() {
        let final_path = dir.join("model_final.ot");
        if final_path.exists() {
            best_path = Some(final_path.to_string_lossy().to_string());
        }
    }

    best_path
}

fn run_mlp_training_from_checkpoint(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    device: Device,
    train_control: Arc<TrainControl>,
    checkpoint_path: &str,
    epoch_offset: usize,
    metadata: &serde_json::Value,
) -> crate::core::Result<()> {
    let (features_all, labels_all) = if config.data_path.is_empty() {
        return Err(LabError::DataLoadFailed("No data path specified for tch-rs engine".to_string()));
    } else {
        let target_column = if config.target_columns.is_empty() {
            "label".to_string()
        } else {
            config.target_columns[0].clone()
        };
        let delimiter = config.custom_params.get("delimiter")
            .and_then(|v| v.as_str())
            .and_then(|s| s.chars().next())
            .unwrap_or(',') as u8;
        load_csv_data(&config.data_path, &config.feature_columns, &target_column, true, delimiter)?
    };

    let num_features = metadata.get("num_features")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| features_all.first().map(|r| r.len() as u64).unwrap_or(1)) as usize;
    let num_classes = metadata.get("num_classes")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize;
    let is_classification = metadata.get("is_classification")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let num_outputs = if is_classification { num_classes } else { 1 };

    let hidden_sizes: Vec<usize> = metadata.get("hidden_sizes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect())
        .unwrap_or_else(|| {
            if num_features <= 10 { vec![64, 32] }
            else if num_features <= 100 { vec![128, 64] }
            else { vec![256, 128, 64] }
        });

    let total = features_all.len();
    let train_end = ((total as f64) * (1.0 - config.validation_split)) as usize;

    let flat_features: Vec<f32> = features_all.iter().flatten().copied().collect();
    let all_features = Tensor::from_slice(&flat_features)
        .reshape(&[total as i64, num_features as i64])
        .to(device);

    let labels_tensor = if is_classification {
        let labels_i64: Vec<i64> = labels_all.iter().map(|l| *l as i64).collect();
        Tensor::from_slice(&labels_i64).to(device)
    } else {
        Tensor::from_slice(&labels_all).reshape(&[total as i64, 1]).to(device)
    };

    let perm = Tensor::randperm(total as i64, (Kind::Int64, device));
    let all_features = all_features.index_select(0, &perm);
    let labels_tensor = labels_tensor.index_select(0, &perm);

    let train_features = all_features.narrow(0, 0, train_end as i64);
    let train_labels = labels_tensor.narrow(0, 0, train_end as i64);
    let val_features = all_features.narrow(0, train_end as i64, (total - train_end) as i64);
    let val_labels = labels_tensor.narrow(0, train_end as i64, (total - train_end) as i64);

    let mut vs = nn::VarStore::new(device);
    let model = TchMlp::new(&vs.root(), num_features, hidden_sizes.clone(), num_outputs);

    vs.load(checkpoint_path)
        .map_err(|e| LabError::InferenceFailed(format!("Failed to load checkpoint: {}", e)))?;

    let mut optimizer = build_optimizer(&vs, config)?;

    let batch_size = config.batch_size as i64;
    let num_train = train_end as i64;
    let num_batches = (num_train + batch_size - 1) / batch_size;

    let mut best_val_loss = f64::MAX;
    let mut patience_counter = 0usize;

    for epoch in 0..config.epochs {
        if train_control.is_cancelled() {
            return Err(LabError::TrainingFailed("Training cancelled".to_string()));
        }
        train_control.wait_while_paused();

        let lr = get_lr_for_epoch(config, epoch);
        optimizer.set_lr(lr);

        let mut epoch_loss = 0.0f64;
        let mut epoch_correct = 0i64;
        let mut epoch_total = 0i64;

        let perm = if config.shuffle {
            Tensor::randperm(num_train, (Kind::Int64, device))
        } else {
            Tensor::arange(num_train, (Kind::Int64, device))
        };

        let shuffled_features = train_features.index_select(0, &perm);
        let shuffled_labels = train_labels.index_select(0, &perm);

        for batch_idx in 0..num_batches {
            if train_control.is_cancelled() {
                return Err(LabError::TrainingFailed("Training cancelled".to_string()));
            }
            train_control.wait_while_paused();

            let start = batch_idx * batch_size;
            let end = std::cmp::min(start + batch_size, num_train);

            let batch_x = shuffled_features.narrow(0, start, end - start);
            let batch_y = shuffled_labels.narrow(0, start, end - start);

            let output = model.forward_train(&batch_x);

            let loss = if is_classification {
                output.cross_entropy_for_logits(&batch_y)
            } else {
                output.mse_loss(&batch_y, Reduction::Mean)
            };

            epoch_loss += loss.double_value(&[]) * (end - start) as f64;

            if is_classification {
                epoch_correct += output.argmax(-1, false).eq_tensor(&batch_y).sum(tch::Kind::Int64).double_value(&[]) as i64;
            }
            epoch_total += end - start;

            optimizer.backward_step(&loss);
        }

        let avg_train_loss = epoch_loss / epoch_total as f64;
        let train_acc = if is_classification { epoch_correct as f64 / epoch_total as f64 } else { 0.0 };

        let val_output = model.forward_infer(&val_features);
        let val_loss_tensor = if is_classification {
            val_output.cross_entropy_for_logits(&val_labels)
        } else {
            val_output.mse_loss(&val_labels, Reduction::Mean)
        };
        let val_loss = val_loss_tensor.double_value(&[]);

        let val_acc = if is_classification {
            val_output.argmax(-1, false).eq_tensor(&val_labels).sum(tch::Kind::Float).double_value(&[])
                / val_labels.size()[0] as f64
        } else {
            0.0
        };

        let actual_epoch = epoch_offset + epoch + 1;

        event_bus.emit(LabEvent::EpochCompleted {
            session_id: session_id.clone(),
            epoch: actual_epoch,
            total_epochs: epoch_offset + config.epochs,
            train_loss: avg_train_loss,
            val_loss: Some(val_loss),
            metrics: serde_json::json!({
                "train_acc": train_acc,
                "val_acc": val_acc,
            }),
        });

        event_bus.emit(LabEvent::ProgressUpdate {
            session_id: session_id.clone(),
            progress: (epoch + 1) as f64 / config.epochs as f64,
            message: format!("Epoch {}/{} (resumed from {})", actual_epoch, epoch_offset + config.epochs, epoch_offset),
        });

        if let Some(interval) = config.checkpoint_interval {
            if (actual_epoch) % interval == 0 {
                let ckpt_path = format!("{}/checkpoint_epoch_{}.ot", artifact_dir, actual_epoch);
                vs.save(&ckpt_path)
                    .map_err(|e| LabError::Custom(format!("Failed to save checkpoint: {}", e)))?;
                event_bus.emit(LabEvent::CheckpointSaved {
                    session_id: session_id.clone(),
                    path: ckpt_path,
                    epoch: actual_epoch,
                });
            }
        }

        if let Some(ref es_config) = config.early_stopping {
            if val_loss < best_val_loss - es_config.min_delta {
                best_val_loss = val_loss;
                patience_counter = 0;
            } else {
                patience_counter += 1;
                if patience_counter >= es_config.patience {
                    event_bus.emit(LabEvent::LogOutput {
                        session_id: session_id.clone(),
                        level: "info".to_string(),
                        message: format!("Early stopping at epoch {}", actual_epoch),
                    });
                    break;
                }
            }
        }
    }

    let final_path = format!("{}/model_final.ot", artifact_dir);
    vs.save(&final_path)
        .map_err(|e| LabError::Custom(format!("Failed to save final model: {}", e)))?;

    Ok(())
}

fn run_cnn_training_from_checkpoint(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    device: Device,
    train_control: Arc<TrainControl>,
    checkpoint_path: &str,
    epoch_offset: usize,
    metadata: &serde_json::Value,
) -> crate::core::Result<()> {
    let input_channels = metadata.get("input_channels")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as i64;
    let input_height = metadata.get("input_height")
        .and_then(|v| v.as_u64())
        .unwrap_or(28) as i64;
    let input_width = metadata.get("input_width")
        .and_then(|v| v.as_u64())
        .unwrap_or(28) as i64;
    let num_classes = metadata.get("num_classes")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as i64;

    let (features_all, labels_all) = if config.data_path.is_empty() {
        return Err(LabError::DataLoadFailed("No data path specified for tch-rs CNN training".to_string()));
    } else {
        let target_column = if config.target_columns.is_empty() {
            "label".to_string()
        } else {
            config.target_columns[0].clone()
        };
        let delimiter = config.custom_params.get("delimiter")
            .and_then(|v| v.as_str())
            .and_then(|s| s.chars().next())
            .unwrap_or(',') as u8;
        load_csv_data(&config.data_path, &config.feature_columns, &target_column, true, delimiter)?
    };

    let total = features_all.len();
    let train_end = ((total as f64) * (1.0 - config.validation_split)) as usize;

    let flat_features: Vec<f32> = features_all.iter().flatten().copied().collect();
    let all_features = Tensor::from_slice(&flat_features)
        .reshape(&[total as i64, input_channels, input_height, input_width])
        .to(device);

    let labels_i64: Vec<i64> = labels_all.iter().map(|l| *l as i64).collect();
    let labels_tensor = Tensor::from_slice(&labels_i64).to(device);

    let perm = Tensor::randperm(total as i64, (Kind::Int64, device));
    let all_features = all_features.index_select(0, &perm);
    let labels_tensor = labels_tensor.index_select(0, &perm);

    let train_features = all_features.narrow(0, 0, train_end as i64);
    let train_labels = labels_tensor.narrow(0, 0, train_end as i64);
    let val_features = all_features.narrow(0, train_end as i64, (total - train_end) as i64);
    let val_labels = labels_tensor.narrow(0, train_end as i64, (total - train_end) as i64);

    let mut vs = nn::VarStore::new(device);
    let model = TchCnn::new(&vs.root(), input_channels, input_height, input_width, num_classes);

    vs.load(checkpoint_path)
        .map_err(|e| LabError::InferenceFailed(format!("Failed to load checkpoint: {}", e)))?;

    let mut optimizer = build_optimizer(&vs, config)?;

    let batch_size = config.batch_size as i64;
    let num_train = train_end as i64;
    let num_batches = (num_train + batch_size - 1) / batch_size;

    let mut best_val_loss = f64::MAX;
    let mut patience_counter = 0usize;

    for epoch in 0..config.epochs {
        if train_control.is_cancelled() {
            return Err(LabError::TrainingFailed("Training cancelled".to_string()));
        }
        train_control.wait_while_paused();

        let lr = get_lr_for_epoch(config, epoch);
        optimizer.set_lr(lr);

        let mut epoch_loss = 0.0f64;
        let mut epoch_correct = 0i64;
        let mut epoch_total = 0i64;

        let perm = if config.shuffle {
            Tensor::randperm(num_train, (Kind::Int64, device))
        } else {
            Tensor::arange(num_train, (Kind::Int64, device))
        };

        let shuffled_features = train_features.index_select(0, &perm);
        let shuffled_labels = train_labels.index_select(0, &perm);

        for batch_idx in 0..num_batches {
            if train_control.is_cancelled() {
                return Err(LabError::TrainingFailed("Training cancelled".to_string()));
            }
            train_control.wait_while_paused();

            let start = batch_idx * batch_size;
            let end = std::cmp::min(start + batch_size, num_train);

            let batch_x = shuffled_features.narrow(0, start, end - start);
            let batch_y = shuffled_labels.narrow(0, start, end - start);

            let output = model.forward_train(&batch_x);
            let loss = output.cross_entropy_for_logits(&batch_y);

            epoch_loss += loss.double_value(&[]) * (end - start) as f64;
            epoch_correct += output.argmax(-1, false).eq_tensor(&batch_y).sum(tch::Kind::Int64).double_value(&[]) as i64;
            epoch_total += end - start;

            optimizer.backward_step(&loss);
        }

        let avg_train_loss = epoch_loss / epoch_total as f64;
        let train_acc = epoch_correct as f64 / epoch_total as f64;

        let val_output = model.forward_infer(&val_features);
        let val_loss = val_output.cross_entropy_for_logits(&val_labels).double_value(&[]);
        let val_acc = val_output.argmax(-1, false).eq_tensor(&val_labels).sum(tch::Kind::Float).double_value(&[])
            / val_labels.size()[0] as f64;

        let actual_epoch = epoch_offset + epoch + 1;

        event_bus.emit(LabEvent::EpochCompleted {
            session_id: session_id.clone(),
            epoch: actual_epoch,
            total_epochs: epoch_offset + config.epochs,
            train_loss: avg_train_loss,
            val_loss: Some(val_loss),
            metrics: serde_json::json!({
                "train_acc": train_acc,
                "val_acc": val_acc,
            }),
        });

        if let Some(interval) = config.checkpoint_interval {
            if actual_epoch % interval == 0 {
                let ckpt_path = format!("{}/checkpoint_epoch_{}.ot", artifact_dir, actual_epoch);
                vs.save(&ckpt_path)
                    .map_err(|e| LabError::Custom(format!("Failed to save checkpoint: {}", e)))?;
            }
        }

        if let Some(ref es_config) = config.early_stopping {
            if val_loss < best_val_loss - es_config.min_delta {
                best_val_loss = val_loss;
                patience_counter = 0;
            } else {
                patience_counter += 1;
                if patience_counter >= es_config.patience {
                    break;
                }
            }
        }
    }

    let final_path = format!("{}/model_final.ot", artifact_dir);
    vs.save(&final_path)
        .map_err(|e| LabError::Custom(format!("Failed to save final model: {}", e)))?;

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TchInferenceResult {
    pub predictions: Vec<f32>,
    pub predicted_classes: Vec<usize>,
    pub probabilities: Vec<Vec<f32>>,
}

pub fn run_tch_inference(
    config: &TrainingConfig,
    artifact_dir: &str,
    input_data: &[Vec<f32>],
) -> crate::core::Result<TchInferenceResult> {
    let metadata = load_tch_metadata(artifact_dir)
        .ok_or_else(|| LabError::InferenceFailed("No model_metadata.json found".to_string()))?;

    let device = select_device(config);
    let mut vs = nn::VarStore::new(device);

    let model_id = metadata.get("model_id")
        .and_then(|v| v.as_str())
        .unwrap_or("mlp");

    let is_classification = metadata.get("is_classification")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let default_model_path = format!("{}/model_final.ot", artifact_dir);
    let model_path = metadata.get("model_path")
        .and_then(|v| v.as_str())
        .unwrap_or(&default_model_path);

    vs.load(model_path)
        .map_err(|e| LabError::InferenceFailed(format!("Failed to load model: {}", e)))?;

    let batch_size = input_data.len();

    if model_id == "cnn" {
        let input_channels = metadata.get("input_channels")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as i64;
        let input_height = metadata.get("input_height")
            .and_then(|v| v.as_u64())
            .unwrap_or(28) as i64;
        let input_width = metadata.get("input_width")
            .and_then(|v| v.as_u64())
            .unwrap_or(28) as i64;
        let num_classes = metadata.get("num_classes")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as i64;

        let model = TchCnn::new(&vs.root(), input_channels, input_height, input_width, num_classes);

        let image_size = (input_channels * input_height * input_width) as usize;
        let mut all_features = Vec::with_capacity(batch_size * image_size);
        for row in input_data {
            if row.len() < image_size {
                let mut padded = row.clone();
                padded.resize(image_size, 0.0);
                all_features.extend_from_slice(&padded);
            } else {
                all_features.extend_from_slice(&row[..image_size]);
            }
        }

        let input_tensor = Tensor::from_slice(&all_features)
            .reshape(&[batch_size as i64, input_channels, input_height, input_width])
            .to(device);

        let output = tch::no_grad(|| model.forward_train(&input_tensor));

        tch_output_to_result(&output, batch_size, num_classes as usize, is_classification)
    } else {
        let num_features = metadata.get("num_features")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| input_data.first().map(|r| r.len() as u64).unwrap_or(1)) as usize;
        let num_classes = metadata.get("num_classes")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        let hidden_sizes: Vec<usize> = metadata.get("hidden_sizes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect())
            .unwrap_or_else(|| {
                if num_features <= 10 { vec![64, 32] }
                else if num_features <= 100 { vec![128, 64] }
                else { vec![256, 128, 64] }
            });

        let model = TchMlp::new(&vs.root(), num_features, hidden_sizes, num_classes);

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

        let input_tensor = Tensor::from_slice(&all_features)
            .reshape(&[batch_size as i64, num_features as i64])
            .to(device);

        let output = tch::no_grad(|| model.forward_train(&input_tensor));

        tch_output_to_result(&output, batch_size, num_classes, is_classification)
    }
}

fn tch_output_to_result(
    output: &Tensor,
    batch_size: usize,
    num_classes: usize,
    is_classification: bool,
) -> crate::core::Result<TchInferenceResult> {
    let output_data: Vec<f32> = output.try_into()
        .map_err(|e| LabError::InferenceFailed(format!("Failed to read output: {:?}", e)))?;

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
                probs.push(((val - max_val).exp() / exp_sum).max(0.0).min(1.0));
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

    Ok(TchInferenceResult { predictions, predicted_classes, probabilities })
}

fn load_tch_metadata(artifact_dir: &str) -> Option<serde_json::Value> {
    let metadata_path = std::path::Path::new(artifact_dir).join("model_metadata.json");
    if metadata_path.exists() {
        let content = std::fs::read_to_string(&metadata_path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

fn run_cnn_training(
    event_bus: Arc<EventBus>,
    session_id: SessionId,
    config: &TrainingConfig,
    artifact_dir: &str,
    device: Device,
    train_control: Arc<TrainControl>,
) -> crate::core::Result<()> {
    let input_channels = config.custom_params.get("input_channels")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as i64;
    let input_height = config.custom_params.get("input_height")
        .and_then(|v| v.as_u64())
        .unwrap_or(28) as i64;
    let input_width = config.custom_params.get("input_width")
        .and_then(|v| v.as_u64())
        .unwrap_or(28) as i64;

    let (features_all, labels_all) = if config.data_path.is_empty() {
        return Err(LabError::DataLoadFailed("No data path specified for tch-rs CNN training".to_string()));
    } else {
        let target_column = if config.target_columns.is_empty() {
            "label".to_string()
        } else {
            config.target_columns[0].clone()
        };
        let delimiter = config.custom_params.get("delimiter")
            .and_then(|v| v.as_str())
            .and_then(|s| s.chars().next())
            .unwrap_or(',') as u8;
        load_csv_data(&config.data_path, &config.feature_columns, &target_column, true, delimiter)?
    };

    let num_classes = labels_all.iter().map(|l| *l as usize).max().unwrap_or(0) + 1;

    event_bus.emit(LabEvent::DataLoaded {
        session_id: session_id.clone(),
        rows: features_all.len(),
        columns: features_all[0].len(),
    });

    let total = features_all.len();
    let train_end = ((total as f64) * (1.0 - config.validation_split)) as usize;

    let flat_features: Vec<f32> = features_all.iter().flatten().copied().collect();
    let all_features = Tensor::from_slice(&flat_features)
        .reshape(&[total as i64, input_channels, input_height, input_width])
        .to(device);

    let labels_i64: Vec<i64> = labels_all.iter().map(|l| *l as i64).collect();
    let labels_tensor = Tensor::from_slice(&labels_i64).to(device);

    let perm = Tensor::randperm(total as i64, (Kind::Int64, device));
    let all_features = all_features.index_select(0, &perm);
    let labels_tensor = labels_tensor.index_select(0, &perm);

    let train_features = all_features.narrow(0, 0, train_end as i64);
    let train_labels = labels_tensor.narrow(0, 0, train_end as i64);
    let val_features = all_features.narrow(0, train_end as i64, (total - train_end) as i64);
    let val_labels = labels_tensor.narrow(0, train_end as i64, (total - train_end) as i64);

    let vs = nn::VarStore::new(device);
    let model = TchCnn::new(&vs.root(), input_channels, input_height, input_width, num_classes as i64);

    let mut optimizer = build_optimizer(&vs, config)?;

    let batch_size = config.batch_size as i64;
    let num_train = train_end as i64;
    let num_batches = (num_train + batch_size - 1) / batch_size;

    let mut best_val_loss = f64::MAX;
    let mut patience_counter = 0usize;

    for epoch in 0..config.epochs {
        if train_control.is_cancelled() {
            return Err(LabError::TrainingFailed("Training cancelled".to_string()));
        }
        train_control.wait_while_paused();

        let lr = get_lr_for_epoch(config, epoch);
        optimizer.set_lr(lr);

        let mut epoch_loss = 0.0f64;
        let mut epoch_correct = 0i64;
        let mut epoch_total = 0i64;

        let perm = if config.shuffle {
            Tensor::randperm(num_train, (Kind::Int64, device))
        } else {
            Tensor::arange(num_train, (Kind::Int64, device))
        };

        let shuffled_features = train_features.index_select(0, &perm);
        let shuffled_labels = train_labels.index_select(0, &perm);

        for batch_idx in 0..num_batches {
            if train_control.is_cancelled() {
                return Err(LabError::TrainingFailed("Training cancelled".to_string()));
            }
            train_control.wait_while_paused();

            let start = batch_idx * batch_size;
            let end = std::cmp::min(start + batch_size, num_train);

            let batch_x = shuffled_features.narrow(0, start, end - start);
            let batch_y = shuffled_labels.narrow(0, start, end - start);

            let output = model.forward_train(&batch_x);
            let loss = output.cross_entropy_for_logits(&batch_y);

            epoch_loss += loss.double_value(&[]) * (end - start) as f64;
            epoch_correct += output.argmax(-1, false).eq_tensor(&batch_y).sum(tch::Kind::Int64).double_value(&[]) as i64;
            epoch_total += end - start;

            optimizer.backward_step(&loss);

            if batch_idx % 10 == 0 || batch_idx == num_batches - 1 {
                event_bus.emit(LabEvent::BatchCompleted {
                    session_id: session_id.clone(),
                    batch: (batch_idx + 1) as usize,
                    total_batches: num_batches as usize,
                    loss: loss.double_value(&[]),
                });
            }
        }

        let avg_train_loss = epoch_loss / epoch_total as f64;
        let train_acc = epoch_correct as f64 / epoch_total as f64;

        let val_output = model.forward_infer(&val_features);
        let val_loss = val_output.cross_entropy_for_logits(&val_labels).double_value(&[]);
        let val_acc = val_output.argmax(-1, false).eq_tensor(&val_labels).sum(tch::Kind::Float).double_value(&[])
            / val_labels.size()[0] as f64;

        let progress_msg = format!(
            "Epoch {}/{} - train_loss: {:.4} - train_acc: {:.1}% - val_loss: {:.4} - val_acc: {:.1}%",
            epoch + 1, config.epochs, avg_train_loss, train_acc * 100.0, val_loss, val_acc * 100.0
        );

        event_bus.emit(LabEvent::EpochCompleted {
            session_id: session_id.clone(),
            epoch: epoch + 1,
            total_epochs: config.epochs,
            train_loss: avg_train_loss,
            val_loss: Some(val_loss),
            metrics: serde_json::json!({
                "train_acc": train_acc,
                "val_acc": val_acc,
            }),
        });

        event_bus.emit(LabEvent::ProgressUpdate {
            session_id: session_id.clone(),
            progress: (epoch + 1) as f64 / config.epochs as f64,
            message: progress_msg.clone(),
        });

        event_bus.emit(LabEvent::LogOutput {
            session_id: session_id.clone(),
            level: "info".to_string(),
            message: progress_msg,
        });

        if let Some(interval) = config.checkpoint_interval {
            if (epoch + 1) % interval == 0 {
                let ckpt_path = format!("{}/checkpoint_epoch_{}.ot", artifact_dir, epoch + 1);
                vs.save(&ckpt_path)
                    .map_err(|e| LabError::Custom(format!("Failed to save checkpoint: {}", e)))?;
                event_bus.emit(LabEvent::CheckpointSaved {
                    session_id: session_id.clone(),
                    path: ckpt_path,
                    epoch: epoch + 1,
                });
            }
        }

        if let Some(ref es_config) = config.early_stopping {
            if val_loss < best_val_loss - es_config.min_delta {
                best_val_loss = val_loss;
                patience_counter = 0;
            } else {
                patience_counter += 1;
                if patience_counter >= es_config.patience {
                    event_bus.emit(LabEvent::LogOutput {
                        session_id: session_id.clone(),
                        level: "info".to_string(),
                        message: format!("Early stopping triggered at epoch {}", epoch + 1),
                    });
                    break;
                }
            }
        }
    }

    let final_path = format!("{}/model_final.ot", artifact_dir);
    vs.save(&final_path)
        .map_err(|e| LabError::Custom(format!("Failed to save final model: {}", e)))?;

    let metadata = serde_json::json!({
        "engine": "tch-rs",
        "model_id": "cnn",
        "input_channels": input_channels,
        "input_height": input_height,
        "input_width": input_width,
        "num_classes": num_classes,
        "task_type": config.task_type,
        "model_path": final_path,
    });
    let metadata_path = std::path::Path::new(artifact_dir).join("model_metadata.json");
    if let Err(e) = std::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap_or_default()) {
        crate::infrastructure::log("TCH", &format!("Failed to write model metadata: {}", e), None);
    }

    Ok(())
}
