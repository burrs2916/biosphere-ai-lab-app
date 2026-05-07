use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::config::TrainingConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HparamValue {
    Float(f64),
    Int(i64),
    String(String),
}

impl HparamValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            HparamValue::Float(v) => Some(*v),
            HparamValue::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            HparamValue::Int(v) => Some(*v),
            HparamValue::Float(v) => Some(*v as i64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            HparamValue::String(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HparamRange {
    FloatRange { min: f64, max: f64 },
    IntRange { min: i64, max: i64 },
    Choice(Vec<HparamValue>),
}

impl HparamRange {
    pub fn float(min: f64, max: f64) -> Self {
        Self::FloatRange { min, max }
    }

    pub fn int(min: i64, max: i64) -> Self {
        Self::IntRange { min, max }
    }

    pub fn choice(values: Vec<HparamValue>) -> Self {
        Self::Choice(values)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HparamSpace {
    pub params: HashMap<String, HparamRange>,
}

impl HparamSpace {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    pub fn add_float(mut self, name: &str, min: f64, max: f64) -> Self {
        self.params.insert(name.to_string(), HparamRange::float(min, max));
        self
    }

    pub fn add_int(mut self, name: &str, min: i64, max: i64) -> Self {
        self.params.insert(name.to_string(), HparamRange::int(min, max));
        self
    }

    pub fn add_choice(mut self, name: &str, values: Vec<HparamValue>) -> Self {
        self.params.insert(name.to_string(), HparamRange::choice(values));
        self
    }

    pub fn grid_combinations(&self) -> Vec<HashMap<String, HparamValue>> {
        let keys: Vec<&String> = self.params.keys().collect();
        if keys.is_empty() {
            return vec![HashMap::new()];
        }

        const MAX_GRID_COMBINATIONS: usize = 1000;

        let mut all_values: Vec<Vec<(String, HparamValue)>> = Vec::new();
        let mut total_combinations: usize = 1;
        for key in &keys {
            let range = &self.params[*key];
            let values: Vec<(String, HparamValue)> = match range {
                HparamRange::FloatRange { min, max } => {
                    let step = (max - min) / 9.0;
                    (0..10)
                        .map(|i| {
                            let v = min + step * i as f64;
                            (key.to_string(), HparamValue::Float(v))
                        })
                        .collect()
                }
                HparamRange::IntRange { min, max } => {
                    let step = ((*max - *min) as f64 / 9.0).ceil() as i64;
                    let count = if step == 0 { 1 } else { ((max - min) / step + 1) as usize };
                    (0..count.min(10))
                        .map(|i| {
                            let v = min + step * i as i64;
                            (key.to_string(), HparamValue::Int(v.min(*max)))
                        })
                        .collect()
                }
                HparamRange::Choice(choices) => {
                    choices
                        .iter()
                        .map(|v| (key.to_string(), v.clone()))
                        .collect()
                }
            };
            total_combinations = total_combinations.saturating_mul(values.len());
            all_values.push(values);
        }

        if total_combinations > MAX_GRID_COMBINATIONS {
            return self.random_combinations(MAX_GRID_COMBINATIONS);
        }

        let mut results = vec![HashMap::new()];
        for values in all_values {
            let mut new_results = Vec::new();
            for existing in &results {
                for (key, value) in &values {
                    let mut combo = existing.clone();
                    combo.insert(key.clone(), value.clone());
                    new_results.push(combo);
                }
            }
            results = new_results;
        }
        results
    }

    fn random_combination_with_seed(&self, seed: u64) -> HashMap<String, HparamValue> {
        let mut combo = HashMap::new();

        for (i, (key, range)) in self.params.iter().enumerate() {
            let value = match range {
                HparamRange::FloatRange { min, max } => {
                    let hash = simple_hash(seed, i as u64);
                    let t = (hash as f64) / (u64::MAX as f64);
                    HparamValue::Float(min + t * (max - min))
                }
                HparamRange::IntRange { min, max } => {
                    let hash = simple_hash(seed, i as u64);
                    let range = (*max - *min) as u64;
                    let offset = if range > 0 { hash % range } else { 0 };
                    HparamValue::Int(min + offset as i64)
                }
                HparamRange::Choice(choices) => {
                    if choices.is_empty() {
                        continue;
                    }
                    let hash = simple_hash(seed, i as u64);
                    let idx = (hash as usize) % choices.len();
                    choices[idx].clone()
                }
            };
            combo.insert(key.clone(), value);
        }
        combo
    }

    pub fn random_combinations(&self, n: usize) -> Vec<HashMap<String, HparamValue>> {
        let base_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        (0..n).map(|i| self.random_combination_with_seed(simple_hash(base_seed, i as u64))).collect()
    }
}

fn simple_hash(seed: u64, idx: u64) -> u64 {
    let mut h = seed.wrapping_add(idx.wrapping_mul(0x9e3779b97f4a7c15));
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    h
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TuneStrategy {
    Grid,
    Random { n_trials: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuneConfig {
    pub base_config: TrainingConfig,
    pub hparam_space: HparamSpace,
    pub strategy: TuneStrategy,
    pub metric_to_optimize: String,
    pub maximize: bool,
    pub max_concurrent: usize,
}

impl TuneConfig {
    pub fn grid(base_config: TrainingConfig, hparam_space: HparamSpace, metric: &str, maximize: bool) -> Self {
        Self {
            base_config,
            hparam_space,
            strategy: TuneStrategy::Grid,
            metric_to_optimize: metric.to_string(),
            maximize,
            max_concurrent: 1,
        }
    }

    pub fn random(base_config: TrainingConfig, hparam_space: HparamSpace, n_trials: usize, metric: &str, maximize: bool) -> Self {
        Self {
            base_config,
            hparam_space,
            strategy: TuneStrategy::Random { n_trials },
            metric_to_optimize: metric.to_string(),
            maximize,
            max_concurrent: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialResult {
    pub experiment_id: String,
    pub params: HashMap<String, HparamValue>,
    pub metric_value: Option<f64>,
    pub status: TrialStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrialStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuneResult {
    pub tune_id: String,
    pub strategy: TuneStrategy,
    pub trials: Vec<TrialResult>,
    pub best_trial: Option<TrialResult>,
    pub best_params: Option<HashMap<String, HparamValue>>,
}

impl TuneResult {
    pub fn new(tune_id: String, strategy: TuneStrategy) -> Self {
        Self {
            tune_id,
            strategy,
            trials: Vec::new(),
            best_trial: None,
            best_params: None,
        }
    }

    pub fn update_best(&mut self, maximize: bool) {
        let best = self.trials.iter()
            .filter(|t| t.status == TrialStatus::Completed && t.metric_value.is_some())
            .fold(None, |acc: Option<&TrialResult>, t| {
                match acc {
                    None => Some(t),
                    Some(prev) => {
                        let prev_val = prev.metric_value.unwrap();
                        let cur_val = t.metric_value.unwrap();
                        if maximize && cur_val > prev_val {
                            Some(t)
                        } else if !maximize && cur_val < prev_val {
                            Some(t)
                        } else {
                            Some(prev)
                        }
                    }
                }
            });

        if let Some(b) = best {
            self.best_trial = Some(b.clone());
            self.best_params = Some(b.params.clone());
        }
    }
}

pub fn apply_params_to_config(config: &TrainingConfig, params: &HashMap<String, HparamValue>) -> TrainingConfig {
    let mut new_config = config.clone();

    for (key, value) in params {
        match key.as_str() {
            "learning_rate" | "lr" => {
                if let Some(v) = value.as_f64() {
                    new_config.learning_rate = v;
                }
            }
            "batch_size" => {
                if let Some(v) = value.as_i64() {
                    new_config.batch_size = v as usize;
                }
            }
            "epochs" => {
                if let Some(v) = value.as_i64() {
                    new_config.epochs = v as usize;
                }
            }
            "optimizer" => {
                if let Some(v) = value.as_str() {
                    new_config.optimizer = match v {
                        "sgd" => crate::core::config::OptimizerConfig::Sgd {
                            momentum: Some(0.9),
                            weight_decay: None,
                        },
                        "adamw" => crate::core::config::OptimizerConfig::AdamW {
                            beta1: 0.9,
                            beta2: 0.999,
                            weight_decay: 0.01,
                        },
                        "rmsprop" => crate::core::config::OptimizerConfig::Rmsprop {
                            alpha: 0.99,
                            weight_decay: None,
                        },
                        _ => crate::core::config::OptimizerConfig::Adam {
                            beta1: 0.9,
                            beta2: 0.999,
                            weight_decay: None,
                        },
                    };
                }
            }
            "weight_decay" => {
                if let Some(v) = value.as_f64() {
                    match &mut new_config.optimizer {
                        crate::core::config::OptimizerConfig::Adam { weight_decay, .. } => {
                            *weight_decay = Some(v);
                        }
                        crate::core::config::OptimizerConfig::AdamW { weight_decay, .. } => {
                            *weight_decay = v;
                        }
                        crate::core::config::OptimizerConfig::Sgd { weight_decay, .. } => {
                            *weight_decay = Some(v);
                        }
                        crate::core::config::OptimizerConfig::Rmsprop { weight_decay, .. } => {
                            *weight_decay = Some(v);
                        }
                        crate::core::config::OptimizerConfig::Custom { params, .. } => {
                            params.insert("weight_decay".to_string(), serde_json::to_value(v).unwrap_or_default());
                        }
                    }
                }
            }
            "momentum" => {
                if let Some(v) = value.as_f64() {
                    match &mut new_config.optimizer {
                        crate::core::config::OptimizerConfig::Sgd { momentum, .. } => {
                            *momentum = Some(v);
                        }
                        crate::core::config::OptimizerConfig::Custom { params, .. } => {
                            params.insert("momentum".to_string(), serde_json::to_value(v).unwrap_or_default());
                        }
                        _ => {}
                    }
                }
            }
            "validation_split" => {
                if let Some(v) = value.as_f64() {
                    new_config.validation_split = v;
                }
            }
            _ => {
                new_config.custom_params.insert(key.clone(), serde_json::to_value(value).unwrap_or_default());
            }
        }
    }

    new_config
}

pub fn generate_trials(config: &TuneConfig) -> Vec<HashMap<String, HparamValue>> {
    match &config.strategy {
        TuneStrategy::Grid => config.hparam_space.grid_combinations(),
        TuneStrategy::Random { n_trials } => config.hparam_space.random_combinations(*n_trials),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hparam_space_grid() {
        let space = HparamSpace::new()
            .add_choice("lr", vec![
                HparamValue::Float(0.001),
                HparamValue::Float(0.01),
            ])
            .add_choice("batch_size", vec![
                HparamValue::Int(16),
                HparamValue::Int(32),
            ]);

        let combos = space.grid_combinations();
        assert_eq!(combos.len(), 4);
    }

    #[test]
    fn test_hparam_space_random() {
        let space = HparamSpace::new()
            .add_float("lr", 0.0001, 0.1)
            .add_int("batch_size", 16, 128);

        let combos = space.random_combinations(5);
        assert_eq!(combos.len(), 5);

        for combo in &combos {
            assert!(combo.contains_key("lr"));
            assert!(combo.contains_key("batch_size"));
            let lr = combo["lr"].as_f64().unwrap();
            assert!(lr >= 0.0001 && lr <= 0.1);
        }
    }

    #[test]
    fn test_apply_params_to_config() {
        let config = TrainingConfig::default();
        let params: HashMap<String, HparamValue> = [
            ("learning_rate".to_string(), HparamValue::Float(0.01)),
            ("batch_size".to_string(), HparamValue::Int(64)),
        ].into_iter().collect();

        let new_config = apply_params_to_config(&config, &params);
        assert_eq!(new_config.learning_rate, 0.01);
        assert_eq!(new_config.batch_size, 64);
    }

    #[test]
    fn test_tune_result_update_best_minimize() {
        let mut result = TuneResult::new("tune-1".to_string(), TuneStrategy::Grid);
        result.trials.push(TrialResult {
            experiment_id: "exp-1".to_string(),
            params: HashMap::new(),
            metric_value: Some(0.5),
            status: TrialStatus::Completed,
        });
        result.trials.push(TrialResult {
            experiment_id: "exp-2".to_string(),
            params: HashMap::new(),
            metric_value: Some(0.3),
            status: TrialStatus::Completed,
        });

        result.update_best(false);
        assert_eq!(result.best_trial.unwrap().metric_value, Some(0.3));
    }

    #[test]
    fn test_tune_result_update_best_maximize() {
        let mut result = TuneResult::new("tune-1".to_string(), TuneStrategy::Grid);
        result.trials.push(TrialResult {
            experiment_id: "exp-1".to_string(),
            params: HashMap::new(),
            metric_value: Some(0.8),
            status: TrialStatus::Completed,
        });
        result.trials.push(TrialResult {
            experiment_id: "exp-2".to_string(),
            params: HashMap::new(),
            metric_value: Some(0.95),
            status: TrialStatus::Completed,
        });

        result.update_best(true);
        assert_eq!(result.best_trial.unwrap().metric_value, Some(0.95));
    }

    #[test]
    fn test_tune_result_ignores_failed() {
        let mut result = TuneResult::new("tune-1".to_string(), TuneStrategy::Grid);
        result.trials.push(TrialResult {
            experiment_id: "exp-1".to_string(),
            params: HashMap::new(),
            metric_value: Some(0.5),
            status: TrialStatus::Failed,
        });
        result.trials.push(TrialResult {
            experiment_id: "exp-2".to_string(),
            params: HashMap::new(),
            metric_value: Some(0.3),
            status: TrialStatus::Completed,
        });

        result.update_best(false);
        assert_eq!(result.best_trial.unwrap().metric_value, Some(0.3));
    }

    #[test]
    fn test_hparam_value_conversions() {
        let fv = HparamValue::Float(3.14);
        assert_eq!(fv.as_f64(), Some(3.14));
        assert_eq!(fv.as_i64(), Some(3));

        let iv = HparamValue::Int(42);
        assert_eq!(iv.as_i64(), Some(42));
        assert_eq!(iv.as_f64(), Some(42.0));

        let sv = HparamValue::String("adam".to_string());
        assert_eq!(sv.as_str(), Some("adam"));
        assert_eq!(sv.as_f64(), None);
    }

    #[test]
    fn test_generate_trials_grid() {
        let space = HparamSpace::new()
            .add_choice("lr", vec![HparamValue::Float(0.001), HparamValue::Float(0.01)]);

        let config = TuneConfig::grid(
            TrainingConfig::default(),
            space,
            "val_loss",
            false,
        );

        let trials = generate_trials(&config);
        assert_eq!(trials.len(), 2);
    }

    #[test]
    fn test_generate_trials_random() {
        let space = HparamSpace::new()
            .add_float("lr", 0.0001, 0.1);

        let config = TuneConfig::random(
            TrainingConfig::default(),
            space,
            5,
            "accuracy",
            true,
        );

        let trials = generate_trials(&config);
        assert_eq!(trials.len(), 5);
    }
}
