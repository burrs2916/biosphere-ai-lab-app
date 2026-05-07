use serde::{Deserialize, Serialize};

use crate::data::arrow_table::ArrowTable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    pub num_shards: usize,
    pub shard_index: usize,
    pub strategy: ShardStrategy,
    pub seed: Option<u64>,
    pub contiguous: bool,
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            num_shards: 1,
            shard_index: 0,
            strategy: ShardStrategy::Contiguous,
            seed: None,
            contiguous: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShardStrategy {
    Contiguous,
    Interleaved,
    Hashed,
    Stratified,
    Weighted,
}

impl std::fmt::Display for ShardStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contiguous => write!(f, "contiguous"),
            Self::Interleaved => write!(f, "interleaved"),
            Self::Hashed => write!(f, "hashed"),
            Self::Stratified => write!(f, "stratified"),
            Self::Weighted => write!(f, "weighted"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    pub shard_index: usize,
    pub num_shards: usize,
    pub start_row: usize,
    pub end_row: usize,
    pub num_rows: usize,
    pub strategy: ShardStrategy,
    pub is_first: bool,
    pub is_last: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPlan {
    pub total_rows: usize,
    pub num_shards: usize,
    pub shards: Vec<ShardInfo>,
    pub strategy: ShardStrategy,
}

pub struct DataSharder;

impl DataSharder {
    pub fn compute_shard_plan(
        total_rows: usize,
        num_shards: usize,
        strategy: ShardStrategy,
        _seed: Option<u64>,
    ) -> ShardPlan {
        let mut shards = Vec::with_capacity(num_shards);

        match strategy {
            ShardStrategy::Contiguous => {
                let base = total_rows / num_shards;
                let remainder = total_rows % num_shards;
                let mut start = 0usize;

                for i in 0..num_shards {
                    let size = if i < remainder { base + 1 } else { base };
                    let end = start + size;
                    shards.push(ShardInfo {
                        shard_index: i,
                        num_shards,
                        start_row: start,
                        end_row: end,
                        num_rows: size,
                        strategy,
                        is_first: i == 0,
                        is_last: i == num_shards - 1,
                    });
                    start = end;
                }
            }

            ShardStrategy::Interleaved => {
                for i in 0..num_shards {
                    let mut count = 0usize;
                    let mut idx = i;
                    while idx < total_rows {
                        count += 1;
                        idx += num_shards;
                    }
                    shards.push(ShardInfo {
                        shard_index: i,
                        num_shards,
                        start_row: i,
                        end_row: total_rows,
                        num_rows: count,
                        strategy,
                        is_first: i == 0,
                        is_last: i == num_shards - 1,
                    });
                }
            }

            ShardStrategy::Hashed => {
                let base = total_rows / num_shards;
                let remainder = total_rows % num_shards;

                for i in 0..num_shards {
                    let size = if i < remainder { base + 1 } else { base };
                    shards.push(ShardInfo {
                        shard_index: i,
                        num_shards,
                        start_row: 0,
                        end_row: total_rows,
                        num_rows: size,
                        strategy,
                        is_first: i == 0,
                        is_last: i == num_shards - 1,
                    });
                }
            }

            ShardStrategy::Stratified | ShardStrategy::Weighted => {
                let base = total_rows / num_shards;
                let remainder = total_rows % num_shards;
                let mut start = 0usize;

                for i in 0..num_shards {
                    let size = if i < remainder { base + 1 } else { base };
                    let end = start + size;
                    shards.push(ShardInfo {
                        shard_index: i,
                        num_shards,
                        start_row: start,
                        end_row: end,
                        num_rows: size,
                        strategy,
                        is_first: i == 0,
                        is_last: i == num_shards - 1,
                    });
                    start = end;
                }
            }
        }

        ShardPlan {
            total_rows,
            num_shards,
            shards,
            strategy,
        }
    }

    pub fn shard_arrow_table(
        table: &ArrowTable,
        config: &ShardingConfig,
    ) -> Result<ArrowTable, String> {
        let total = table.num_rows();
        let plan = Self::compute_shard_plan(total, config.num_shards, config.strategy, config.seed);

        let shard_info = plan.shards.get(config.shard_index)
            .ok_or_else(|| format!("Shard index {} out of range (0..{})", config.shard_index, config.num_shards))?;

        match config.strategy {
            ShardStrategy::Contiguous | ShardStrategy::Stratified | ShardStrategy::Weighted => {
                table.slice(shard_info.start_row, shard_info.num_rows)
            }

            ShardStrategy::Interleaved => {
                let single = table.to_single_batch()?;
                let schema = table.schema.clone();
                let mut builder = crate::data::arrow_table::ArrowTableBuilder::new(
                    &format!("{}_shard_{}", table.name, config.shard_index),
                    schema.clone(),
                );

                let mut idx = config.shard_index;
                while idx < total {
                    for col_idx in 0..schema.fields().len() {
                        let col = single.column(col_idx);
                        let field = schema.field(col_idx);
                        let val = ArrowTable::value_to_json(col.as_ref(), idx);
                        match field.data_type() {
                            arrow::datatypes::DataType::Int64 => {
                                if let Some(v) = val.as_i64() {
                                    builder.push_int(field.name(), v);
                                } else {
                                    builder.push_null(field.name());
                                }
                            }
                            arrow::datatypes::DataType::Float64 => {
                                if let Some(v) = val.as_f64() {
                                    builder.push_float(field.name(), v);
                                } else {
                                    builder.push_null(field.name());
                                }
                            }
                            arrow::datatypes::DataType::Boolean => {
                                if let Some(v) = val.as_bool() {
                                    builder.push_bool(field.name(), v);
                                } else {
                                    builder.push_null(field.name());
                                }
                            }
                            _ => {
                                if let Some(s) = val.as_str() {
                                    builder.push_string(field.name(), s);
                                } else {
                                    builder.push_null(field.name());
                                }
                            }
                        }
                    }
                    idx += config.num_shards;
                }

                builder.build()
            }

            ShardStrategy::Hashed => {
                let single = table.to_single_batch()?;
                let schema = table.schema.clone();
                let mut builder = crate::data::arrow_table::ArrowTableBuilder::new(
                    &format!("{}_shard_{}", table.name, config.shard_index),
                    schema.clone(),
                );

                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};

                for row_idx in 0..total {
                    let mut hasher = DefaultHasher::new();
                    row_idx.hash(&mut hasher);
                    let hash_val = hasher.finish();
                    let assigned_shard = (hash_val as usize) % config.num_shards;

                    if assigned_shard == config.shard_index {
                        for col_idx in 0..schema.fields().len() {
                            let col = single.column(col_idx);
                            let field = schema.field(col_idx);
                            let val = ArrowTable::value_to_json(col.as_ref(), row_idx);
                            match field.data_type() {
                                arrow::datatypes::DataType::Int64 => {
                                    if let Some(v) = val.as_i64() {
                                        builder.push_int(field.name(), v);
                                    } else {
                                        builder.push_null(field.name());
                                    }
                                }
                                arrow::datatypes::DataType::Float64 => {
                                    if let Some(v) = val.as_f64() {
                                        builder.push_float(field.name(), v);
                                    } else {
                                        builder.push_null(field.name());
                                    }
                                }
                                arrow::datatypes::DataType::Boolean => {
                                    if let Some(v) = val.as_bool() {
                                        builder.push_bool(field.name(), v);
                                    } else {
                                        builder.push_null(field.name());
                                    }
                                }
                                _ => {
                                    if let Some(s) = val.as_str() {
                                        builder.push_string(field.name(), s);
                                    } else {
                                        builder.push_null(field.name());
                                    }
                                }
                            }
                        }
                    }
                }

                builder.build()
            }
        }
    }

    pub fn shard_indices(
        total_rows: usize,
        config: &ShardingConfig,
    ) -> Result<Vec<usize>, String> {
        let plan = Self::compute_shard_plan(total_rows, config.num_shards, config.strategy, config.seed);
        let shard_info = plan.shards.get(config.shard_index)
            .ok_or_else(|| format!("Shard index {} out of range", config.shard_index))?;

        match config.strategy {
            ShardStrategy::Contiguous | ShardStrategy::Stratified | ShardStrategy::Weighted => {
                Ok((shard_info.start_row..shard_info.end_row).collect())
            }
            ShardStrategy::Interleaved => {
                let mut indices = Vec::new();
                let mut idx = config.shard_index;
                while idx < total_rows {
                    indices.push(idx);
                    idx += config.num_shards;
                }
                Ok(indices)
            }
            ShardStrategy::Hashed => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};

                let mut indices = Vec::new();
                for row_idx in 0..total_rows {
                    let mut hasher = DefaultHasher::new();
                    row_idx.hash(&mut hasher);
                    let hash_val = hasher.finish();
                    let assigned_shard = (hash_val as usize) % config.num_shards;
                    if assigned_shard == config.shard_index {
                        indices.push(row_idx);
                    }
                }
                Ok(indices)
            }
        }
    }

    pub fn estimate_shard_sizes(
        total_rows: usize,
        num_shards: usize,
        strategy: ShardStrategy,
    ) -> Vec<usize> {
        let plan = Self::compute_shard_plan(total_rows, num_shards, strategy, None);
        plan.shards.iter().map(|s| s.num_rows).collect()
    }

    pub fn validate_config(config: &ShardingConfig) -> Result<(), String> {
        if config.num_shards == 0 {
            return Err("num_shards must be greater than 0".to_string());
        }
        if config.shard_index >= config.num_shards {
            return Err(format!(
                "shard_index {} must be less than num_shards {}",
                config.shard_index, config.num_shards
            ));
        }
        Ok(())
    }
}
