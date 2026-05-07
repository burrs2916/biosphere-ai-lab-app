use serde::{Deserialize, Serialize};
use rand::Rng;
use rand::SeedableRng;

use crate::data::arrow_table::ArrowTable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterleaveConfig {
    pub datasets: Vec<DatasetMixEntry>,
    pub strategy: InterleaveStrategy,
    pub total_samples: Option<usize>,
    pub seed: Option<u64>,
    pub stopping_strategy: StoppingStrategy,
}

impl Default for InterleaveConfig {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            strategy: InterleaveStrategy::Proportional,
            total_samples: None,
            seed: None,
            stopping_strategy: StoppingStrategy::AllExhausted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMixEntry {
    pub dataset_id: String,
    pub dataset_name: String,
    pub weight: f64,
    pub max_samples: Option<usize>,
    pub source_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterleaveStrategy {
    Proportional,
    Equal,
    RoundRobin,
    WeightedRandom,
    Curriculum,
    Oversample,
}

impl std::fmt::Display for InterleaveStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proportional => write!(f, "proportional"),
            Self::Equal => write!(f, "equal"),
            Self::RoundRobin => write!(f, "round_robin"),
            Self::WeightedRandom => write!(f, "weighted_random"),
            Self::Curriculum => write!(f, "curriculum"),
            Self::Oversample => write!(f, "oversample"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoppingStrategy {
    AllExhausted,
    FirstExhausted,
    FixedSize,
}

impl std::fmt::Display for StoppingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllExhausted => write!(f, "all_exhausted"),
            Self::FirstExhausted => write!(f, "first_exhausted"),
            Self::FixedSize => write!(f, "fixed_size"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterleavedSample {
    pub data: serde_json::Value,
    pub source_dataset_id: String,
    pub source_dataset_name: String,
    pub source_index: usize,
    pub global_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterleaveReport {
    pub total_samples: usize,
    pub dataset_contributions: Vec<DatasetContribution>,
    pub strategy: InterleaveStrategy,
    pub distribution_match: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetContribution {
    pub dataset_id: String,
    pub dataset_name: String,
    pub target_weight: f64,
    pub actual_weight: f64,
    pub samples_used: usize,
    pub samples_available: usize,
}

pub struct DatasetInterleaver;

impl DatasetInterleaver {
    pub fn interleave(
        datasets: &[(&str, &str, &ArrowTable)],
        config: &InterleaveConfig,
    ) -> Result<(Vec<InterleavedSample>, InterleaveReport), String> {
        if datasets.is_empty() {
            return Err("No datasets to interleave".to_string());
        }

        let total_weight: f64 = config.datasets.iter().map(|d| d.weight).sum();
        if total_weight <= 0.0 {
            return Err("Total weight must be positive".to_string());
        }

        let normalized_weights: Vec<f64> = config.datasets.iter()
            .map(|d| d.weight / total_weight)
            .collect();

        let dataset_sizes: Vec<usize> = datasets.iter()
            .map(|(_, _, table)| table.num_rows())
            .collect();

        let total_available: usize = dataset_sizes.iter().sum();

        let target_total = match config.stopping_strategy {
            StoppingStrategy::FixedSize => {
                config.total_samples.unwrap_or(total_available)
            }
            StoppingStrategy::FirstExhausted => {
                let min_size = dataset_sizes.iter().min().copied().unwrap_or(0);
                let min_weight = normalized_weights.iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .copied().unwrap_or(1.0);
                if min_weight > 0.0 {
                    (min_size as f64 / min_weight).ceil() as usize
                } else {
                    min_size
                }
            }
            StoppingStrategy::AllExhausted => {
                config.total_samples.unwrap_or(total_available)
            }
        };

        let target_per_dataset: Vec<usize> = normalized_weights.iter()
            .map(|&w| (target_total as f64 * w).ceil() as usize)
            .collect();

        let mut samples = Vec::with_capacity(target_total);
        let mut cursors: Vec<usize> = vec![0; datasets.len()];
        let mut exhausted: Vec<bool> = vec![false; datasets.len()];
        let mut used_counts: Vec<usize> = vec![0; datasets.len()];

        match config.strategy {
            InterleaveStrategy::RoundRobin => {
                let mut global_idx = 0usize;
                loop {
                    let mut any_added = false;
                    for ds_idx in 0..datasets.len() {
                        if exhausted[ds_idx] {
                            continue;
                        }
                        if cursors[ds_idx] >= dataset_sizes[ds_idx] {
                            exhausted[ds_idx] = true;
                            continue;
                        }
                        if used_counts[ds_idx] >= target_per_dataset[ds_idx] {
                            exhausted[ds_idx] = true;
                            continue;
                        }

                        let (ds_id, ds_name, table) = datasets[ds_idx];
                        let single = table.to_single_batch()?;
                        let row_idx = cursors[ds_idx];
                        let mut row_data = serde_json::Map::new();
                        for col_idx in 0..single.num_columns() {
                            let col = single.column(col_idx);
                            let field = table.schema.field(col_idx);
                            row_data.insert(
                                field.name().clone(),
                                ArrowTable::value_to_json(col.as_ref(), row_idx),
                            );
                        }

                        samples.push(InterleavedSample {
                            data: serde_json::Value::Object(row_data),
                            source_dataset_id: ds_id.to_string(),
                            source_dataset_name: ds_name.to_string(),
                            source_index: row_idx,
                            global_index: global_idx,
                        });

                        cursors[ds_idx] += 1;
                        used_counts[ds_idx] += 1;
                        global_idx += 1;
                        any_added = true;
                    }

                    if !any_added {
                        break;
                    }
                    if exhausted.iter().all(|&e| e) {
                        break;
                    }
                }
            }

            InterleaveStrategy::Proportional | InterleaveStrategy::Equal => {
                let weights: Vec<f64> = match config.strategy {
                    InterleaveStrategy::Equal => vec![1.0; datasets.len()],
                    _ => normalized_weights.clone(),
                };

                let mut global_idx = 0usize;
                let mut remaining = target_total;

                while remaining > 0 {
                    let mut any_added = false;
                    for ds_idx in 0..datasets.len() {
                        if exhausted[ds_idx] {
                            continue;
                        }
                        let batch_size = (weights[ds_idx] * 10.0).ceil() as usize;
                        for _ in 0..batch_size {
                            if remaining == 0 { break; }
                            if cursors[ds_idx] >= dataset_sizes[ds_idx] {
                                exhausted[ds_idx] = true;
                                break;
                            }
                            if used_counts[ds_idx] >= target_per_dataset[ds_idx] {
                                exhausted[ds_idx] = true;
                                break;
                            }

                            let (ds_id, ds_name, table) = datasets[ds_idx];
                            let single = table.to_single_batch()?;
                            let row_idx = cursors[ds_idx];
                            let mut row_data = serde_json::Map::new();
                            for col_idx in 0..single.num_columns() {
                                let col = single.column(col_idx);
                                let field = table.schema.field(col_idx);
                                row_data.insert(
                                    field.name().clone(),
                                    ArrowTable::value_to_json(col.as_ref(), row_idx),
                                );
                            }

                            samples.push(InterleavedSample {
                                data: serde_json::Value::Object(row_data),
                                source_dataset_id: ds_id.to_string(),
                                source_dataset_name: ds_name.to_string(),
                                source_index: row_idx,
                                global_index: global_idx,
                            });

                            cursors[ds_idx] += 1;
                            used_counts[ds_idx] += 1;
                            global_idx += 1;
                            remaining -= 1;
                            any_added = true;
                        }
                    }
                    if !any_added { break; }
                }
            }

            InterleaveStrategy::WeightedRandom => {
                let mut rng: Box<dyn rand::RngCore> = if let Some(s) = config.seed {
                    Box::new(rand::rngs::StdRng::seed_from_u64(s))
                } else {
                    Box::new(rand::rngs::StdRng::from_entropy())
                };

                let mut global_idx = 0usize;
                for _ in 0..target_total {
                    let r: f64 = rng.gen();
                    let mut cumulative = 0.0;
                    let mut chosen = 0usize;

                    for (i, &w) in normalized_weights.iter().enumerate() {
                        cumulative += w;
                        if r <= cumulative {
                            chosen = i;
                            break;
                        }
                    }

                    if exhausted[chosen] {
                        let mut found = false;
                        for i in 0..datasets.len() {
                            if !exhausted[i] {
                                chosen = i;
                                found = true;
                                break;
                            }
                        }
                        if !found { break; }
                    }

                    if cursors[chosen] >= dataset_sizes[chosen] {
                        exhausted[chosen] = true;
                        continue;
                    }

                    let (ds_id, ds_name, table) = datasets[chosen];
                    let single = table.to_single_batch()?;
                    let row_idx = cursors[chosen];
                    let mut row_data = serde_json::Map::new();
                    for col_idx in 0..single.num_columns() {
                        let col = single.column(col_idx);
                        let field = table.schema.field(col_idx);
                        row_data.insert(
                            field.name().clone(),
                            ArrowTable::value_to_json(col.as_ref(), row_idx),
                        );
                    }

                    samples.push(InterleavedSample {
                        data: serde_json::Value::Object(row_data),
                        source_dataset_id: ds_id.to_string(),
                        source_dataset_name: ds_name.to_string(),
                        source_index: row_idx,
                        global_index: global_idx,
                    });

                    cursors[chosen] += 1;
                    used_counts[chosen] += 1;
                    global_idx += 1;
                }
            }

            InterleaveStrategy::Curriculum => {
                let mut global_idx = 0usize;
                let phases = vec![0.3, 0.5, 0.2];
                let mut phase_start = 0usize;

                for &phase_ratio in &phases {
                    let phase_samples = (target_total as f64 * phase_ratio) as usize;
                    let phase_end = phase_start + phase_samples;

                    for ds_idx in 0..datasets.len() {
                        if exhausted[ds_idx] { continue; }
                        let ds_phase_samples = (phase_samples as f64 * normalized_weights[ds_idx]) as usize;

                        for _ in 0..ds_phase_samples {
                            if global_idx >= phase_end { break; }
                            if cursors[ds_idx] >= dataset_sizes[ds_idx] {
                                exhausted[ds_idx] = true;
                                break;
                            }

                            let (ds_id, ds_name, table) = datasets[ds_idx];
                            let single = table.to_single_batch()?;
                            let row_idx = cursors[ds_idx];
                            let mut row_data = serde_json::Map::new();
                            for col_idx in 0..single.num_columns() {
                                let col = single.column(col_idx);
                                let field = table.schema.field(col_idx);
                                row_data.insert(
                                    field.name().clone(),
                                    ArrowTable::value_to_json(col.as_ref(), row_idx),
                                );
                            }

                            samples.push(InterleavedSample {
                                data: serde_json::Value::Object(row_data),
                                source_dataset_id: ds_id.to_string(),
                                source_dataset_name: ds_name.to_string(),
                                source_index: row_idx,
                                global_index: global_idx,
                            });

                            cursors[ds_idx] += 1;
                            used_counts[ds_idx] += 1;
                            global_idx += 1;
                        }
                    }
                    phase_start = phase_end;
                }
            }

            InterleaveStrategy::Oversample => {
                let mut global_idx = 0usize;
                for _ in 0..target_total {
                    let mut any_added = false;
                    for ds_idx in 0..datasets.len() {
                        if used_counts[ds_idx] >= target_per_dataset[ds_idx] {
                            continue;
                        }

                        let (ds_id, ds_name, table) = datasets[ds_idx];
                        let size = dataset_sizes[ds_idx];
                        let row_idx = if cursors[ds_idx] < size {
                            let idx = cursors[ds_idx];
                            cursors[ds_idx] += 1;
                            idx
                        } else {
                            cursors[ds_idx] % size
                        };

                        let single = table.to_single_batch()?;
                        let actual_idx = row_idx % size;
                        let mut row_data = serde_json::Map::new();
                        for col_idx in 0..single.num_columns() {
                            let col = single.column(col_idx);
                            let field = table.schema.field(col_idx);
                            row_data.insert(
                                field.name().clone(),
                                ArrowTable::value_to_json(col.as_ref(), actual_idx),
                            );
                        }

                        samples.push(InterleavedSample {
                            data: serde_json::Value::Object(row_data),
                            source_dataset_id: ds_id.to_string(),
                            source_dataset_name: ds_name.to_string(),
                            source_index: actual_idx,
                            global_index: global_idx,
                        });

                        used_counts[ds_idx] += 1;
                        global_idx += 1;
                        any_added = true;
                    }
                    if !any_added { break; }
                }
            }
        }

        let total_used: usize = used_counts.iter().sum();
        let contributions: Vec<DatasetContribution> = datasets.iter().enumerate()
            .map(|(i, (ds_id, ds_name, _))| DatasetContribution {
                dataset_id: ds_id.to_string(),
                dataset_name: ds_name.to_string(),
                target_weight: normalized_weights[i],
                actual_weight: if total_used > 0 {
                    used_counts[i] as f64 / total_used as f64
                } else {
                    0.0
                },
                samples_used: used_counts[i],
                samples_available: dataset_sizes[i],
            })
            .collect();

        let distribution_match = contributions.iter()
            .map(|c| 1.0 - (c.target_weight - c.actual_weight).abs())
            .sum::<f64>() / contributions.len().max(1) as f64;

        let report = InterleaveReport {
            total_samples: samples.len(),
            dataset_contributions: contributions,
            strategy: config.strategy,
            distribution_match,
        };

        Ok((samples, report))
    }

    pub fn validate_config(config: &InterleaveConfig) -> Result<(), String> {
        if config.datasets.is_empty() {
            return Err("At least one dataset required".to_string());
        }
        for entry in &config.datasets {
            if entry.weight <= 0.0 {
                return Err(format!(
                    "Dataset '{}' has non-positive weight: {}",
                    entry.dataset_name, entry.weight
                ));
            }
        }
        Ok(())
    }

    pub fn compute_optimal_weights(
        dataset_sizes: &[usize],
        target_distribution: &[f64],
    ) -> Vec<f64> {
        let total: usize = dataset_sizes.iter().sum();
        if total == 0 {
            return vec![0.0; dataset_sizes.len()];
        }

        let current_dist: Vec<f64> = dataset_sizes.iter()
            .map(|&s| s as f64 / total as f64)
            .collect();

        let mut weights = vec![0.0; dataset_sizes.len()];
        for i in 0..dataset_sizes.len() {
            if current_dist[i] > 0.0 {
                weights[i] = target_distribution[i] / current_dist[i];
            }
        }

        let max_weight = weights.iter().cloned().fold(0.0f64, f64::max);
        if max_weight > 0.0 {
            for w in &mut weights {
                *w /= max_weight;
            }
        }

        weights
    }
}
