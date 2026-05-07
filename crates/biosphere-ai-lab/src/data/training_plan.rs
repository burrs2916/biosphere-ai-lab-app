use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::data_recipe::{DataRecipe, QualityThresholds};
use super::global_dedup::GlobalDedupConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingPlan {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub plan_type: PlanType,
    pub phases: Vec<PlanPhase>,
    pub data_budget: DataBudget,
    pub quality_gates: Vec<QualityGate>,
    pub dedup_config: Option<GlobalDedupConfig>,
    pub preprocessing: Option<PreprocessingConfig>,
    pub validation: Option<ValidationConfig>,
    pub experiment_tracking: Option<ExperimentTracking>,
    pub output_dir: PathBuf,
    pub seed: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for TrainingPlan {
    fn default() -> Self {
        Self {
            name: "default_plan".to_string(),
            version: "1.0".to_string(),
            description: None,
            plan_type: PlanType::Pretraining,
            phases: Vec::new(),
            data_budget: DataBudget::default(),
            quality_gates: Vec::new(),
            dedup_config: None,
            preprocessing: None,
            validation: None,
            experiment_tracking: None,
            output_dir: PathBuf::from("./training_output"),
            seed: 42,
            metadata: HashMap::new(),
        }
    }
}

impl TrainingPlan {
    pub fn validate(&self) -> Result<PlanValidationResult, String> {
        let mut result = PlanValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            checks: Vec::new(),
        };

        if self.name.trim().is_empty() {
            result.errors.push("Plan name is empty".to_string());
        }

        if self.phases.is_empty() {
            result.errors.push("Plan has no phases defined".to_string());
        }

        for (i, phase) in self.phases.iter().enumerate() {
            match phase.validate() {
                Ok(check_results) => {
                    for check in check_results {
                        if check.status == CheckStatus::Failed {
                            result.errors.push(format!("Phase {}: {}", i, check.message));
                        } else if check.status == CheckStatus::Warning {
                            result.warnings.push(format!("Phase {}: {}", i, check.message));
                        }
                        result.checks.push(check);
                    }
                }
                Err(e) => {
                    result.errors.push(format!("Phase {} validation error: {}", i, e));
                }
            }
        }

        if let Err(e) = self.data_budget.validate() {
            result.errors.push(format!("Budget error: {}", e));
        }

        for gate in &self.quality_gates {
            if let Err(e) = gate.validate() {
                result.errors.push(format!("Quality gate error: {}", e));
            }
        }

        result.is_valid = result.errors.is_empty();

        Ok(result)
    }

    pub fn total_estimated_tokens(&self) -> u64 {
        self.phases.iter()
            .map(|p| p.recipe.total_samples_target.unwrap_or(0) as u64)
            .sum()
    }

    pub fn total_estimated_steps(&self) -> u64 {
        self.phases.iter()
            .map(|p| p.training_steps.unwrap_or(0) as u64)
            .sum()
    }

    pub fn phase_count(&self) -> usize {
        self.phases.len()
    }

    pub fn dataset_count(&self) -> usize {
        self.phases.iter()
            .map(|p| p.recipe.datasets.len())
            .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanType {
    Pretraining,
    FineTuning,
    SFT,
    RLHF,
    DPO,
    ContinuedPretraining,
    InstructionTuning,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPhase {
    pub name: String,
    pub phase_type: PhaseType,
    pub recipe: DataRecipe,
    pub training_steps: Option<usize>,
    pub learning_rate: Option<f64>,
    pub batch_size: Option<usize>,
    pub sequence_length: Option<usize>,
    pub gradient_accumulation_steps: Option<usize>,
    pub description: Option<String>,
    pub depends_on: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PlanPhase {
    pub fn validate(&self) -> Result<Vec<PlanCheck>, String> {
        let mut checks = Vec::new();

        if self.name.trim().is_empty() {
            checks.push(PlanCheck {
                check_name: "phase_name".to_string(),
                status: CheckStatus::Failed,
                message: "Phase name is empty".to_string(),
                details: None,
            });
        }

        match self.recipe.validate() {
            Ok(()) => {
                checks.push(PlanCheck {
                    check_name: "recipe_valid".to_string(),
                    status: CheckStatus::Passed,
                    message: format!("Recipe '{}' is valid", self.recipe.name),
                    details: Some(serde_json::json!({
                        "datasets": self.recipe.datasets.len(),
                        "total_weight": self.recipe.total_weight(),
                    })),
                });
            }
            Err(e) => {
                checks.push(PlanCheck {
                    check_name: "recipe_valid".to_string(),
                    status: CheckStatus::Failed,
                    message: format!("Recipe validation failed: {}", e),
                    details: None,
                });
            }
        }

        if let Some(steps) = self.training_steps {
            if steps == 0 {
                checks.push(PlanCheck {
                    check_name: "training_steps".to_string(),
                    status: CheckStatus::Failed,
                    message: "Training steps is zero".to_string(),
                    details: None,
                });
            }
        } else {
            checks.push(PlanCheck {
                check_name: "training_steps".to_string(),
                status: CheckStatus::Warning,
                message: "Training steps not specified".to_string(),
                details: None,
            });
        }

        if let Some(lr) = self.learning_rate {
            if lr <= 0.0 {
                checks.push(PlanCheck {
                    check_name: "learning_rate".to_string(),
                    status: CheckStatus::Failed,
                    message: "Learning rate must be positive".to_string(),
                    details: None,
                });
            }
        }

        Ok(checks)
    }

    pub fn estimated_tokens(&self) -> u64 {
        let samples = self.recipe.total_samples_target.unwrap_or(0) as u64;
        let seq_len = self.sequence_length.unwrap_or(2048) as u64;
        samples * seq_len
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseType {
    Warmup,
    Main,
    Annealing,
    Cooldown,
    Evaluation,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBudget {
    pub total_tokens_target: u64,
    pub total_samples_target: Option<usize>,
    pub max_epochs: Option<usize>,
    pub tokens_per_sample_estimate: Option<u64>,
    pub budget_type: BudgetType,
    pub cost_estimate: Option<CostEstimate>,
}

impl Default for DataBudget {
    fn default() -> Self {
        Self {
            total_tokens_target: 1_000_000_000,
            total_samples_target: None,
            max_epochs: Some(10),
            tokens_per_sample_estimate: Some(2048),
            budget_type: BudgetType::Tokens,
            cost_estimate: None,
        }
    }
}

impl DataBudget {
    pub fn validate(&self) -> Result<(), String> {
        if self.total_tokens_target == 0 {
            return Err("Total tokens target is zero".to_string());
        }
        if let Some(epochs) = self.max_epochs {
            if epochs == 0 {
                return Err("Max epochs is zero".to_string());
            }
        }
        Ok(())
    }

    pub fn estimated_samples(&self) -> u64 {
        let tps = self.tokens_per_sample_estimate.unwrap_or(2048);
        self.total_tokens_target / tps
    }

    pub fn estimated_gpu_hours(&self, gpu_type: &str) -> f64 {
        let tokens = self.total_tokens_target as f64;
        let efficiency = match gpu_type {
            "A100-80GB" => 3.0e12,
            "H100" => 6.0e12,
            "A100-40GB" => 2.5e12,
            "V100" => 1.0e12,
            _ => 2.0e12,
        };
        tokens / efficiency * 3600.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BudgetType {
    Tokens,
    Samples,
    Epochs,
    Steps,
    TimeBased,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub gpu_type: String,
    pub gpu_count: usize,
    pub cost_per_gpu_hour: f64,
    pub estimated_gpu_hours: f64,
    pub estimated_total_cost: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGate {
    pub name: String,
    pub gate_type: QualityGateType,
    pub threshold: f64,
    pub action: GateAction,
    pub description: Option<String>,
}

impl QualityGate {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Quality gate name is empty".to_string());
        }
        if self.threshold < 0.0 || self.threshold > 1.0 {
            return Err(format!(
                "Quality gate '{}' threshold must be in [0, 1]",
                self.name
            ));
        }
        Ok(())
    }

    pub fn check(&self, value: f64) -> GateResult {
        let passed = match self.gate_type {
            QualityGateType::Minimum => value >= self.threshold,
            QualityGateType::Maximum => value <= self.threshold,
            QualityGateType::Range { min: _, max: _ } => {
                if let QualityGateType::Range { min, max } = &self.gate_type {
                    value >= *min && value <= *max
                } else {
                    false
                }
            }
        };

        GateResult {
            gate_name: self.name.clone(),
            passed,
            actual_value: value,
            threshold: self.threshold,
            action: if passed { GateAction::Proceed } else { self.action.clone() },
            message: if passed {
                format!("Gate '{}' passed: {} >= {}", self.name, value, self.threshold)
            } else {
                format!("Gate '{}' failed: {} < {}", self.name, value, self.threshold)
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QualityGateType {
    Minimum,
    Maximum,
    Range { min: f64, max: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GateAction {
    Proceed,
    Warn,
    Abort,
    Skip,
    Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub actual_value: f64,
    pub threshold: f64,
    pub action: GateAction,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingConfig {
    pub tokenizer_name: String,
    pub max_sequence_length: usize,
    pub truncation_strategy: TruncationStrategy,
    pub padding_strategy: PaddingStrategy,
    pub add_special_tokens: bool,
    pub strip_whitespace: bool,
    pub lowercase: bool,
    pub remove_urls: bool,
    pub remove_emails: bool,
    pub remove_phone_numbers: bool,
    pub normalize_unicode: bool,
    pub min_text_length: usize,
    pub max_text_length: usize,
}

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            tokenizer_name: "default".to_string(),
            max_sequence_length: 2048,
            truncation_strategy: TruncationStrategy::TruncateEnd,
            padding_strategy: PaddingStrategy::Longest,
            add_special_tokens: true,
            strip_whitespace: true,
            lowercase: false,
            remove_urls: true,
            remove_emails: true,
            remove_phone_numbers: false,
            normalize_unicode: true,
            min_text_length: 10,
            max_text_length: 100000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TruncationStrategy {
    TruncateStart,
    TruncateEnd,
    TruncateMiddle,
    NoTruncation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaddingStrategy {
    Longest,
    MaxLength,
    NoPadding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub validation_split_ratio: f64,
    pub validation_metrics: Vec<String>,
    pub eval_every_n_steps: usize,
    pub early_stopping_patience: Option<usize>,
    pub save_best_only: bool,
    pub metric_for_best: Option<String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            validation_split_ratio: 0.05,
            validation_metrics: vec!["loss".to_string(), "perplexity".to_string()],
            eval_every_n_steps: 1000,
            early_stopping_patience: Some(5),
            save_best_only: true,
            metric_for_best: Some("loss".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentTracking {
    pub experiment_name: String,
    pub tags: Vec<String>,
    pub log_metrics: bool,
    pub log_artifacts: bool,
    pub log_model_checkpoints: bool,
    pub tracking_backend: TrackingBackend,
    pub project_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrackingBackend {
    Local,
    MLflow,
    WandB,
    TensorBoard,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub checks: Vec<PlanCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanCheck {
    pub check_name: String,
    pub status: CheckStatus,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    Warning,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecutionState {
    pub plan_name: String,
    pub current_phase: usize,
    pub total_phases: usize,
    pub current_step: usize,
    pub phase_steps: Vec<usize>,
    pub status: PlanStatus,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub metrics: HashMap<String, Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Validated,
    Running,
    Paused,
    Completed,
    Failed(String),
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub name: String,
    pub plan_type: PlanType,
    pub total_phases: usize,
    pub total_datasets: usize,
    pub total_estimated_tokens: u64,
    pub total_estimated_steps: u64,
    pub quality_gates: usize,
    pub has_dedup: bool,
    pub has_preprocessing: bool,
    pub has_validation: bool,
    pub estimated_gpu_hours: Option<f64>,
}

impl TrainingPlan {
    pub fn summarize(&self) -> PlanSummary {
        let gpu_hours = self.data_budget.cost_estimate.as_ref()
            .map(|c| c.estimated_gpu_hours);

        PlanSummary {
            name: self.name.clone(),
            plan_type: self.plan_type.clone(),
            total_phases: self.phases.len(),
            total_datasets: self.dataset_count(),
            total_estimated_tokens: self.total_estimated_tokens(),
            total_estimated_steps: self.total_estimated_steps(),
            quality_gates: self.quality_gates.len(),
            has_dedup: self.dedup_config.is_some(),
            has_preprocessing: self.preprocessing.is_some(),
            has_validation: self.validation.is_some(),
            estimated_gpu_hours: gpu_hours,
        }
    }
}

pub struct PlanBuilder {
    plan: TrainingPlan,
}

impl PlanBuilder {
    pub fn new(name: &str, plan_type: PlanType) -> Self {
        Self {
            plan: TrainingPlan {
                name: name.to_string(),
                plan_type,
                ..Default::default()
            },
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.plan.description = Some(desc.to_string());
        self
    }

    pub fn add_phase(mut self, phase: PlanPhase) -> Self {
        self.plan.phases.push(phase);
        self
    }

    pub fn with_budget(mut self, budget: DataBudget) -> Self {
        self.plan.data_budget = budget;
        self
    }

    pub fn add_quality_gate(mut self, gate: QualityGate) -> Self {
        self.plan.quality_gates.push(gate);
        self
    }

    pub fn with_dedup(mut self, config: GlobalDedupConfig) -> Self {
        self.plan.dedup_config = Some(config);
        self
    }

    pub fn with_preprocessing(mut self, config: PreprocessingConfig) -> Self {
        self.plan.preprocessing = Some(config);
        self
    }

    pub fn with_validation(mut self, config: ValidationConfig) -> Self {
        self.plan.validation = Some(config);
        self
    }

    pub fn with_experiment_tracking(mut self, tracking: ExperimentTracking) -> Self {
        self.plan.experiment_tracking = Some(tracking);
        self
    }

    pub fn output_dir(mut self, dir: PathBuf) -> Self {
        self.plan.output_dir = dir;
        self
    }

    pub fn seed(mut self, seed: u64) -> Self {
        self.plan.seed = seed;
        self
    }

    pub fn metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.plan.metadata.insert(key.to_string(), value);
        self
    }

    pub fn build(self) -> Result<TrainingPlan, String> {
        let result = self.plan.validate()?;
        if !result.is_valid {
            return Err(format!(
                "Plan validation failed: {}",
                result.errors.join("; ")
            ));
        }
        Ok(self.plan)
    }
}

pub fn create_standard_llm_pretraining_plan() -> TrainingPlan {
    use super::data_recipe::create_llm_pretraining_recipe;

    TrainingPlan {
        name: "standard_llm_pretraining".to_string(),
        version: "1.0".to_string(),
        description: Some("Standard LLM pretraining plan with curriculum learning".to_string()),
        plan_type: PlanType::Pretraining,
        phases: vec![
            PlanPhase {
                name: "warmup_phase".to_string(),
                phase_type: PhaseType::Warmup,
                recipe: create_llm_pretraining_recipe(),
                training_steps: Some(10000),
                learning_rate: Some(1e-4),
                batch_size: Some(512),
                sequence_length: Some(2048),
                gradient_accumulation_steps: Some(4),
                description: Some("Warmup phase with lower difficulty data".to_string()),
                depends_on: vec![],
                metadata: HashMap::new(),
            },
            PlanPhase {
                name: "main_training".to_string(),
                phase_type: PhaseType::Main,
                recipe: create_llm_pretraining_recipe(),
                training_steps: Some(90000),
                learning_rate: Some(3e-4),
                batch_size: Some(1024),
                sequence_length: Some(2048),
                gradient_accumulation_steps: Some(8),
                description: Some("Main training phase with full data mix".to_string()),
                depends_on: vec!["warmup_phase".to_string()],
                metadata: HashMap::new(),
            },
        ],
        data_budget: DataBudget {
            total_tokens_target: 200_000_000_000,
            total_samples_target: Some(100_000_000),
            max_epochs: Some(3),
            tokens_per_sample_estimate: Some(2048),
            budget_type: BudgetType::Tokens,
            cost_estimate: Some(CostEstimate {
                gpu_type: "A100-80GB".to_string(),
                gpu_count: 64,
                cost_per_gpu_hour: 2.50,
                estimated_gpu_hours: 5000.0,
                estimated_total_cost: 12500.0,
                currency: "USD".to_string(),
            }),
        },
        quality_gates: vec![
            QualityGate {
                name: "min_quality_score".to_string(),
                gate_type: QualityGateType::Minimum,
                threshold: 0.3,
                action: GateAction::Warn,
                description: Some("Minimum overall quality score".to_string()),
            },
            QualityGate {
                name: "max_toxicity".to_string(),
                gate_type: QualityGateType::Maximum,
                threshold: 0.5,
                action: GateAction::Abort,
                description: Some("Maximum toxicity threshold".to_string()),
            },
            QualityGate {
                name: "dedup_ratio".to_string(),
                gate_type: QualityGateType::Maximum,
                threshold: 0.3,
                action: GateAction::Warn,
                description: Some("Maximum acceptable dedup ratio".to_string()),
            },
        ],
        dedup_config: Some(GlobalDedupConfig::default()),
        preprocessing: Some(PreprocessingConfig::default()),
        validation: Some(ValidationConfig::default()),
        experiment_tracking: Some(ExperimentTracking {
            experiment_name: "llm_pretraining_v1".to_string(),
            tags: vec!["pretraining".to_string(), "llm".to_string(), "v1".to_string()],
            log_metrics: true,
            log_artifacts: true,
            log_model_checkpoints: true,
            tracking_backend: TrackingBackend::Local,
            project_name: Some("biosphere-llm".to_string()),
        }),
        output_dir: PathBuf::from("./training_output/llm_pretraining"),
        seed: 42,
        metadata: HashMap::new(),
    }
}

pub fn create_sft_training_plan() -> TrainingPlan {
    use super::data_recipe::create_sft_recipe;

    TrainingPlan {
        name: "sft_instruction_tuning".to_string(),
        version: "1.0".to_string(),
        description: Some("Supervised fine-tuning plan for instruction following".to_string()),
        plan_type: PlanType::SFT,
        phases: vec![
            PlanPhase {
                name: "sft_main".to_string(),
                phase_type: PhaseType::Main,
                recipe: create_sft_recipe(),
                training_steps: Some(5000),
                learning_rate: Some(2e-5),
                batch_size: Some(128),
                sequence_length: Some(2048),
                gradient_accumulation_steps: Some(2),
                description: Some("SFT training phase".to_string()),
                depends_on: vec![],
                metadata: HashMap::new(),
            },
        ],
        data_budget: DataBudget {
            total_tokens_target: 10_000_000_000,
            total_samples_target: Some(100_000),
            max_epochs: Some(3),
            tokens_per_sample_estimate: Some(512),
            budget_type: BudgetType::Samples,
            cost_estimate: Some(CostEstimate {
                gpu_type: "A100-80GB".to_string(),
                gpu_count: 8,
                cost_per_gpu_hour: 2.50,
                estimated_gpu_hours: 100.0,
                estimated_total_cost: 250.0,
                currency: "USD".to_string(),
            }),
        },
        quality_gates: vec![
            QualityGate {
                name: "high_quality_ratio".to_string(),
                gate_type: QualityGateType::Minimum,
                threshold: 0.5,
                action: GateAction::Warn,
                description: Some("Minimum ratio of high-quality samples".to_string()),
            },
        ],
        dedup_config: Some(GlobalDedupConfig {
            similarity_threshold: 0.9,
            ..Default::default()
        }),
        preprocessing: Some(PreprocessingConfig {
            max_sequence_length: 2048,
            ..Default::default()
        }),
        validation: Some(ValidationConfig {
            validation_split_ratio: 0.1,
            ..Default::default()
        }),
        experiment_tracking: Some(ExperimentTracking {
            experiment_name: "sft_v1".to_string(),
            tags: vec!["sft".to_string(), "instruction".to_string()],
            log_metrics: true,
            log_artifacts: true,
            log_model_checkpoints: true,
            tracking_backend: TrackingBackend::Local,
            project_name: Some("biosphere-sft".to_string()),
        }),
        output_dir: PathBuf::from("./training_output/sft"),
        seed: 42,
        metadata: HashMap::new(),
    }
}

pub fn create_rlhf_training_plan() -> TrainingPlan {
    use super::data_recipe::create_rlhf_preference_recipe;

    TrainingPlan {
        name: "rlhf_preference_training".to_string(),
        version: "1.0".to_string(),
        description: Some("RLHF training plan with preference optimization".to_string()),
        plan_type: PlanType::RLHF,
        phases: vec![
            PlanPhase {
                name: "reward_modeling".to_string(),
                phase_type: PhaseType::Main,
                recipe: create_rlhf_preference_recipe(),
                training_steps: Some(3000),
                learning_rate: Some(1e-5),
                batch_size: Some(64),
                sequence_length: Some(1024),
                gradient_accumulation_steps: Some(4),
                description: Some("Reward model training phase".to_string()),
                depends_on: vec![],
                metadata: HashMap::new(),
            },
        ],
        data_budget: DataBudget {
            total_tokens_target: 5_000_000_000,
            total_samples_target: Some(50_000),
            max_epochs: Some(2),
            tokens_per_sample_estimate: Some(512),
            budget_type: BudgetType::Samples,
            cost_estimate: Some(CostEstimate {
                gpu_type: "A100-80GB".to_string(),
                gpu_count: 8,
                cost_per_gpu_hour: 2.50,
                estimated_gpu_hours: 50.0,
                estimated_total_cost: 125.0,
                currency: "USD".to_string(),
            }),
        },
        quality_gates: vec![
            QualityGate {
                name: "preference_quality".to_string(),
                gate_type: QualityGateType::Minimum,
                threshold: 0.7,
                action: GateAction::Abort,
                description: Some("Minimum preference data quality".to_string()),
            },
        ],
        dedup_config: Some(GlobalDedupConfig {
            similarity_threshold: 0.95,
            ..Default::default()
        }),
        preprocessing: Some(PreprocessingConfig {
            max_sequence_length: 1024,
            ..Default::default()
        }),
        validation: Some(ValidationConfig {
            validation_split_ratio: 0.1,
            ..Default::default()
        }),
        experiment_tracking: Some(ExperimentTracking {
            experiment_name: "rlhf_v1".to_string(),
            tags: vec!["rlhf".to_string(), "preference".to_string()],
            log_metrics: true,
            log_artifacts: true,
            log_model_checkpoints: true,
            tracking_backend: TrackingBackend::Local,
            project_name: Some("biosphere-rlhf".to_string()),
        }),
        output_dir: PathBuf::from("./training_output/rlhf"),
        seed: 42,
        metadata: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_plan_validation_empty() {
        let plan = TrainingPlan::default();
        let result = plan.validate().unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_data_budget_validation() {
        let budget = DataBudget::default();
        assert!(budget.validate().is_ok());

        let bad = DataBudget {
            total_tokens_target: 0,
            ..Default::default()
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn test_data_budget_estimated_samples() {
        let budget = DataBudget {
            total_tokens_target: 1_000_000,
            tokens_per_sample_estimate: Some(1000),
            ..Default::default()
        };
        assert_eq!(budget.estimated_samples(), 1000);
    }

    #[test]
    fn test_quality_gate_minimum() {
        let gate = QualityGate {
            name: "test_gate".to_string(),
            gate_type: QualityGateType::Minimum,
            threshold: 0.5,
            action: GateAction::Abort,
            description: None,
        };

        let result = gate.check(0.6);
        assert!(result.passed);

        let result = gate.check(0.4);
        assert!(!result.passed);
    }

    #[test]
    fn test_quality_gate_maximum() {
        let gate = QualityGate {
            name: "test_gate".to_string(),
            gate_type: QualityGateType::Maximum,
            threshold: 0.5,
            action: GateAction::Warn,
            description: None,
        };

        let result = gate.check(0.3);
        assert!(result.passed);

        let result = gate.check(0.7);
        assert!(!result.passed);
    }

    #[test]
    fn test_quality_gate_range() {
        let gate = QualityGate {
            name: "test_gate".to_string(),
            gate_type: QualityGateType::Range { min: 0.3, max: 0.7 },
            threshold: 0.5,
            action: GateAction::Warn,
            description: None,
        };

        let result = gate.check(0.5);
        assert!(result.passed);

        let result = gate.check(0.1);
        assert!(!result.passed);

        let result = gate.check(0.9);
        assert!(!result.passed);
    }

    #[test]
    fn test_plan_phase_validation() {
        let phase = PlanPhase {
            name: "test_phase".to_string(),
            phase_type: PhaseType::Main,
            recipe: DataRecipe::default(),
            training_steps: Some(100),
            learning_rate: Some(1e-4),
            batch_size: Some(32),
            sequence_length: Some(512),
            gradient_accumulation_steps: None,
            description: None,
            depends_on: vec![],
            metadata: HashMap::new(),
        };

        let checks = phase.validate().unwrap();
        assert!(!checks.is_empty());
    }

    #[test]
    fn test_plan_builder() {
        use super::super::data_recipe::RecipeDataset;

        let recipe = DataRecipe {
            name: "test".to_string(),
            datasets: vec![
                RecipeDataset { name: "ds1".to_string(), weight: 1.0, ..Default::default() },
            ],
            ..Default::default()
        };

        let phase = PlanPhase {
            name: "main".to_string(),
            phase_type: PhaseType::Main,
            recipe,
            training_steps: Some(100),
            learning_rate: Some(1e-4),
            batch_size: Some(32),
            sequence_length: Some(512),
            gradient_accumulation_steps: None,
            description: None,
            depends_on: vec![],
            metadata: HashMap::new(),
        };

        let plan = PlanBuilder::new("test_plan", PlanType::Pretraining)
            .description("Test plan")
            .add_phase(phase)
            .with_budget(DataBudget::default())
            .seed(123)
            .build()
            .unwrap();

        assert_eq!(plan.name, "test_plan");
        assert_eq!(plan.phases.len(), 1);
        assert_eq!(plan.seed, 123);
    }

    #[test]
    fn test_standard_llm_pretraining_plan() {
        let plan = create_standard_llm_pretraining_plan();
        let result = plan.validate().unwrap();
        assert!(result.is_valid);
        assert_eq!(plan.phases.len(), 2);
        assert_eq!(plan.quality_gates.len(), 3);
    }

    #[test]
    fn test_sft_training_plan() {
        let plan = create_sft_training_plan();
        let result = plan.validate().unwrap();
        assert!(result.is_valid);
        assert_eq!(plan.plan_type, PlanType::SFT);
    }

    #[test]
    fn test_rlhf_training_plan() {
        let plan = create_rlhf_training_plan();
        let result = plan.validate().unwrap();
        assert!(result.is_valid);
        assert_eq!(plan.plan_type, PlanType::RLHF);
    }

    #[test]
    fn test_plan_summary() {
        let plan = create_standard_llm_pretraining_plan();
        let summary = plan.summarize();
        assert_eq!(summary.total_phases, 2);
        assert!(summary.has_dedup);
        assert!(summary.has_preprocessing);
        assert!(summary.has_validation);
    }

    #[test]
    fn test_phase_estimated_tokens() {
        let phase = PlanPhase {
            name: "test".to_string(),
            phase_type: PhaseType::Main,
            recipe: DataRecipe {
                name: "test".to_string(),
                datasets: vec![
                    super::super::data_recipe::RecipeDataset {
                        name: "ds1".to_string(),
                        weight: 1.0,
                        ..Default::default()
                    },
                ],
                total_samples_target: Some(1000),
                ..Default::default()
            },
            training_steps: Some(100),
            learning_rate: Some(1e-4),
            batch_size: Some(32),
            sequence_length: Some(2048),
            gradient_accumulation_steps: None,
            description: None,
            depends_on: vec![],
            metadata: HashMap::new(),
        };

        assert_eq!(phase.estimated_tokens(), 1000 * 2048);
    }

    #[test]
    fn test_data_budget_gpu_hours() {
        let budget = DataBudget {
            total_tokens_target: 1_000_000_000_000,
            ..Default::default()
        };

        let hours = budget.estimated_gpu_hours("A100-80GB");
        assert!(hours > 0.0);
    }
}
