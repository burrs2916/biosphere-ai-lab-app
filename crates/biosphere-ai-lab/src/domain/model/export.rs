use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::core::config::TrainingConfig;
use crate::infrastructure::log;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExportFormat {
    TorchScript,
    Onnx,
    BurnRecord,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TorchScript => write!(f, "torchscript"),
            Self::Onnx => write!(f, "onnx"),
            Self::BurnRecord => write!(f, "burn_record"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub experiment_id: String,
    pub format: ExportFormat,
    pub output_path: Option<String>,
    pub opset_version: Option<i64>,
    pub input_shapes: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    pub format: ExportFormat,
    pub output_path: String,
    pub file_size_bytes: u64,
    pub message: String,
}

pub fn get_default_export_dir(experiment_id: &str) -> String {
    let artifact_dir = crate::core::config::get_artifact_dir(experiment_id);
    format!("{}/exports", artifact_dir)
}

pub fn get_export_filename(experiment_id: &str, format: &ExportFormat) -> String {
    let ext = match format {
        ExportFormat::TorchScript => "pt",
        ExportFormat::Onnx => "onnx",
        ExportFormat::BurnRecord => "bin",
    };
    format!("{}_model.{}", experiment_id, ext)
}

fn find_latest_checkpoint(artifact_dir: &str) -> Option<(i64, String)> {
    let artifact_path = Path::new(artifact_dir);
    if !artifact_path.exists() {
        log("EXPORT", &format!("Artifact dir does not exist: {}", artifact_dir), None);
        return None;
    }

    let model_final_path = artifact_path.join("model.mpk");
    if model_final_path.exists() {
        log("EXPORT", &format!("Found final model.mpk: {}", model_final_path.display()), None);
        return Some((0, model_final_path.to_string_lossy().to_string()));
    }

    let checkpoint_dir = format!("{}/checkpoint", artifact_dir);
    if Path::new(&checkpoint_dir).exists() {
        log("EXPORT", &format!("Searching in checkpoint dir: {}", checkpoint_dir), None);
        if let Ok(entries) = std::fs::read_dir(&checkpoint_dir) {
            let mut latest_epoch: usize = 0;
            let mut latest_path: Option<String> = None;

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() { continue; }
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if !name.starts_with("model-") { continue; }

                let epoch_result = name
                    .strip_prefix("model-")
                    .and_then(|s| {
                        s.strip_suffix(".mpk")
                            .or_else(|| s.strip_suffix(".mpk.gz"))
                            .or_else(|| s.strip_suffix(".bin"))
                    })
                    .and_then(|s| s.parse::<usize>().ok());

                if let Some(epoch) = epoch_result {
                    if epoch >= latest_epoch {
                        latest_epoch = epoch;
                        latest_path = Some(path.to_string_lossy().to_string());
                    }
                }
            }

            if let Some(cp_path) = latest_path {
                log("EXPORT", &format!("Found latest checkpoint: epoch={}, path={}", latest_epoch, cp_path), None);
                return Some((latest_epoch as i64, cp_path));
            }
        }
    }

    let checkpoints_dir = format!("{}/checkpoints", artifact_dir);
    if Path::new(&checkpoints_dir).exists() {
        log("EXPORT", &format!("Searching in checkpoints dir: {}", checkpoints_dir), None);
        if let Ok(entries) = std::fs::read_dir(&checkpoints_dir) {
            let mut latest: Option<(i64, String)> = None;

            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let epoch = name
                    .strip_prefix("checkpoint-epoch-")
                    .and_then(|s| {
                        s.strip_suffix(".pt")
                            .or_else(|| s.strip_suffix(".ot"))
                            .or_else(|| s.strip_suffix(".bin"))
                    })
                    .and_then(|s| s.parse::<i64>().ok());

                if let Some(epoch) = epoch {
                    match &latest {
                        None => latest = Some((epoch, entry.path().to_string_lossy().to_string())),
                        Some((prev, _)) if epoch > *prev => {
                            latest = Some((epoch, entry.path().to_string_lossy().to_string()));
                        }
                        _ => {}
                    }
                }
            }

            if latest.is_some() {
                return latest;
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(artifact_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() { continue; }
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("model.") || name.starts_with("checkpoint-epoch-") || name.ends_with(".mpk") || name.ends_with(".ot") || name.ends_with(".bin") {
                log("EXPORT", &format!("Found model file in root: {}", path.display()), None);
                return Some((0, path.to_string_lossy().to_string()));
            }
        }
    }

    log("EXPORT", "No checkpoint or model file found", None);
    None
}

#[cfg(feature = "tch-engine")]
pub fn export_tch_model(
    experiment_id: &str,
    config: &TrainingConfig,
    format: &ExportFormat,
    output_dir: &str,
    input_shapes: &[Vec<i64>],
) -> ExportResult {
    let artifact_dir = crate::core::config::get_artifact_dir(experiment_id);
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        return ExportResult {
            success: false,
            format: format.clone(),
            output_path: String::new(),
            file_size_bytes: 0,
            message: format!("Failed to create output directory: {}", e),
        };
    }

    let checkpoint = match find_latest_checkpoint(&artifact_dir) {
        Some(c) => c,
        None => {
            return ExportResult {
                success: false,
                format: format.clone(),
                output_path: String::new(),
                file_size_bytes: 0,
                message: "No checkpoints found".to_string(),
            };
        }
    };

    match format {
        ExportFormat::TorchScript => {
            export_tch_torchscript(experiment_id, config, &artifact_dir, output_dir, input_shapes, &checkpoint)
        }
        ExportFormat::Onnx => {
            export_tch_onnx(experiment_id, config, &artifact_dir, output_dir, input_shapes, &checkpoint)
        }
        ExportFormat::BurnRecord => {
            ExportResult {
                success: false,
                format: format.clone(),
                output_path: String::new(),
                file_size_bytes: 0,
                message: "BurnRecord format not supported for tch engine".to_string(),
            }
        }
    }
}

#[cfg(feature = "tch-engine")]
fn export_tch_torchscript(
    experiment_id: &str,
    config: &TrainingConfig,
    _artifact_dir: &str,
    output_dir: &str,
    input_shapes: &[Vec<i64>],
    checkpoint: &(i64, String),
) -> ExportResult {
    use tch::{nn, Device};

    let output_path = format!("{}/{}", output_dir, get_export_filename(experiment_id, &ExportFormat::TorchScript));

    let device = Device::Cpu;
    let mut vs = nn::VarStore::new(device);

    let num_features = config.feature_columns.len().max(1);
    let num_outputs = config.target_columns.len().max(1);

    let model = crate::engine::tch_engine::create_model_for_export(
        &config.model_id,
        &vs.root(),
        num_features,
        num_outputs,
        config.batch_size as i64,
    );

    if let Err(e) = vs.load_partial(&checkpoint.1) {
        return ExportResult {
            success: false,
            format: ExportFormat::TorchScript,
            output_path,
            file_size_bytes: 0,
            message: format!("Failed to load checkpoint: {}", e),
        };
    }

    log("EXPORT", &format!("Loaded checkpoint from epoch {}", checkpoint.0), None);

    let default_input = if input_shapes.is_empty() {
        vec![vec![1, num_features as i64]]
    } else {
        input_shapes.to_vec()
    };

    let input_tensor = tch::Tensor::randn(&default_input[0], (tch::Kind::Float, device));
    let _warmup_output = model.forward_t(&input_tensor, false);

    if let Err(e) = vs.save(&output_path) {
        ExportResult {
            success: false,
            format: ExportFormat::TorchScript,
            output_path,
            file_size_bytes: 0,
            message: format!("Failed to save model weights: {}", e),
        }
    } else {
        let file_size = std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);
        log("EXPORT", &format!("TorchScript weights exported to {}", output_path), None);
        ExportResult {
            success: true,
            format: ExportFormat::TorchScript,
            output_path,
            file_size_bytes: file_size,
            message: format!("Model weights exported from epoch {}", checkpoint.0),
        }
    }
}

#[cfg(feature = "tch-engine")]
fn export_tch_onnx(
    experiment_id: &str,
    config: &TrainingConfig,
    _artifact_dir: &str,
    output_dir: &str,
    input_shapes: &[Vec<i64>],
    checkpoint: &(i64, String),
) -> ExportResult {
    use tch::{nn, Device};

    let output_path = format!("{}/{}", output_dir, get_export_filename(experiment_id, &ExportFormat::Onnx));

    let device = Device::Cpu;
    let mut vs = nn::VarStore::new(device);

    let num_features = config.feature_columns.len().max(1);
    let num_outputs = config.target_columns.len().max(1);

    let model = crate::engine::tch_engine::create_model_for_export(
        &config.model_id,
        &vs.root(),
        num_features,
        num_outputs,
        config.batch_size as i64,
    );

    if let Err(e) = vs.load_partial(&checkpoint.1) {
        return ExportResult {
            success: false,
            format: ExportFormat::Onnx,
            output_path,
            file_size_bytes: 0,
            message: format!("Failed to load checkpoint: {}", e),
        };
    }

    log("EXPORT", &format!("Loaded checkpoint from epoch {}", checkpoint.0), None);

    let default_input = if input_shapes.is_empty() {
        vec![config.batch_size as i64, num_features as i64]
    } else {
        input_shapes[0].clone()
    };

    let input_tensor = tch::Tensor::randn(&default_input, (tch::Kind::Float, device));

    let ts_path = format!("{}/{}", output_dir, get_export_filename(experiment_id, &ExportFormat::TorchScript));

    let model_ref = &model;
    match tch::CModule::create_by_tracing("export", "forward", &[input_tensor], &mut |inputs| {
        let output = model_ref.forward_t(&inputs[0], false);
        vec![output]
    }) {
        Ok(module) => {
            if let Err(e) = module.save(&ts_path) {
                return ExportResult {
                    success: false,
                    format: ExportFormat::Onnx,
                    output_path: ts_path,
                    file_size_bytes: 0,
                    message: format!("Traced model but failed to save: {}", e),
                };
            }
            let file_size = std::fs::metadata(&ts_path).map(|m| m.len()).unwrap_or(0);
            log("EXPORT", &format!("TorchScript traced model exported to {}", ts_path), None);
            ExportResult {
                success: true,
                format: ExportFormat::Onnx,
                output_path: ts_path,
                file_size_bytes: file_size,
                message: format!("Exported as TorchScript trace (use Python torch.onnx.export to convert to ONNX). Epoch {}", checkpoint.0),
            }
        }
        Err(e) => {
            log("EXPORT", &format!("TorchScript trace failed: {}, falling back to weight save", e), None);
            match vs.save(&ts_path) {
                Ok(_) => {
                    let file_size = std::fs::metadata(&ts_path).map(|m| m.len()).unwrap_or(0);
                    ExportResult {
                        success: true,
                        format: ExportFormat::TorchScript,
                        output_path: ts_path,
                        file_size_bytes: file_size,
                        message: format!("Trace failed ({}), saved weights instead. Use Python to convert to ONNX.", e),
                    }
                }
                Err(e2) => {
                    ExportResult {
                        success: false,
                        format: ExportFormat::Onnx,
                        output_path,
                        file_size_bytes: 0,
                        message: format!("Both trace and weight save failed: {}, {}", e, e2),
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "tch-engine"))]
pub fn export_tch_model(
    _experiment_id: &str,
    _config: &TrainingConfig,
    _format: &ExportFormat,
    _output_dir: &str,
    _input_shapes: &[Vec<i64>],
) -> ExportResult {
    ExportResult {
        success: false,
        format: _format.clone(),
        output_path: String::new(),
        file_size_bytes: 0,
        message: "tch-engine feature not enabled".to_string(),
    }
}

pub fn export_burn_model(
    experiment_id: &str,
    output_dir: &str,
) -> ExportResult {
    let artifact_dir = crate::core::config::get_artifact_dir(experiment_id);
    let output_path = format!("{}/{}", output_dir, get_export_filename(experiment_id, &ExportFormat::BurnRecord));

    log("EXPORT", &format!("export_burn_model: artifact_dir='{}', output_dir='{}', output_path='{}'", artifact_dir, output_dir, output_path), None);

    if let Err(e) = std::fs::create_dir_all(output_dir) {
        return ExportResult {
            success: false,
            format: ExportFormat::BurnRecord,
            output_path,
            file_size_bytes: 0,
            message: format!("Failed to create output directory: {}", e),
        };
    }

    let checkpoint = match find_latest_checkpoint(&artifact_dir) {
        Some(c) => {
            log("EXPORT", &format!("Found checkpoint: epoch={}, path='{}'", c.0, c.1), None);
            c
        }
        None => {
            log("EXPORT", &format!("No checkpoint found in artifact_dir: '{}'", artifact_dir), None);
            return ExportResult {
                success: false,
                format: ExportFormat::BurnRecord,
                output_path,
                file_size_bytes: 0,
                message: format!("No checkpoints found for Burn model in: {}", artifact_dir),
            };
        }
    };

    match std::fs::copy(&checkpoint.1, &output_path) {
        Ok(bytes) => {
            log("EXPORT", &format!("Burn record exported from epoch {} to {}", checkpoint.0, output_path), None);
            ExportResult {
                success: true,
                format: ExportFormat::BurnRecord,
                output_path,
                file_size_bytes: bytes,
                message: format!("Burn record exported from epoch {} ({})", checkpoint.0, checkpoint.1),
            }
        }
        Err(e) => {
            log("EXPORT", &format!("Failed to copy checkpoint: {} -> {}, error: {}", checkpoint.1, output_path, e), None);
            ExportResult {
                success: false,
                format: ExportFormat::BurnRecord,
                output_path,
                file_size_bytes: 0,
                message: format!("Failed to copy checkpoint: {}", e),
            }
        }
    }
}

pub fn export_model(request: &ExportRequest, config: &TrainingConfig) -> ExportResult {
    let output_dir = request.output_path.clone()
        .unwrap_or_else(|| get_default_export_dir(&request.experiment_id));

    let artifact_dir = crate::core::config::get_artifact_dir(&request.experiment_id);
    log("EXPORT", &format!(
        "Exporting model: experiment={}, engine={}, format={}, output_dir={}, artifact_dir={}",
        request.experiment_id, config.engine_id, request.format, output_dir, artifact_dir
    ), None);

    match config.engine_id.as_str() {
        "tch" => export_tch_model(
            &request.experiment_id,
            config,
            &request.format,
            &output_dir,
            &request.input_shapes,
        ),
        "burn" => {
            match &request.format {
                ExportFormat::BurnRecord => export_burn_model(
                    &request.experiment_id,
                    &output_dir,
                ),
                ExportFormat::TorchScript | ExportFormat::Onnx => {
                    log("EXPORT", &format!("{} export not supported for Burn engine", request.format), None);
                    ExportResult {
                        success: false,
                        format: request.format.clone(),
                        output_path: String::new(),
                        file_size_bytes: 0,
                        message: format!("{} export not supported for Burn engine. Use BurnRecord format.", request.format),
                    }
                }
            }
        }
        _ => {
            log("EXPORT", &format!("Unknown engine: {}", config.engine_id), None);
            ExportResult {
                success: false,
                format: request.format.clone(),
                output_path: String::new(),
                file_size_bytes: 0,
                message: format!("Unknown engine: {}", config.engine_id),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_display() {
        assert_eq!(ExportFormat::TorchScript.to_string(), "torchscript");
        assert_eq!(ExportFormat::Onnx.to_string(), "onnx");
        assert_eq!(ExportFormat::BurnRecord.to_string(), "burn_record");
    }

    #[test]
    fn test_get_export_filename() {
        assert_eq!(get_export_filename("exp-1", &ExportFormat::Onnx), "exp-1_model.onnx");
        assert_eq!(get_export_filename("exp-1", &ExportFormat::TorchScript), "exp-1_model.pt");
        assert_eq!(get_export_filename("exp-1", &ExportFormat::BurnRecord), "exp-1_model.bin");
    }

    #[test]
    fn test_get_default_export_dir() {
        let dir = get_default_export_dir("test-exp");
        assert!(dir.contains("test-exp"));
        assert!(dir.contains("exports"));
    }

    #[test]
    fn test_export_request_serialization() {
        let request = ExportRequest {
            experiment_id: "exp-1".to_string(),
            format: ExportFormat::Onnx,
            output_path: Some("/tmp/export".to_string()),
            opset_version: Some(14),
            input_shapes: vec![vec![1, 3, 224, 224]],
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ExportRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.experiment_id, "exp-1");
        assert_eq!(deserialized.format, ExportFormat::Onnx);
        assert_eq!(deserialized.opset_version, Some(14));
    }

    #[test]
    fn test_export_result_serialization() {
        let result = ExportResult {
            success: true,
            format: ExportFormat::TorchScript,
            output_path: "/tmp/model.pt".to_string(),
            file_size_bytes: 1024,
            message: "OK".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ExportResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.file_size_bytes, 1024);
    }

    #[test]
    fn test_find_latest_checkpoint_no_dir() {
        let result = find_latest_checkpoint("/nonexistent/path");
        assert!(result.is_none());
    }
}
