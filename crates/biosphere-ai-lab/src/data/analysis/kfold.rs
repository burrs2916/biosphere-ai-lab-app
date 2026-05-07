use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::dataset::aggregate::{DatasetSplit, SplitStrategy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KFoldResult {
    pub dataset_id: String,
    pub k: usize,
    pub total_rows: usize,
    pub strategy: KFoldStrategy,
    pub folds: Vec<FoldInfo>,
    pub summary: KFoldSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KFoldStrategy {
    Standard,
    Stratified,
    Group,
}

impl std::fmt::Display for KFoldStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard => write!(f, "standard"),
            Self::Stratified => write!(f, "stratified"),
            Self::Group => write!(f, "group"),
        }
    }
}

impl std::str::FromStr for KFoldStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "standard" => Ok(Self::Standard),
            "stratified" => Ok(Self::Stratified),
            "group" => Ok(Self::Group),
            _ => Err(format!("Unknown K-Fold strategy: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldInfo {
    pub fold_index: usize,
    pub split_name: String,
    pub train_count: usize,
    pub val_count: usize,
    pub train_ratio: f64,
    pub val_ratio: f64,
    pub split: DatasetSplit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KFoldSummary {
    pub avg_train_size: f64,
    pub avg_val_size: f64,
    pub min_val_size: usize,
    pub max_val_size: usize,
    pub size_variance: f64,
    pub is_balanced: bool,
    pub recommendations: Vec<String>,
}

pub struct KFoldSplitter;

impl KFoldSplitter {
    pub fn create_standard(
        dataset_id: &str,
        k: usize,
        total_rows: usize,
        seed: u64,
    ) -> KFoldResult {
        let mut indices: Vec<usize> = (0..total_rows).collect();
        Self::shuffle_with_seed(&mut indices, seed);

        let fold_size = total_rows / k;
        let remainder = total_rows % k;

        let mut folds = Vec::new();
        let mut start = 0;

        for i in 0..k {
            let extra = if i < remainder { 1 } else { 0 };
            let end = start + fold_size + extra;

            let val_indices: Vec<usize> = indices[start..end].to_vec();
            let train_indices: Vec<usize> = indices[..start]
                .iter()
                .chain(indices[end..].iter())
                .copied()
                .collect();

            let split_name = format!("kfold_{}", i);
            let train_count = train_indices.len();
            let val_count = val_indices.len();

            let split = DatasetSplit {
                name: split_name.clone(),
                strategy: SplitStrategy::Random,
                train_ratio: train_count as f64 / total_rows as f64,
                val_ratio: val_count as f64 / total_rows as f64,
                test_ratio: 0.0,
                seed,
                stratify_column: None,
                group_column: None,
                train_indices,
                val_indices,
                test_indices: Vec::new(),
                created_at: chrono::Utc::now(),
            };

            folds.push(FoldInfo {
                fold_index: i,
                split_name,
                train_count,
                val_count,
                train_ratio: train_count as f64 / total_rows as f64,
                val_ratio: val_count as f64 / total_rows as f64,
                split,
            });

            start = end;
        }

        let summary = Self::compute_summary(&folds, total_rows);

        KFoldResult {
            dataset_id: dataset_id.to_string(),
            k,
            total_rows,
            strategy: KFoldStrategy::Standard,
            folds,
            summary,
        }
    }

    pub fn create_stratified(
        dataset_id: &str,
        k: usize,
        total_rows: usize,
        seed: u64,
        column_values: &[String],
    ) -> KFoldResult {
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, v) in column_values.iter().enumerate() {
            groups.entry(v.clone()).or_default().push(i);
        }

        let mut fold_buckets: Vec<Vec<usize>> = vec![Vec::new(); k];

        let mut seed_state = seed;
        for (_, mut indices) in groups {
            Self::shuffle_with_seed(&mut indices, seed_state);
            seed_state = seed_state.wrapping_add(1);

            for (j, &idx) in indices.iter().enumerate() {
                fold_buckets[j % k].push(idx);
            }
        }

        let mut folds = Vec::new();
        for i in 0..k {
            let val_indices = fold_buckets[i].clone();
            let train_indices: Vec<usize> = (0..k)
                .filter(|&j| j != i)
                .flat_map(|j| fold_buckets[j].clone())
                .collect();

            let split_name = format!("kfold_stratified_{}", i);
            let train_count = train_indices.len();
            let val_count = val_indices.len();

            let split = DatasetSplit {
                name: split_name.clone(),
                strategy: SplitStrategy::Stratified,
                train_ratio: train_count as f64 / total_rows as f64,
                val_ratio: val_count as f64 / total_rows as f64,
                test_ratio: 0.0,
                seed,
                stratify_column: None,
                group_column: None,
                train_indices,
                val_indices,
                test_indices: Vec::new(),
                created_at: chrono::Utc::now(),
            };

            folds.push(FoldInfo {
                fold_index: i,
                split_name,
                train_count,
                val_count,
                train_ratio: train_count as f64 / total_rows as f64,
                val_ratio: val_count as f64 / total_rows as f64,
                split,
            });
        }

        let summary = Self::compute_summary(&folds, total_rows);

        KFoldResult {
            dataset_id: dataset_id.to_string(),
            k,
            total_rows,
            strategy: KFoldStrategy::Stratified,
            folds,
            summary,
        }
    }

    pub fn create_group(
        dataset_id: &str,
        k: usize,
        total_rows: usize,
        seed: u64,
        group_values: &[String],
    ) -> KFoldResult {
        let unique_groups: Vec<String> = {
            let set: std::collections::HashSet<&str> = group_values.iter().map(|s| s.as_str()).collect();
            let mut v: Vec<String> = set.into_iter().map(|s| s.to_string()).collect();
            Self::shuffle_with_seed_str(&mut v, seed);
            v
        };

        let group_size = unique_groups.len().max(1) / k;
        let mut fold_buckets: Vec<Vec<usize>> = vec![Vec::new(); k];

        let mut group_to_indices: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, g) in group_values.iter().enumerate() {
            group_to_indices.entry(g.clone()).or_default().push(i);
        }

        for (fold_idx, chunk) in unique_groups.chunks(group_size.max(1)).enumerate() {
            let target_fold = fold_idx.min(k - 1);
            for group_name in chunk {
                if let Some(indices) = group_to_indices.get(group_name) {
                    fold_buckets[target_fold].extend_from_slice(indices);
                }
            }
        }

        let mut folds = Vec::new();
        for i in 0..k {
            let val_indices = fold_buckets[i].clone();
            let train_indices: Vec<usize> = (0..k)
                .filter(|&j| j != i)
                .flat_map(|j| fold_buckets[j].clone())
                .collect();

            let split_name = format!("kfold_group_{}", i);
            let train_count = train_indices.len();
            let val_count = val_indices.len();

            let split = DatasetSplit {
                name: split_name.clone(),
                strategy: SplitStrategy::Group,
                train_ratio: train_count as f64 / total_rows as f64,
                val_ratio: val_count as f64 / total_rows as f64,
                test_ratio: 0.0,
                seed,
                stratify_column: None,
                group_column: None,
                train_indices,
                val_indices,
                test_indices: Vec::new(),
                created_at: chrono::Utc::now(),
            };

            folds.push(FoldInfo {
                fold_index: i,
                split_name,
                train_count,
                val_count,
                train_ratio: train_count as f64 / total_rows as f64,
                val_ratio: val_count as f64 / total_rows as f64,
                split,
            });
        }

        let summary = Self::compute_summary(&folds, total_rows);

        KFoldResult {
            dataset_id: dataset_id.to_string(),
            k,
            total_rows,
            strategy: KFoldStrategy::Group,
            folds,
            summary,
        }
    }

    fn compute_summary(folds: &[FoldInfo], total_rows: usize) -> KFoldSummary {
        let avg_train = folds.iter().map(|f| f.train_count as f64).sum::<f64>() / folds.len() as f64;
        let avg_val = folds.iter().map(|f| f.val_count as f64).sum::<f64>() / folds.len() as f64;
        let min_val = folds.iter().map(|f| f.val_count).min().unwrap_or(0);
        let max_val = folds.iter().map(|f| f.val_count).max().unwrap_or(0);

        let mean_val = avg_val;
        let variance = folds.iter()
            .map(|f| {
                let diff = f.val_count as f64 - mean_val;
                diff * diff
            })
            .sum::<f64>() / folds.len() as f64;

        let is_balanced = (max_val as f64 - min_val as f64) / avg_val.max(1.0) < 0.1;

        let mut recommendations = Vec::new();
        if !is_balanced {
            recommendations.push(format!(
                "验证集大小不均匀（{} ~ {}），建议使用 Stratified K-Fold",
                min_val, max_val
            ));
        }
        if total_rows < folds.len() * 100 {
            recommendations.push(format!(
                "数据量较小（{} 行），{} 折交叉验证可能导致每折样本不足",
                total_rows, folds.len()
            ));
        }
        if is_balanced && total_rows >= folds.len() * 100 {
            recommendations.push(format!(
                "✅ {} 折交叉验证配置合理，每折验证集约 {} 行",
                folds.len(), avg_val as usize
            ));
        }

        KFoldSummary {
            avg_train_size: avg_train,
            avg_val_size: avg_val,
            min_val_size: min_val,
            max_val_size: max_val,
            size_variance: variance,
            is_balanced,
            recommendations,
        }
    }

    fn shuffle_with_seed<T>(items: &mut [T], seed: u64) {
        let mut state = seed;
        for i in (1..items.len()).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (state as usize) % (i + 1);
            items.swap(i, j);
        }
    }

    fn shuffle_with_seed_str(items: &mut [String], seed: u64) {
        let mut state = seed;
        for i in (1..items.len()).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (state as usize) % (i + 1);
            items.swap(i, j);
        }
    }
}
