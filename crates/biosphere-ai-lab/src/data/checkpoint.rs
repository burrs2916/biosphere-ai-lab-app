use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLoaderCheckpoint {
    pub checkpoint_id: String,
    pub training_id: String,
    pub dataset_id: String,
    pub epoch: usize,
    pub global_step: usize,
    pub samples_seen: usize,
    pub cursor_positions: HashMap<String, DatasetCursor>,
    pub shuffle_state: Option<ShuffleState>,
    pub created_at: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetCursor {
    pub dataset_id: String,
    pub current_index: usize,
    pub total_samples: usize,
    pub exhausted: bool,
    pub shard_index: Option<usize>,
    pub num_shards: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShuffleState {
    pub seed: u64,
    pub permutation: Vec<usize>,
    pub current_position: usize,
    pub buffer_state: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    pub save_interval_steps: usize,
    pub save_interval_seconds: f64,
    pub max_checkpoints: usize,
    pub save_directory: String,
    pub compress: bool,
    pub include_shuffle_state: bool,
    pub auto_resume: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            save_interval_steps: 1000,
            save_interval_seconds: 300.0,
            max_checkpoints: 5,
            save_directory: "./checkpoints".to_string(),
            compress: false,
            include_shuffle_state: true,
            auto_resume: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckpointManager {
    config: CheckpointConfig,
    checkpoints: Vec<DataLoaderCheckpoint>,
    last_save_step: usize,
    last_save_time: std::time::Instant,
}

impl CheckpointManager {
    pub fn new(config: CheckpointConfig) -> Self {
        Self {
            config,
            checkpoints: Vec::new(),
            last_save_step: 0,
            last_save_time: std::time::Instant::now(),
        }
    }

    pub fn should_save(&self, current_step: usize) -> bool {
        let steps_elapsed = current_step - self.last_save_step;
        let time_elapsed = self.last_save_time.elapsed().as_secs_f64();

        steps_elapsed >= self.config.save_interval_steps
            || time_elapsed >= self.config.save_interval_seconds
    }

    pub fn create_checkpoint(
        &mut self,
        training_id: &str,
        dataset_id: &str,
        epoch: usize,
        global_step: usize,
        samples_seen: usize,
        cursors: HashMap<String, DatasetCursor>,
        shuffle_state: Option<ShuffleState>,
    ) -> DataLoaderCheckpoint {
        let checkpoint = DataLoaderCheckpoint {
            checkpoint_id: format!("ckpt_{}_{}", training_id, global_step),
            training_id: training_id.to_string(),
            dataset_id: dataset_id.to_string(),
            epoch,
            global_step,
            samples_seen,
            cursor_positions: cursors,
            shuffle_state: if self.config.include_shuffle_state {
                shuffle_state
            } else {
                None
            },
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        };

        self.checkpoints.push(checkpoint.clone());
        self.last_save_step = global_step;
        self.last_save_time = std::time::Instant::now();

        self.prune_old_checkpoints();

        checkpoint
    }

    fn prune_old_checkpoints(&mut self) {
        while self.checkpoints.len() > self.config.max_checkpoints {
            self.checkpoints.remove(0);
        }
    }

    pub fn get_latest_checkpoint(&self) -> Option<&DataLoaderCheckpoint> {
        self.checkpoints.last()
    }

    pub fn get_checkpoint_by_step(&self, step: usize) -> Option<&DataLoaderCheckpoint> {
        self.checkpoints.iter().find(|c| c.global_step == step)
    }

    pub fn save_to_disk(&self, checkpoint: &DataLoaderCheckpoint) -> Result<PathBuf, String> {
        let dir = PathBuf::from(&self.config.save_directory);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create checkpoint dir: {}", e))?;

        let filename = format!("{}.json", checkpoint.checkpoint_id);
        let path = dir.join(&filename);

        let json = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| format!("Failed to serialize checkpoint: {}", e))?;

        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write checkpoint: {}", e))?;

        Ok(path)
    }

    pub fn load_from_disk(&self, checkpoint_id: &str) -> Result<DataLoaderCheckpoint, String> {
        let dir = PathBuf::from(&self.config.save_directory);
        let filename = format!("{}.json", checkpoint_id);
        let path = dir.join(&filename);

        let json = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read checkpoint: {}", e))?;

        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize checkpoint: {}", e))
    }

    pub fn list_checkpoints(&self) -> Result<Vec<DataLoaderCheckpoint>, String> {
        let dir = PathBuf::from(&self.config.save_directory);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut checkpoints = Vec::new();
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| format!("Failed to read checkpoint dir: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(ckpt) = serde_json::from_str::<DataLoaderCheckpoint>(&json) {
                        checkpoints.push(ckpt);
                    }
                }
            }
        }

        checkpoints.sort_by(|a, b| b.global_step.cmp(&a.global_step));
        Ok(checkpoints)
    }

    pub fn auto_resume(&self) -> Result<Option<DataLoaderCheckpoint>, String> {
        if !self.config.auto_resume {
            return Ok(None);
        }

        let checkpoints = self.list_checkpoints()?;
        Ok(checkpoints.into_iter().next())
    }

    pub fn compute_resume_cursor(
        checkpoint: &DataLoaderCheckpoint,
        dataset_id: &str,
    ) -> Option<DatasetCursor> {
        checkpoint.cursor_positions.get(dataset_id).cloned()
    }

    pub fn create_cursor(
        dataset_id: &str,
        current_index: usize,
        total_samples: usize,
        exhausted: bool,
    ) -> DatasetCursor {
        DatasetCursor {
            dataset_id: dataset_id.to_string(),
            current_index,
            total_samples,
            exhausted,
            shard_index: None,
            num_shards: None,
        }
    }

    pub fn create_shuffle_state(
        seed: u64,
        total_samples: usize,
    ) -> ShuffleState {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut permutation: Vec<usize> = (0..total_samples).collect();
        permutation.shuffle(&mut rng);

        ShuffleState {
            seed,
            permutation,
            current_position: 0,
            buffer_state: None,
        }
    }

    pub fn advance_shuffle_state(state: &mut ShuffleState, steps: usize) {
        state.current_position = (state.current_position + steps).min(state.permutation.len());
    }

    pub fn get_next_batch_indices(
        state: &ShuffleState,
        batch_size: usize,
    ) -> Vec<usize> {
        let start = state.current_position;
        let end = (start + batch_size).min(state.permutation.len());
        state.permutation[start..end].to_vec()
    }

    pub fn get_checkpoint_summary(&self) -> CheckpointSummary {
        CheckpointSummary {
            total_checkpoints: self.checkpoints.len(),
            latest_step: self.checkpoints.last().map(|c| c.global_step).unwrap_or(0),
            latest_epoch: self.checkpoints.last().map(|c| c.epoch).unwrap_or(0),
            total_samples_seen: self.checkpoints.last().map(|c| c.samples_seen).unwrap_or(0),
            save_directory: self.config.save_directory.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSummary {
    pub total_checkpoints: usize,
    pub latest_step: usize,
    pub latest_epoch: usize,
    pub total_samples_seen: usize,
    pub save_directory: String,
}
