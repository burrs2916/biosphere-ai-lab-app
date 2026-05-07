use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::core::config::TrainingConfig;
use crate::domain::experiment::aggregate::ExperimentId;
use crate::domain::model::aggregate::{ModelId, ModelRegistration, ModelStatus};
use crate::domain::model::repository::ModelRepository;
use crate::engine::registry::EngineRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeRequest {
    pub model_id: String,
    pub inputs: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeResponse {
    pub model_id: String,
    pub model_name: String,
    pub model_version: String,
    pub predictions: Vec<f32>,
    pub predicted_classes: Vec<usize>,
    pub probabilities: Vec<Vec<f32>>,
    pub latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeEndpoint {
    pub model_id: String,
    pub model_name: String,
    pub model_version: String,
    pub status: String,
    pub url: String,
    pub input_shape: Vec<i64>,
    pub output_shape: Vec<i64>,
    pub request_count: u64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeStats {
    pub total_endpoints: usize,
    pub active_endpoints: usize,
    pub total_requests: u64,
    pub avg_latency_ms: f64,
}

struct ActiveEndpoint {
    model: ModelRegistration,
    config: TrainingConfig,
    request_count: u64,
    total_latency_ms: f64,
    last_persisted_count: u64,
}

pub struct ModelServer {
    model_repo: Arc<dyn ModelRepository>,
    engine_registry: Arc<EngineRegistry>,
    experiment_repo: Arc<dyn crate::domain::experiment::repository::ExperimentRepository>,
    endpoints: Arc<RwLock<HashMap<String, ActiveEndpoint>>>,
    db_conn: Option<Arc<std::sync::Mutex<rusqlite::Connection>>>,
}

impl ModelServer {
    pub fn new(
        model_repo: Arc<dyn ModelRepository>,
        engine_registry: Arc<EngineRegistry>,
        experiment_repo: Arc<dyn crate::domain::experiment::repository::ExperimentRepository>,
    ) -> Self {
        Self {
            model_repo,
            engine_registry,
            experiment_repo,
            endpoints: Arc::new(RwLock::new(HashMap::new())),
            db_conn: None,
        }
    }

    pub fn with_db_conn(mut self, conn: Arc<std::sync::Mutex<rusqlite::Connection>>) -> Self {
        self.db_conn = Some(conn);
        self
    }

    pub async fn restore_deployed_models(&self) {
        let conn = match &self.db_conn {
            Some(c) => c.clone(),
            None => return,
        };

        let model_ids = {
            let guard = conn.lock().unwrap_or_else(|e| e.into_inner());
            let mut stmt = match guard.prepare("SELECT model_id FROM serving_endpoints WHERE status = 'active'") {
                Ok(s) => s,
                Err(e) => {
                    crate::infrastructure::log("SERVE", &format!("Failed to query serving_endpoints: {}", e), None);
                    return;
                }
            };
            let rows: Vec<String> = match stmt.query_map([], |row| row.get(0)) {
                Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
                Err(e) => {
                    crate::infrastructure::log("SERVE", &format!("Failed to map serving rows: {}", e), None);
                    vec![]
                }
            };
            rows
        };

        for model_id in model_ids {
            if self.is_deployed(&model_id).await {
                continue;
            }
            match self.deploy(&model_id).await {
                Ok(_) => {
                    crate::infrastructure::log("SERVE", &format!("Restored deployed model: {}", model_id), None);
                }
                Err(e) => {
                    crate::infrastructure::log("SERVE", &format!("Failed to restore model {}: {}", model_id, e), None);
                    if let Some(conn) = &self.db_conn {
                        let guard = conn.lock().unwrap_or_else(|e| e.into_inner());
                        let _ = guard.execute(
                            "UPDATE serving_endpoints SET status = 'failed' WHERE model_id = ?1",
                            [&model_id],
                        );
                    }
                }
            }
        }
    }

    fn persist_deploy(&self, model_id: &str) {
        if let Some(conn) = &self.db_conn {
            let guard = conn.lock().unwrap_or_else(|e| e.into_inner());
            let now = chrono::Utc::now().to_rfc3339();
            let _ = guard.execute(
                "INSERT OR REPLACE INTO serving_endpoints (model_id, deployed_at, request_count, total_latency_ms, status) VALUES (?1, ?2, 0, 0.0, 'active')",
                rusqlite::params![model_id, now],
            );
        }
    }

    fn persist_undeploy(&self, model_id: &str) {
        if let Some(conn) = &self.db_conn {
            let guard = conn.lock().unwrap_or_else(|e| e.into_inner());
            let _ = guard.execute(
                "UPDATE serving_endpoints SET status = 'undeployed' WHERE model_id = ?1",
                [&model_id],
            );
        }
    }

    fn persist_stats(&self, model_id: &str, request_count: u64, total_latency_ms: f64) {
        if let Some(conn) = &self.db_conn {
            let guard = conn.lock().unwrap_or_else(|e| e.into_inner());
            let _ = guard.execute(
                "UPDATE serving_endpoints SET request_count = ?1, total_latency_ms = ?2 WHERE model_id = ?3 AND status = 'active'",
                rusqlite::params![request_count as i64, total_latency_ms, model_id],
            );
        }
    }

    pub async fn deploy(&self, model_id: &str) -> Result<(), String> {
        let id = ModelId::from_str(model_id);
        let model = self.model_repo.load(&id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Model not found: {}", model_id))?;

        if model.status != ModelStatus::Production && model.status != ModelStatus::Staging {
            return Err(format!("Model must be in Staging or Production status to deploy, current: {}", model.status));
        }

        let experiment_id_str = model.metadata.get("experiment_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Model has no associated experiment_id in metadata".to_string())?;

        let exp_id = ExperimentId::from_str(experiment_id_str);
        let experiment = self.experiment_repo.load(&exp_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Experiment not found: {}", experiment_id_str))?;

        {
            let mut endpoints = self.endpoints.write().await;
            endpoints.insert(model_id.to_string(), ActiveEndpoint {
                model,
                config: experiment.config,
                request_count: 0,
                total_latency_ms: 0.0,
                last_persisted_count: 0,
            });
        }

        self.persist_deploy(model_id);

        crate::infrastructure::log("SERVE", &format!("Model deployed: {}", model_id), None);
        Ok(())
    }

    pub async fn undeploy(&self, model_id: &str) -> Result<(), String> {
        let mut endpoints = self.endpoints.write().await;
        if endpoints.remove(model_id).is_some() {
            self.persist_undeploy(model_id);
            crate::infrastructure::log("SERVE", &format!("Model undeployed: {}", model_id), None);
            Ok(())
        } else {
            Err(format!("Model {} is not deployed", model_id))
        }
    }

    pub async fn predict(&self, request: ServeRequest) -> Result<ServeResponse, String> {
        if request.inputs.is_empty() {
            return Err("Input data cannot be empty".to_string());
        }
        if request.inputs.len() > 1024 {
            return Err(format!("Batch size {} exceeds maximum of 1024", request.inputs.len()));
        }
        for (i, row) in request.inputs.iter().enumerate() {
            if row.is_empty() {
                return Err(format!("Input row {} is empty", i));
            }
            if row.iter().any(|v| v.is_nan() || v.is_infinite()) {
                return Err(format!("Input row {} contains NaN or infinite values", i));
            }
        }

        let start = std::time::Instant::now();

        let (model_info, config_clone, experiment_id_str, signature) = {
            let endpoints = self.endpoints.read().await;
            let endpoint = endpoints.get(&request.model_id)
                .ok_or_else(|| format!("Model {} is not deployed. Deploy it first.", request.model_id))?;

            let model = &endpoint.model;
            let model_info = (model.id.to_string(), model.name.clone(), model.version.clone());
            let experiment_id_str = model.metadata.get("experiment_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let sig = model.structured_signature.clone();
            (model_info, endpoint.config.clone(), experiment_id_str, sig)
        };

        if let Some(ref sig) = signature {
            if let Some(input_spec) = sig.inputs.first() {
                let expected_features = input_spec.shape.iter().skip(1).product::<i64>() as usize;
                if expected_features > 0 {
                    for (i, row) in request.inputs.iter().enumerate() {
                        if row.len() != expected_features {
                            return Err(format!(
                                "Input row {} has {} features, but model expects {} (signature: {:?})",
                                i, row.len(), expected_features, input_spec
                            ));
                        }
                    }
                }
                if input_spec.dtype != "float32" {
                    return Err(format!(
                        "Model expects input dtype '{}', but 'float32' was provided",
                        input_spec.dtype
                    ));
                }
            }
        }

        let engine = self.engine_registry.find_by_id_str(&config_clone.engine_id)
            .await
            .ok_or_else(|| format!("Engine '{}' not found", config_clone.engine_id))?;

        let artifact_dir = crate::core::config::get_artifact_dir(&experiment_id_str);

        let inference_result = engine.run_inference(&config_clone, &artifact_dir, &request.inputs)
            .await
            .map_err(|e| format!("Inference failed: {}", e))?;

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        {
            let mut endpoints = self.endpoints.write().await;
            if let Some(ep) = endpoints.get_mut(&request.model_id) {
                ep.request_count += 1;
                ep.total_latency_ms += latency_ms;
                if ep.request_count - ep.last_persisted_count >= 10 {
                    self.persist_stats(&request.model_id, ep.request_count, ep.total_latency_ms);
                    ep.last_persisted_count = ep.request_count;
                }
            }
        }

        Ok(ServeResponse {
            model_id: model_info.0,
            model_name: model_info.1,
            model_version: model_info.2,
            predictions: inference_result.predictions,
            predicted_classes: inference_result.predicted_classes,
            probabilities: inference_result.probabilities,
            latency_ms,
        })
    }

    pub async fn list_endpoints(&self) -> Vec<ServeEndpoint> {
        let endpoints = self.endpoints.read().await;
        endpoints.values().map(|ep| {
            let (input_shape, output_shape) = if let Some(ref sig) = ep.model.structured_signature {
                let inp = sig.inputs.first().map(|s| s.shape.clone()).unwrap_or_default();
                let out = sig.outputs.first().map(|s| s.shape.clone()).unwrap_or_default();
                (inp, out)
            } else {
                (vec![], vec![])
            };

            ServeEndpoint {
                model_id: ep.model.id.to_string(),
                model_name: ep.model.name.clone(),
                model_version: ep.model.version.clone(),
                status: format!("{}", ep.model.status),
                url: format!("/api/v1/models/{}/predict", ep.model.id),
                input_shape,
                output_shape,
                request_count: ep.request_count,
                avg_latency_ms: if ep.request_count > 0 {
                    ep.total_latency_ms / ep.request_count as f64
                } else {
                    0.0
                },
            }
        }).collect()
    }

    pub async fn get_stats(&self) -> ServeStats {
        let endpoints = self.endpoints.read().await;
        let total_endpoints = endpoints.len();
        let active_endpoints = endpoints.values()
            .filter(|ep| ep.model.status == ModelStatus::Production)
            .count();
        let total_requests: u64 = endpoints.values().map(|ep| ep.request_count).sum();
        let total_latency: f64 = endpoints.values().map(|ep| ep.total_latency_ms).sum();
        let avg_latency_ms = if total_requests > 0 {
            total_latency / total_requests as f64
        } else {
            0.0
        };

        ServeStats {
            total_endpoints,
            active_endpoints,
            total_requests,
            avg_latency_ms,
        }
    }

    pub async fn is_deployed(&self, model_id: &str) -> bool {
        let endpoints = self.endpoints.read().await;
        endpoints.contains_key(model_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serve_request_serialization() {
        let req = ServeRequest {
            model_id: "test-model".to_string(),
            inputs: vec![vec![1.0, 2.0, 3.0]],
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: ServeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.model_id, "test-model");
        assert_eq!(decoded.inputs.len(), 1);
    }

    #[test]
    fn test_serve_response_serialization() {
        let resp = ServeResponse {
            model_id: "m1".to_string(),
            model_name: "test".to_string(),
            model_version: "1.0.0".to_string(),
            predictions: vec![0.9],
            predicted_classes: vec![1],
            probabilities: vec![vec![0.1, 0.9]],
            latency_ms: 5.3,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: ServeResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.model_name, "test");
        assert_eq!(decoded.latency_ms, 5.3);
    }

    #[test]
    fn test_serve_stats_default() {
        let stats = ServeStats {
            total_endpoints: 0,
            active_endpoints: 0,
            total_requests: 0,
            avg_latency_ms: 0.0,
        };
        assert_eq!(stats.total_endpoints, 0);
        assert_eq!(stats.avg_latency_ms, 0.0);
    }
}
