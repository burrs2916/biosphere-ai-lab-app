use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecipe {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub datasets: Vec<RecipeDataset>,
    pub mixing_strategy: MixingStrategy,
    pub curriculum: Option<CurriculumConfig>,
    pub dynamic_ratio: Option<DynamicRatioConfig>,
    pub quality_thresholds: Option<QualityThresholds>,
    pub total_samples_target: Option<usize>,
    pub seed: u64,
}

impl Default for DataRecipe {
    fn default() -> Self {
        Self {
            name: "default_recipe".to_string(),
            version: "1.0".to_string(),
            description: None,
            datasets: Vec::new(),
            mixing_strategy: MixingStrategy::Proportional,
            curriculum: None,
            dynamic_ratio: None,
            quality_thresholds: None,
            total_samples_target: None,
            seed: 42,
        }
    }
}

impl DataRecipe {
    pub fn validate(&self) -> Result<(), String> {
        if self.datasets.is_empty() {
            return Err("Recipe must contain at least one dataset".to_string());
        }

        let total_weight: f64 = self.datasets.iter().map(|d| d.weight).sum();
        if total_weight <= 0.0 {
            return Err("Total dataset weight must be positive".to_string());
        }

        for (i, ds) in self.datasets.iter().enumerate() {
            if ds.name.trim().is_empty() {
                return Err(format!("Dataset {} has empty name", i));
            }
            if ds.weight < 0.0 {
                return Err(format!("Dataset '{}' has negative weight", ds.name));
            }
            if let Some(ref path) = ds.local_path {
                if path.trim().is_empty() {
                    return Err(format!("Dataset '{}' has empty local_path", ds.name));
                }
            }
        }

        if let Some(ref curriculum) = self.curriculum {
            curriculum.validate()?;
        }

        if let Some(ref dynamic) = self.dynamic_ratio {
            dynamic.validate()?;
        }

        Ok(())
    }

    pub fn normalized_weights(&self) -> Vec<f64> {
        let total: f64 = self.datasets.iter().map(|d| d.weight).sum();
        if total == 0.0 {
            return vec![1.0 / self.datasets.len() as f64; self.datasets.len()];
        }
        self.datasets.iter().map(|d| d.weight / total).collect()
    }

    pub fn total_weight(&self) -> f64 {
        self.datasets.iter().map(|d| d.weight).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeDataset {
    pub name: String,
    pub weight: f64,
    pub local_path: Option<String>,
    pub cloud_key: Option<String>,
    pub cloud_provider: Option<String>,
    pub cloud_bucket: Option<String>,
    pub format: Option<String>,
    pub max_samples: Option<usize>,
    pub min_quality_score: Option<f64>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub difficulty_range: Option<DifficultyRange>,
    pub repetition: DatasetRepetition,
}

impl Default for RecipeDataset {
    fn default() -> Self {
        Self {
            name: String::new(),
            weight: 1.0,
            local_path: None,
            cloud_key: None,
            cloud_provider: None,
            cloud_bucket: None,
            format: None,
            max_samples: None,
            min_quality_score: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
            difficulty_range: None,
            repetition: DatasetRepetition::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyRange {
    pub min_difficulty: f64,
    pub max_difficulty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRepetition {
    pub allow_repeat: bool,
    pub max_epochs: Option<usize>,
    pub shuffle_between_epochs: bool,
}

impl Default for DatasetRepetition {
    fn default() -> Self {
        Self {
            allow_repeat: true,
            max_epochs: None,
            shuffle_between_epochs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MixingStrategy {
    Proportional,
    Interleaved {
        samples_per_dataset: usize,
    },
    Stratified {
        strata_column: String,
    },
    WeightedRandom {
        temperature: f64,
    },
    Sequential {
        order: Vec<String>,
    },
    Staged {
        stages: Vec<RecipeStage>,
    },
}

impl Default for MixingStrategy {
    fn default() -> Self {
        Self::Proportional
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStage {
    pub name: String,
    pub start_step: usize,
    pub end_step: usize,
    pub dataset_weights: HashMap<String, f64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumConfig {
    pub enabled: bool,
    pub difficulty_metric: DifficultyMetric,
    pub pacing: CurriculumPacing,
    pub initial_difficulty: f64,
    pub final_difficulty: f64,
    pub warmup_steps: usize,
    pub total_steps: usize,
}

impl CurriculumConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.initial_difficulty < 0.0 || self.initial_difficulty > 1.0 {
            return Err("initial_difficulty must be in [0, 1]".to_string());
        }
        if self.final_difficulty < 0.0 || self.final_difficulty > 1.0 {
            return Err("final_difficulty must be in [0, 1]".to_string());
        }
        if self.total_steps == 0 {
            return Err("total_steps must be positive".to_string());
        }
        Ok(())
    }

    pub fn difficulty_at_step(&self, step: usize) -> f64 {
        if step < self.warmup_steps {
            return self.initial_difficulty;
        }

        let effective_step = step - self.warmup_steps;
        let total_effective = self.total_steps.saturating_sub(self.warmup_steps).max(1);

        let progress = (effective_step as f64 / total_effective as f64).min(1.0);

        match self.pacing {
            CurriculumPacing::Linear => {
                self.initial_difficulty + (self.final_difficulty - self.initial_difficulty) * progress
            }
            CurriculumPacing::Exponential { base } => {
                let factor = (base.powf(progress) - 1.0) / (base - 1.0);
                self.initial_difficulty + (self.final_difficulty - self.initial_difficulty) * factor
            }
            CurriculumPacing::Cosine => {
                let cos_val = (progress * std::f64::consts::PI).cos();
                let factor = 0.5 * (1.0 - cos_val);
                self.initial_difficulty + (self.final_difficulty - self.initial_difficulty) * factor
            }
            CurriculumPacing::Step { step_size, gamma } => {
                let num_steps = (effective_step / step_size) as f64;
                let factor = gamma.powf(num_steps);
                self.initial_difficulty + (self.final_difficulty - self.initial_difficulty) * (1.0 - factor)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifficultyMetric {
    Perplexity,
    TextLength,
    VocabularyComplexity,
    SyntacticDepth,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CurriculumPacing {
    Linear,
    Exponential { base: f64 },
    Cosine,
    Step { step_size: usize, gamma: f64 },
}

impl Default for CurriculumPacing {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicRatioConfig {
    pub enabled: bool,
    pub schedule: RatioSchedule,
    pub target_ratios: HashMap<String, f64>,
    pub annealing_steps: usize,
}

impl DynamicRatioConfig {
    pub fn validate(&self) -> Result<(), String> {
        let total: f64 = self.target_ratios.values().sum();
        if (total - 1.0).abs() > 0.01 {
            return Err(format!("Target ratios must sum to 1.0, got {}", total));
        }
        Ok(())
    }

    pub fn ratio_at_step(&self, step: usize, dataset_name: &str, initial_ratio: f64) -> f64 {
        let target = self.target_ratios.get(dataset_name).copied().unwrap_or(initial_ratio);

        if step >= self.annealing_steps {
            return target;
        }

        let progress = step as f64 / self.annealing_steps.max(1) as f64;

        match self.schedule {
            RatioSchedule::Linear => {
                initial_ratio + (target - initial_ratio) * progress
            }
            RatioSchedule::Cosine => {
                let cos_val = (progress * std::f64::consts::PI).cos();
                let factor = 0.5 * (1.0 - cos_val);
                initial_ratio + (target - initial_ratio) * factor
            }
            RatioSchedule::Exponential { rate } => {
                let factor = 1.0 - (-rate * progress).exp();
                initial_ratio + (target - initial_ratio) * factor
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatioSchedule {
    Linear,
    Cosine,
    Exponential { rate: f64 },
}

impl Default for RatioSchedule {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholds {
    pub min_overall_score: f64,
    pub max_toxicity: f64,
    pub min_language_confidence: f64,
    pub max_repetition_ratio: f64,
    pub require_no_pii: bool,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            min_overall_score: 0.3,
            max_toxicity: 0.5,
            min_language_confidence: 0.7,
            max_repetition_ratio: 0.3,
            require_no_pii: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeExecutionState {
    pub recipe_name: String,
    pub current_step: usize,
    pub total_steps: usize,
    pub dataset_cursors: HashMap<String, DatasetCursor>,
    pub current_difficulty: f64,
    pub current_ratios: HashMap<String, f64>,
    pub samples_yielded: usize,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetCursor {
    pub dataset_name: String,
    pub current_index: usize,
    pub total_available: usize,
    pub epoch: usize,
    pub exhausted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStats {
    pub recipe_name: String,
    pub total_steps: usize,
    pub current_step: usize,
    pub progress_pct: f64,
    pub samples_yielded: usize,
    pub per_dataset_samples: HashMap<String, usize>,
    pub current_difficulty: f64,
    pub current_ratios: HashMap<String, f64>,
    pub elapsed_steps: usize,
    pub estimated_remaining_steps: usize,
}

pub struct RecipeExecutor {
    recipe: DataRecipe,
    state: RecipeExecutionState,
    rng: StdRng,
}

impl RecipeExecutor {
    pub fn new(recipe: DataRecipe) -> Result<Self, String> {
        recipe.validate()?;

        let total_steps = recipe.total_samples_target.unwrap_or(usize::MAX);
        let normalized = recipe.normalized_weights();

        let mut dataset_cursors = HashMap::new();
        let mut current_ratios = HashMap::new();

        for (i, ds) in recipe.datasets.iter().enumerate() {
            dataset_cursors.insert(ds.name.clone(), DatasetCursor {
                dataset_name: ds.name.clone(),
                current_index: 0,
                total_available: ds.max_samples.unwrap_or(usize::MAX),
                epoch: 0,
                exhausted: false,
            });
            current_ratios.insert(ds.name.clone(), normalized[i]);
        }

        let initial_difficulty = recipe.curriculum.as_ref()
            .map(|c| c.initial_difficulty)
            .unwrap_or(0.0);

        Ok(Self {
            state: RecipeExecutionState {
                recipe_name: recipe.name.clone(),
                current_step: 0,
                total_steps,
                dataset_cursors,
                current_difficulty: initial_difficulty,
                current_ratios,
                samples_yielded: 0,
                is_complete: false,
            },
            recipe,
            rng: StdRng::seed_from_u64(42),
        })
    }

    pub fn next_dataset(&mut self) -> Option<String> {
        if self.state.is_complete {
            return None;
        }

        if let Some(ref target) = self.recipe.total_samples_target {
            if self.state.samples_yielded >= *target {
                self.state.is_complete = true;
                return None;
            }
        }

        let strategy = self.recipe.mixing_strategy.clone();
        let dataset_name = self.select_dataset(&strategy);

        if let Some(ref name) = dataset_name {
            if let Some(cursor) = self.state.dataset_cursors.get_mut(name) {
                cursor.current_index += 1;
                if cursor.current_index >= cursor.total_available {
                    if self.recipe.datasets.iter().any(|d| {
                        d.name == *name && d.repetition.allow_repeat
                    }) {
                        cursor.current_index = 0;
                        cursor.epoch += 1;
                        if let Some(max_epochs) = self.recipe.datasets.iter()
                            .find(|d| d.name == *name)
                            .and_then(|d| d.repetition.max_epochs)
                        {
                            if cursor.epoch >= max_epochs {
                                cursor.exhausted = true;
                            }
                        }
                    } else {
                        cursor.exhausted = true;
                    }
                }
            }
        }

        self.state.samples_yielded += 1;
        self.state.current_step += 1;

        self.update_dynamic_state();

        dataset_name
    }

    fn select_dataset(&mut self, strategy: &MixingStrategy) -> Option<String> {
        match strategy {
            MixingStrategy::Proportional => self.next_proportional(),
            MixingStrategy::Interleaved { samples_per_dataset } => {
                self.next_interleaved(*samples_per_dataset)
            }
            MixingStrategy::WeightedRandom { temperature } => {
                self.next_weighted_random(*temperature)
            }
            MixingStrategy::Sequential { order } => {
                self.next_sequential(order)
            }
            MixingStrategy::Staged { stages } => {
                self.next_staged(stages)
            }
            MixingStrategy::Stratified { .. } => {
                self.next_proportional()
            }
        }
    }

    fn next_proportional(&mut self) -> Option<String> {
        let active: Vec<&RecipeDataset> = self.recipe.datasets.iter()
            .filter(|ds| {
                self.state.dataset_cursors.get(&ds.name)
                    .map(|c| !c.exhausted)
                    .unwrap_or(false)
            })
            .collect();

        if active.is_empty() {
            self.state.is_complete = true;
            return None;
        }

        let total_weight: f64 = active.iter().map(|d| d.weight).sum();
        let mut r = self.rng.gen::<f64>() * total_weight;
        for ds in &active {
            r -= ds.weight;
            if r <= 0.0 {
                return Some(ds.name.clone());
            }
        }

        Some(active.last()?.name.clone())
    }

    fn next_interleaved(&mut self, samples_per_dataset: usize) -> Option<String> {
        let active: Vec<&RecipeDataset> = self.recipe.datasets.iter()
            .filter(|ds| {
                self.state.dataset_cursors.get(&ds.name)
                    .map(|c| !c.exhausted)
                    .unwrap_or(false)
            })
            .collect();

        if active.is_empty() {
            self.state.is_complete = true;
            return None;
        }

        let block = self.state.current_step / samples_per_dataset;
        let idx = block % active.len();
        Some(active[idx].name.clone())
    }

    fn next_weighted_random(&mut self, temperature: f64) -> Option<String> {
        let active: Vec<&RecipeDataset> = self.recipe.datasets.iter()
            .filter(|ds| {
                self.state.dataset_cursors.get(&ds.name)
                    .map(|c| !c.exhausted)
                    .unwrap_or(false)
            })
            .collect();

        if active.is_empty() {
            self.state.is_complete = true;
            return None;
        }

        let mut weights: Vec<f64> = active.iter()
            .map(|ds| (ds.weight / temperature).exp())
            .collect();

        let total: f64 = weights.iter().sum();
        for w in &mut weights {
            *w /= total;
        }

        let mut r = self.rng.gen::<f64>();
        for (i, &w) in weights.iter().enumerate() {
            r -= w;
            if r <= 0.0 {
                return Some(active[i].name.clone());
            }
        }

        Some(active.last()?.name.clone())
    }

    fn next_sequential(&mut self, order: &[String]) -> Option<String> {
        if order.is_empty() {
            return None;
        }

        let idx = self.state.current_step % order.len();
        let name = &order[idx];

        if let Some(cursor) = self.state.dataset_cursors.get(name) {
            if cursor.exhausted {
                let next_idx = (idx + 1) % order.len();
                return Some(order[next_idx].clone());
            }
        }

        Some(name.clone())
    }

    fn next_staged(&mut self, stages: &[RecipeStage]) -> Option<String> {
        let step = self.state.current_step;

        let active_stage = stages.iter()
            .find(|s| step >= s.start_step && step < s.end_step);

        if let Some(stage) = active_stage {
            let total: f64 = stage.dataset_weights.values().sum();
            if total <= 0.0 {
                return None;
            }

            let mut r = self.rng.gen::<f64>() * total;
            for (name, weight) in &stage.dataset_weights {
                r -= weight;
                if r <= 0.0 {
                    if let Some(cursor) = self.state.dataset_cursors.get(name) {
                        if !cursor.exhausted {
                            return Some(name.clone());
                        }
                    }
                }
            }

            stage.dataset_weights.keys().next().cloned()
        } else {
            self.state.is_complete = true;
            None
        }
    }

    fn update_dynamic_state(&mut self) {
        let step = self.state.current_step;

        if let Some(ref curriculum) = self.recipe.curriculum {
            if curriculum.enabled {
                self.state.current_difficulty = curriculum.difficulty_at_step(step);
            }
        }

        if let Some(ref dynamic) = self.recipe.dynamic_ratio {
            if dynamic.enabled {
                let initial_ratios = self.recipe.normalized_weights();
                for (i, ds) in self.recipe.datasets.iter().enumerate() {
                    let initial = initial_ratios[i];
                    let new_ratio = dynamic.ratio_at_step(step, &ds.name, initial);
                    self.state.current_ratios.insert(ds.name.clone(), new_ratio);
                }
            }
        }
    }

    pub fn state(&self) -> &RecipeExecutionState {
        &self.state
    }

    pub fn recipe(&self) -> &DataRecipe {
        &self.recipe
    }

    pub fn stats(&self) -> RecipeStats {
        let mut per_dataset = HashMap::new();
        for (name, cursor) in &self.state.dataset_cursors {
            per_dataset.insert(name.clone(), cursor.current_index);
        }

        let progress = if self.state.total_steps == usize::MAX {
            0.0
        } else {
            self.state.current_step as f64 / self.state.total_steps.max(1) as f64
        };

        RecipeStats {
            recipe_name: self.recipe.name.clone(),
            total_steps: self.state.total_steps,
            current_step: self.state.current_step,
            progress_pct: progress * 100.0,
            samples_yielded: self.state.samples_yielded,
            per_dataset_samples: per_dataset,
            current_difficulty: self.state.current_difficulty,
            current_ratios: self.state.current_ratios.clone(),
            elapsed_steps: self.state.current_step,
            estimated_remaining_steps: self.state.total_steps.saturating_sub(self.state.current_step),
        }
    }

    pub fn reset(&mut self) -> Result<(), String> {
        *self = Self::new(self.recipe.clone())?;
        Ok(())
    }
}

pub struct RecipeBuilder {
    recipe: DataRecipe,
}

impl RecipeBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            recipe: DataRecipe {
                name: name.to_string(),
                ..Default::default()
            },
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.recipe.description = Some(desc.to_string());
        self
    }

    pub fn add_dataset(mut self, name: &str, weight: f64) -> Self {
        self.recipe.datasets.push(RecipeDataset {
            name: name.to_string(),
            weight,
            ..Default::default()
        });
        self
    }

    pub fn add_dataset_with_path(
        mut self,
        name: &str,
        weight: f64,
        local_path: &str,
        format: Option<&str>,
    ) -> Self {
        self.recipe.datasets.push(RecipeDataset {
            name: name.to_string(),
            weight,
            local_path: Some(local_path.to_string()),
            format: format.map(|s| s.to_string()),
            ..Default::default()
        });
        self
    }

    pub fn add_cloud_dataset(
        mut self,
        name: &str,
        weight: f64,
        cloud_key: &str,
        provider: &str,
        bucket: &str,
    ) -> Self {
        self.recipe.datasets.push(RecipeDataset {
            name: name.to_string(),
            weight,
            cloud_key: Some(cloud_key.to_string()),
            cloud_provider: Some(provider.to_string()),
            cloud_bucket: Some(bucket.to_string()),
            ..Default::default()
        });
        self
    }

    pub fn proportional(mut self) -> Self {
        self.recipe.mixing_strategy = MixingStrategy::Proportional;
        self
    }

    pub fn interleaved(mut self, samples_per_dataset: usize) -> Self {
        self.recipe.mixing_strategy = MixingStrategy::Interleaved { samples_per_dataset };
        self
    }

    pub fn weighted_random(mut self, temperature: f64) -> Self {
        self.recipe.mixing_strategy = MixingStrategy::WeightedRandom { temperature };
        self
    }

    pub fn sequential(mut self, order: Vec<String>) -> Self {
        self.recipe.mixing_strategy = MixingStrategy::Sequential { order };
        self
    }

    pub fn staged(mut self, stages: Vec<RecipeStage>) -> Self {
        self.recipe.mixing_strategy = MixingStrategy::Staged { stages };
        self
    }

    pub fn with_curriculum(
        mut self,
        difficulty_metric: DifficultyMetric,
        pacing: CurriculumPacing,
        initial_difficulty: f64,
        final_difficulty: f64,
        warmup_steps: usize,
        total_steps: usize,
    ) -> Self {
        self.recipe.curriculum = Some(CurriculumConfig {
            enabled: true,
            difficulty_metric,
            pacing,
            initial_difficulty,
            final_difficulty,
            warmup_steps,
            total_steps,
        });
        self
    }

    pub fn with_dynamic_ratio(
        mut self,
        schedule: RatioSchedule,
        target_ratios: HashMap<String, f64>,
        annealing_steps: usize,
    ) -> Self {
        self.recipe.dynamic_ratio = Some(DynamicRatioConfig {
            enabled: true,
            schedule,
            target_ratios,
            annealing_steps,
        });
        self
    }

    pub fn with_quality_thresholds(mut self, thresholds: QualityThresholds) -> Self {
        self.recipe.quality_thresholds = Some(thresholds);
        self
    }

    pub fn total_samples(mut self, target: usize) -> Self {
        self.recipe.total_samples_target = Some(target);
        self
    }

    pub fn seed(mut self, seed: u64) -> Self {
        self.recipe.seed = seed;
        self
    }

    pub fn build(self) -> Result<DataRecipe, String> {
        self.recipe.validate()?;
        Ok(self.recipe)
    }
}

pub fn create_llm_pretraining_recipe() -> DataRecipe {
    DataRecipe {
        name: "llm_pretraining".to_string(),
        version: "1.0".to_string(),
        description: Some("Standard LLM pretraining recipe: 70% code + 20% math + 10% general".to_string()),
        datasets: vec![
            RecipeDataset {
                name: "code".to_string(),
                weight: 0.70,
                tags: vec!["code".to_string(), "programming".to_string()],
                ..Default::default()
            },
            RecipeDataset {
                name: "math".to_string(),
                weight: 0.20,
                tags: vec!["math".to_string(), "reasoning".to_string()],
                ..Default::default()
            },
            RecipeDataset {
                name: "general".to_string(),
                weight: 0.10,
                tags: vec!["general".to_string(), "web".to_string()],
                ..Default::default()
            },
        ],
        mixing_strategy: MixingStrategy::Proportional,
        curriculum: Some(CurriculumConfig {
            enabled: true,
            difficulty_metric: DifficultyMetric::Perplexity,
            pacing: CurriculumPacing::Linear,
            initial_difficulty: 0.0,
            final_difficulty: 1.0,
            warmup_steps: 1000,
            total_steps: 100000,
        }),
        dynamic_ratio: Some(DynamicRatioConfig {
            enabled: true,
            schedule: RatioSchedule::Cosine,
            target_ratios: [
                ("code".to_string(), 0.50),
                ("math".to_string(), 0.30),
                ("general".to_string(), 0.20),
            ].into_iter().collect(),
            annealing_steps: 50000,
        }),
        quality_thresholds: Some(QualityThresholds::default()),
        total_samples_target: Some(1000000),
        seed: 42,
    }
}

pub fn create_sft_recipe() -> DataRecipe {
    DataRecipe {
        name: "sft_instruction".to_string(),
        version: "1.0".to_string(),
        description: Some("Supervised fine-tuning recipe with instruction data".to_string()),
        datasets: vec![
            RecipeDataset {
                name: "high_quality_instructions".to_string(),
                weight: 0.60,
                min_quality_score: Some(0.8),
                tags: vec!["instruction".to_string(), "high_quality".to_string()],
                ..Default::default()
            },
            RecipeDataset {
                name: "general_instructions".to_string(),
                weight: 0.30,
                min_quality_score: Some(0.5),
                tags: vec!["instruction".to_string(), "general".to_string()],
                ..Default::default()
            },
            RecipeDataset {
                name: "safety_instructions".to_string(),
                weight: 0.10,
                min_quality_score: Some(0.9),
                tags: vec!["instruction".to_string(), "safety".to_string()],
                ..Default::default()
            },
        ],
        mixing_strategy: MixingStrategy::Proportional,
        curriculum: None,
        dynamic_ratio: None,
        quality_thresholds: Some(QualityThresholds {
            min_overall_score: 0.5,
            max_toxicity: 0.1,
            min_language_confidence: 0.9,
            max_repetition_ratio: 0.1,
            require_no_pii: true,
        }),
        total_samples_target: Some(100000),
        seed: 42,
    }
}

pub fn create_rlhf_preference_recipe() -> DataRecipe {
    DataRecipe {
        name: "rlhf_preference".to_string(),
        version: "1.0".to_string(),
        description: Some("RLHF preference data recipe with chosen/rejected pairs".to_string()),
        datasets: vec![
            RecipeDataset {
                name: "helpfulness".to_string(),
                weight: 0.40,
                tags: vec!["rlhf".to_string(), "helpfulness".to_string()],
                ..Default::default()
            },
            RecipeDataset {
                name: "harmlessness".to_string(),
                weight: 0.35,
                tags: vec!["rlhf".to_string(), "safety".to_string()],
                ..Default::default()
            },
            RecipeDataset {
                name: "honesty".to_string(),
                weight: 0.25,
                tags: vec!["rlhf".to_string(), "truthfulness".to_string()],
                ..Default::default()
            },
        ],
        mixing_strategy: MixingStrategy::Proportional,
        curriculum: None,
        dynamic_ratio: None,
        quality_thresholds: Some(QualityThresholds {
            min_overall_score: 0.7,
            max_toxicity: 0.05,
            min_language_confidence: 0.95,
            max_repetition_ratio: 0.05,
            require_no_pii: true,
        }),
        total_samples_target: Some(50000),
        seed: 42,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_validation_empty() {
        let recipe = DataRecipe::default();
        assert!(recipe.validate().is_err());
    }

    #[test]
    fn test_recipe_validation_valid() {
        let recipe = DataRecipe {
            datasets: vec![
                RecipeDataset { name: "ds1".to_string(), weight: 1.0, ..Default::default() },
            ],
            ..Default::default()
        };
        assert!(recipe.validate().is_ok());
    }

    #[test]
    fn test_normalized_weights() {
        let recipe = DataRecipe {
            datasets: vec![
                RecipeDataset { name: "a".to_string(), weight: 7.0, ..Default::default() },
                RecipeDataset { name: "b".to_string(), weight: 2.0, ..Default::default() },
                RecipeDataset { name: "c".to_string(), weight: 1.0, ..Default::default() },
            ],
            ..Default::default()
        };

        let weights = recipe.normalized_weights();
        assert!((weights[0] - 0.7).abs() < 0.01);
        assert!((weights[1] - 0.2).abs() < 0.01);
        assert!((weights[2] - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_curriculum_linear() {
        let config = CurriculumConfig {
            enabled: true,
            difficulty_metric: DifficultyMetric::Perplexity,
            pacing: CurriculumPacing::Linear,
            initial_difficulty: 0.0,
            final_difficulty: 1.0,
            warmup_steps: 100,
            total_steps: 1100,
        };

        assert_eq!(config.difficulty_at_step(0), 0.0);
        assert_eq!(config.difficulty_at_step(50), 0.0);
        assert_eq!(config.difficulty_at_step(100), 0.0);
        assert!((config.difficulty_at_step(600) - 0.5).abs() < 0.01);
        assert!((config.difficulty_at_step(1100) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_curriculum_cosine() {
        let config = CurriculumConfig {
            enabled: true,
            difficulty_metric: DifficultyMetric::Perplexity,
            pacing: CurriculumPacing::Cosine,
            initial_difficulty: 0.0,
            final_difficulty: 1.0,
            warmup_steps: 0,
            total_steps: 1000,
        };

        let mid = config.difficulty_at_step(500);
        assert!((mid - 0.5).abs() < 0.01);
        assert!((config.difficulty_at_step(1000) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_dynamic_ratio_linear() {
        let config = DynamicRatioConfig {
            enabled: true,
            schedule: RatioSchedule::Linear,
            target_ratios: [("code".to_string(), 0.5)].into_iter().collect(),
            annealing_steps: 1000,
        };

        let r = config.ratio_at_step(0, "code", 0.7);
        assert!((r - 0.7).abs() < 0.01);

        let r = config.ratio_at_step(500, "code", 0.7);
        assert!((r - 0.6).abs() < 0.01);

        let r = config.ratio_at_step(1000, "code", 0.7);
        assert!((r - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_recipe_executor_proportional() {
        let recipe = DataRecipe {
            name: "test".to_string(),
            datasets: vec![
                RecipeDataset { name: "a".to_string(), weight: 1.0, max_samples: Some(100), ..Default::default() },
                RecipeDataset { name: "b".to_string(), weight: 1.0, max_samples: Some(100), ..Default::default() },
            ],
            mixing_strategy: MixingStrategy::Proportional,
            total_samples_target: Some(100),
            ..Default::default()
        };

        let mut executor = RecipeExecutor::new(recipe).unwrap();
        let mut counts: HashMap<String, usize> = HashMap::new();

        for _ in 0..100 {
            if let Some(name) = executor.next_dataset() {
                *counts.entry(name).or_default() += 1;
            }
        }

        assert!(counts.get("a").unwrap_or(&0) > &0);
        assert!(counts.get("b").unwrap_or(&0) > &0);
    }

    #[test]
    fn test_recipe_executor_interleaved() {
        let recipe = DataRecipe {
            name: "test".to_string(),
            datasets: vec![
                RecipeDataset { name: "a".to_string(), weight: 1.0, max_samples: Some(100), ..Default::default() },
                RecipeDataset { name: "b".to_string(), weight: 1.0, max_samples: Some(100), ..Default::default() },
            ],
            mixing_strategy: MixingStrategy::Interleaved { samples_per_dataset: 5 },
            total_samples_target: Some(20),
            ..Default::default()
        };

        let mut executor = RecipeExecutor::new(recipe).unwrap();
        let mut sequence = Vec::new();

        for _ in 0..20 {
            if let Some(name) = executor.next_dataset() {
                sequence.push(name);
            }
        }

        assert_eq!(sequence[0], "a");
        assert_eq!(sequence[5], "b");
        assert_eq!(sequence[10], "a");
    }

    #[test]
    fn test_recipe_executor_staged() {
        let recipe = DataRecipe {
            name: "test".to_string(),
            datasets: vec![
                RecipeDataset { name: "easy".to_string(), weight: 1.0, max_samples: Some(100), ..Default::default() },
                RecipeDataset { name: "hard".to_string(), weight: 1.0, max_samples: Some(100), ..Default::default() },
            ],
            mixing_strategy: MixingStrategy::Staged {
                stages: vec![
                    RecipeStage {
                        name: "warmup".to_string(),
                        start_step: 0,
                        end_step: 10,
                        dataset_weights: [("easy".to_string(), 1.0)].into_iter().collect(),
                        description: None,
                    },
                    RecipeStage {
                        name: "main".to_string(),
                        start_step: 10,
                        end_step: 20,
                        dataset_weights: [
                            ("easy".to_string(), 0.5),
                            ("hard".to_string(), 0.5),
                        ].into_iter().collect(),
                        description: None,
                    },
                ],
            },
            total_samples_target: Some(20),
            ..Default::default()
        };

        let mut executor = RecipeExecutor::new(recipe).unwrap();

        for _ in 0..10 {
            let name = executor.next_dataset().unwrap();
            assert_eq!(name, "easy");
        }

        let mut has_hard = false;
        for _ in 0..10 {
            if let Some(name) = executor.next_dataset() {
                if name == "hard" {
                    has_hard = true;
                }
            }
        }
        assert!(has_hard);
    }

    #[test]
    fn test_recipe_builder() {
        let recipe = RecipeBuilder::new("test_recipe")
            .description("A test recipe")
            .add_dataset_with_path("code", 0.7, "/data/code", Some("jsonl"))
            .add_dataset_with_path("math", 0.2, "/data/math", Some("jsonl"))
            .add_dataset("general", 0.1)
            .proportional()
            .total_samples(10000)
            .seed(123)
            .build()
            .unwrap();

        assert_eq!(recipe.name, "test_recipe");
        assert_eq!(recipe.datasets.len(), 3);
        assert_eq!(recipe.total_samples_target, Some(10000));
        assert_eq!(recipe.seed, 123);
    }

    #[test]
    fn test_llm_pretraining_recipe() {
        let recipe = create_llm_pretraining_recipe();
        assert!(recipe.validate().is_ok());
        assert_eq!(recipe.datasets.len(), 3);
        assert!(recipe.curriculum.is_some());
        assert!(recipe.dynamic_ratio.is_some());
    }

    #[test]
    fn test_sft_recipe() {
        let recipe = create_sft_recipe();
        assert!(recipe.validate().is_ok());
        assert_eq!(recipe.datasets.len(), 3);
    }

    #[test]
    fn test_rlhf_recipe() {
        let recipe = create_rlhf_preference_recipe();
        assert!(recipe.validate().is_ok());
        assert_eq!(recipe.datasets.len(), 3);
    }

    #[test]
    fn test_recipe_executor_stats() {
        let recipe = DataRecipe {
            name: "test".to_string(),
            datasets: vec![
                RecipeDataset { name: "ds1".to_string(), weight: 1.0, max_samples: Some(100), ..Default::default() },
            ],
            mixing_strategy: MixingStrategy::Proportional,
            total_samples_target: Some(50),
            ..Default::default()
        };

        let mut executor = RecipeExecutor::new(recipe).unwrap();
        for _ in 0..25 {
            executor.next_dataset();
        }

        let stats = executor.stats();
        assert_eq!(stats.samples_yielded, 25);
        assert!((stats.progress_pct - 50.0).abs() < 1.0);
    }
}
