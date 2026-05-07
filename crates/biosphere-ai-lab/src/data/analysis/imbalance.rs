use serde::{Deserialize, Serialize};

use crate::domain::dataset::aggregate::{ColumnProfile, Dataset, DatasetSplit};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassImbalanceReport {
    pub dataset_id: String,
    pub column_name: String,
    pub total_count: usize,
    pub class_count: usize,
    pub distribution: Vec<ClassDistribution>,
    pub imbalance_ratio: f64,
    pub effective_sample_size: f64,
    pub level: ImbalanceLevel,
    pub recommendations: Vec<String>,
    pub split_reports: Vec<SplitImbalanceReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDistribution {
    pub class_value: String,
    pub count: usize,
    pub ratio: f64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImbalanceLevel {
    Balanced,
    Mild,
    Moderate,
    Severe,
    Extreme,
}

impl std::fmt::Display for ImbalanceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Balanced => write!(f, "balanced"),
            Self::Mild => write!(f, "mild"),
            Self::Moderate => write!(f, "moderate"),
            Self::Severe => write!(f, "severe"),
            Self::Extreme => write!(f, "extreme"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitImbalanceReport {
    pub split_name: String,
    pub split_type: String,
    pub total_count: usize,
    pub distribution: Vec<ClassDistribution>,
    pub imbalance_ratio: f64,
    pub level: ImbalanceLevel,
}

pub struct ImbalanceAnalyzer;

impl ImbalanceAnalyzer {
    pub fn analyze(dataset: &Dataset, column_name: &str, splits: &[DatasetSplit]) -> ClassImbalanceReport {
        let profile = dataset.column_profiles.iter().find(|p| p.name == column_name);
        let (distribution, total_count, class_count) = match profile {
            Some(p) => {
                let total: usize = p.top_values.iter().map(|(_, c)| *c).sum();
                let non_top = p.total_count.saturating_sub(p.null_count).saturating_sub(total);
                let mut dist: Vec<ClassDistribution> = p.top_values.iter()
                    .map(|(v, c)| ClassDistribution {
                        class_value: v.clone(),
                        count: *c,
                        ratio: *c as f64 / p.total_count as f64,
                        percentage: (*c as f64 / p.total_count as f64) * 100.0,
                    })
                    .collect();
                if non_top > 0 {
                    dist.push(ClassDistribution {
                        class_value: "(other)".to_string(),
                        count: non_top,
                        ratio: non_top as f64 / p.total_count as f64,
                        percentage: (non_top as f64 / p.total_count as f64) * 100.0,
                    });
                }
                let cc = p.distinct_count;
                (dist, p.total_count.saturating_sub(p.null_count), cc)
            }
            None => (Vec::new(), 0, 0),
        };

        let imbalance_ratio = Self::compute_imbalance_ratio(&distribution);
        let effective_sample_size = Self::compute_effective_sample_size(&distribution);
        let level = Self::classify_imbalance(imbalance_ratio);
        let recommendations = Self::generate_recommendations(level, class_count, total_count, &distribution);

        let split_reports: Vec<SplitImbalanceReport> = splits.iter().map(|split| {
            Self::analyze_split(split, column_name, &dataset.column_profiles)
        }).collect();

        ClassImbalanceReport {
            dataset_id: dataset.id.to_string(),
            column_name: column_name.to_string(),
            total_count,
            class_count,
            distribution,
            imbalance_ratio,
            effective_sample_size,
            level,
            recommendations,
            split_reports,
        }
    }

    fn analyze_split(split: &DatasetSplit, column_name: &str, profiles: &[ColumnProfile]) -> SplitImbalanceReport {
        let profile = profiles.iter().find(|p| p.name == column_name);
        let (distribution, total_count) = match profile {
            Some(p) => {
                let total: usize = p.top_values.iter().map(|(_, c)| *c).sum();
                let non_top = p.total_count.saturating_sub(p.null_count).saturating_sub(total);
                let mut dist: Vec<ClassDistribution> = p.top_values.iter()
                    .map(|(v, c)| ClassDistribution {
                        class_value: v.clone(),
                        count: *c,
                        ratio: *c as f64 / p.total_count as f64,
                        percentage: (*c as f64 / p.total_count as f64) * 100.0,
                    })
                    .collect();
                if non_top > 0 {
                    dist.push(ClassDistribution {
                        class_value: "(other)".to_string(),
                        count: non_top,
                        ratio: non_top as f64 / p.total_count as f64,
                        percentage: (non_top as f64 / p.total_count as f64) * 100.0,
                    });
                }
                (dist, p.total_count.saturating_sub(p.null_count))
            }
            None => (Vec::new(), 0),
        };

        let ratio = Self::compute_imbalance_ratio(&distribution);
        let level = Self::classify_imbalance(ratio);

        SplitImbalanceReport {
            split_name: split.name.clone(),
            split_type: "train".to_string(),
            total_count,
            distribution,
            imbalance_ratio: ratio,
            level,
        }
    }

    fn compute_imbalance_ratio(distribution: &[ClassDistribution]) -> f64 {
        if distribution.len() < 2 {
            return 1.0;
        }
        let max_count = distribution.iter().map(|d| d.count).max().unwrap_or(1);
        let min_count = distribution.iter().map(|d| d.count).min().unwrap_or(1);
        if min_count == 0 {
            return f64::INFINITY;
        }
        max_count as f64 / min_count as f64
    }

    fn compute_effective_sample_size(distribution: &[ClassDistribution]) -> f64 {
        if distribution.is_empty() {
            return 0.0;
        }
        let total: usize = distribution.iter().map(|d| d.count).sum();
        if total == 0 {
            return 0.0;
        }
        let sum_inv: f64 = distribution.iter()
            .filter(|d| d.count > 0)
            .map(|d| (d.count as f64 / total as f64).powi(-1))
            .sum();
        if sum_inv == 0.0 {
            return 0.0;
        }
        let k = distribution.len() as f64;
        k / sum_inv * (total as f64)
    }

    fn classify_imbalance(ratio: f64) -> ImbalanceLevel {
        if ratio <= 1.5 {
            ImbalanceLevel::Balanced
        } else if ratio <= 3.0 {
            ImbalanceLevel::Mild
        } else if ratio <= 10.0 {
            ImbalanceLevel::Moderate
        } else if ratio <= 50.0 {
            ImbalanceLevel::Severe
        } else {
            ImbalanceLevel::Extreme
        }
    }

    fn generate_recommendations(level: ImbalanceLevel, class_count: usize, total_count: usize, distribution: &[ClassDistribution]) -> Vec<String> {
        let mut recs = Vec::new();

        match level {
            ImbalanceLevel::Balanced => {
                recs.push("数据集类别分布均衡，可直接用于训练".to_string());
            }
            ImbalanceLevel::Mild => {
                recs.push("存在轻微类别不平衡，建议使用 class_weight='balanced' 参数".to_string());
            }
            ImbalanceLevel::Moderate => {
                recs.push("类别不平衡较明显，建议采取以下措施之一：".to_string());
                recs.push("  1. 使用 SMOTE 等过采样方法增加少数类样本".to_string());
                recs.push("  2. 对多数类进行欠采样".to_string());
                recs.push("  3. 使用 F1-score 或 AUC 作为评估指标，而非准确率".to_string());
            }
            ImbalanceLevel::Severe | ImbalanceLevel::Extreme => {
                recs.push("⚠️ 严重类别不平衡！直接训练可能导致模型完全忽略少数类".to_string());
                recs.push("  1. 必须使用过采样（SMOTE/ADASYN）+ 欠采样组合策略".to_string());
                recs.push("  2. 考虑将问题转化为异常检测任务".to_string());
                recs.push("  3. 使用 Precision-Recall 曲线评估，不要使用 ROC 曲线".to_string());
                recs.push("  4. 考虑收集更多少数类样本".to_string());
            }
        }

        if class_count > 20 {
            recs.push(format!("类别数较多（{}），考虑合并低频类别或使用层次分类", class_count));
        }

        let min_count = distribution.iter().map(|d| d.count).min().unwrap_or(0);
        if min_count > 0 && min_count < 10 {
            recs.push(format!("最小类别仅 {} 个样本，统计意义不足，建议至少每类 30+ 样本", min_count));
        }

        if total_count < 100 {
            recs.push("总样本量过少，建议收集更多数据以保证训练可靠性".to_string());
        }

        recs
    }
}
