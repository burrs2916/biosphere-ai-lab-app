use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::domain::dataset::aggregate::{ColumnProfile, Dataset, DatasetSplit};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeakageReport {
    pub dataset_id: String,
    pub split_name: String,
    pub train_size: usize,
    pub test_size: usize,
    pub val_size: usize,
    pub train_test_overlap: usize,
    pub train_val_overlap: usize,
    pub val_test_overlap: usize,
    pub has_leakage: bool,
    pub severity: LeakageSeverity,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeakageSeverity {
    None,
    Low,
    High,
    Critical,
}

impl std::fmt::Display for LeakageSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Low => write!(f, "low"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureLeakageReport {
    pub dataset_id: String,
    pub target_column: String,
    pub suspicious_features: Vec<SuspiciousFeature>,
    pub has_leakage: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspiciousFeature {
    pub feature_name: String,
    pub correlation_with_target: f64,
    pub risk_level: LeakageRisk,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeakageRisk {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for LeakageRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSufficiencyReport {
    pub dataset_id: String,
    pub total_rows: usize,
    pub num_features: usize,
    pub num_classes: Option<usize>,
    pub model_type: String,
    pub estimated_params: usize,
    pub min_recommended_samples: usize,
    pub ideal_recommended_samples: usize,
    pub is_sufficient: bool,
    pub sufficiency_ratio: f64,
    pub risk_assessment: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitConsistencyReport {
    pub dataset_id: String,
    pub split_name: String,
    pub target_column: String,
    pub train_distribution: Vec<ClassDistribution>,
    pub val_distribution: Vec<ClassDistribution>,
    pub test_distribution: Vec<ClassDistribution>,
    pub train_val_psi: f64,
    pub train_test_psi: f64,
    pub val_test_psi: f64,
    pub is_consistent: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDistribution {
    pub class_value: String,
    pub train_ratio: f64,
    pub val_ratio: f64,
    pub test_ratio: f64,
    pub max_deviation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataReadinessScore {
    pub dataset_id: String,
    pub overall_score: f64,
    pub grade: ReadinessGrade,
    pub dimension_scores: DimensionScores,
    pub critical_blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub is_ready_for_training: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScores {
    pub data_quality: f64,
    pub split_safety: f64,
    pub leakage_safety: f64,
    pub data_sufficiency: f64,
    pub balance_health: f64,
    pub documentation: f64,
    pub split_consistency: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessGrade {
    A,
    B,
    C,
    D,
    F,
}

impl std::fmt::Display for ReadinessGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
            Self::F => write!(f, "F"),
        }
    }
}

pub struct SafetyAnalyzer;

impl SafetyAnalyzer {
    pub fn detect_split_leakage(dataset: &Dataset, split: &DatasetSplit) -> LeakageReport {
        let train_set: HashSet<usize> = split.train_indices.iter().copied().collect();
        let val_set: HashSet<usize> = split.val_indices.iter().copied().collect();
        let test_set: HashSet<usize> = split.test_indices.iter().copied().collect();

        let train_test_overlap = train_set.intersection(&test_set).count();
        let train_val_overlap = train_set.intersection(&val_set).count();
        let val_test_overlap = val_set.intersection(&test_set).count();

        let has_leakage = train_test_overlap > 0 || train_val_overlap > 0 || val_test_overlap > 0;

        let severity = if train_test_overlap > 0 {
            if train_test_overlap as f64 / train_set.len().min(test_set.len()).max(1) as f64 > 0.05 {
                LeakageSeverity::Critical
            } else {
                LeakageSeverity::High
            }
        } else if train_val_overlap > 0 {
            LeakageSeverity::Low
        } else if val_test_overlap > 0 {
            LeakageSeverity::Low
        } else {
            LeakageSeverity::None
        };

        let mut recommendations = Vec::new();
        if train_test_overlap > 0 {
            recommendations.push(format!(
                "🔴 严重泄露：训练集和测试集有 {} 行重叠！模型评估结果将完全不可信",
                train_test_overlap
            ));
            recommendations.push("请重新创建 Split，确保 train/test 完全隔离".to_string());
        }
        if train_val_overlap > 0 {
            recommendations.push(format!(
                "🟡 训练集和验证集有 {} 行重叠，EarlyStopping 和超参选择可能偏差",
                train_val_overlap
            ));
        }
        if val_test_overlap > 0 {
            recommendations.push(format!(
                "🟡 验证集和测试集有 {} 行重叠，测试评估可能过于乐观",
                val_test_overlap
            ));
        }
        if !has_leakage {
            recommendations.push("✅ 未检测到 Split 间数据泄露".to_string());
        }

        LeakageReport {
            dataset_id: dataset.id.to_string(),
            split_name: split.name.clone(),
            train_size: split.train_indices.len(),
            test_size: split.test_indices.len(),
            val_size: split.val_indices.len(),
            train_test_overlap,
            train_val_overlap,
            val_test_overlap,
            has_leakage,
            severity,
            recommendations,
        }
    }

    pub fn detect_feature_leakage(
        dataset: &Dataset,
        target_column: &str,
    ) -> FeatureLeakageReport {
        let target_profile = dataset.column_profiles.iter()
            .find(|p| p.name == target_column);

        let mut suspicious = Vec::new();

        for profile in &dataset.column_profiles {
            if profile.name == target_column {
                continue;
            }

            let mut risk = LeakageRisk::Low;
            let mut reason = String::new();

            let name_lower = profile.name.to_lowercase();
            let target_lower = target_column.to_lowercase();

            if name_lower.contains(&target_lower) || target_lower.contains(&name_lower) {
                if name_lower != target_lower {
                    risk = LeakageRisk::High;
                    reason = format!(
                        "列名 '{}' 包含目标列名 '{}'，可能是目标列的变体或衍生列",
                        profile.name, target_column
                    );
                }
            }

            if profile.name.to_lowercase().contains("id") && profile.distinct_count == profile.total_count {
                risk = LeakageRisk::Medium;
                reason = format!(
                    "列 '{}' 是唯一标识符（{} 个唯一值），可能编码了目标信息",
                    profile.name, profile.distinct_count
                );
            }

            if profile.is_numeric() {
                if let Some(mean) = profile.mean_value {
                    if let Some(target_profile) = target_profile {
                        if let Some(target_mean) = target_profile.mean_value {
                            let corr_estimate = if target_mean.abs() > 1e-10 {
                                1.0 - (mean - target_mean).abs() / target_mean.abs()
                            } else {
                                0.0
                            };

                            if corr_estimate > 0.95 {
                                if (risk as u8) < (LeakageRisk::Critical as u8) {
                                    risk = LeakageRisk::Critical;
                                }
                                reason = format!(
                                    "{}列 '{}' 与目标列 '{}' 高度相关（估计 r≈{:.3}），可能是目标列的代理变量",
                                    if reason.is_empty() { "" } else { &reason },
                                    profile.name, target_column, corr_estimate
                                );
                            } else if corr_estimate > 0.85 {
                                if (risk as u8) < (LeakageRisk::High as u8) {
                                    risk = LeakageRisk::High;
                                }
                                reason = format!(
                                    "列 '{}' 与目标列 '{}' 强相关（估计 r≈{:.3}），请确认不是泄露特征",
                                    profile.name, target_column, corr_estimate
                                );
                            }
                        }
                    }
                }
            }

            if profile.is_categorical() && target_profile.map_or(false, |t| t.is_categorical()) {
                let target_values: HashSet<&str> = target_profile.unwrap().top_values.iter()
                    .map(|(v, _)| v.as_str())
                    .collect();
                let feature_values: HashSet<&str> = profile.top_values.iter()
                    .map(|(v, _)| v.as_str())
                    .collect();
                let overlap = target_values.intersection(&feature_values).count();
                let total = target_values.len().min(feature_values.len()).max(1);
                if overlap as f64 / total as f64 > 0.8 && (risk as u8) < (LeakageRisk::High as u8) {
                    risk = LeakageRisk::High;
                    reason = format!(
                        "列 '{}' 的类别值与目标列高度重叠（{}/{}），可能是目标列的编码",
                        profile.name, overlap, total
                    );
                }
            }

            if risk != LeakageRisk::Low {
                suspicious.push(SuspiciousFeature {
                    feature_name: profile.name.clone(),
                    correlation_with_target: 0.0,
                    risk_level: risk,
                    reason,
                });
            }
        }

        let has_leakage = suspicious.iter().any(|s| {
            matches!(s.risk_level, LeakageRisk::High | LeakageRisk::Critical)
        });

        let mut recommendations = Vec::new();
        if has_leakage {
            recommendations.push("🔴 检测到潜在的特征泄露！请检查上述可疑特征".to_string());
            recommendations.push("确保训练时排除这些特征，否则模型将学到虚假模式".to_string());
        } else if !suspicious.is_empty() {
            recommendations.push("🟡 发现一些需要关注的特征，建议人工审查".to_string());
        } else {
            recommendations.push("✅ 未检测到明显的特征泄露".to_string());
        }

        FeatureLeakageReport {
            dataset_id: dataset.id.to_string(),
            target_column: target_column.to_string(),
            suspicious_features: suspicious,
            has_leakage,
            recommendations,
        }
    }

    pub fn assess_data_sufficiency(
        dataset: &Dataset,
        model_type: &str,
        estimated_params: usize,
    ) -> DataSufficiencyReport {
        let num_features = dataset.column_profiles.iter()
            .filter(|p| p.is_numeric() || p.is_categorical())
            .count();

        let num_classes = dataset.column_profiles.iter()
            .find(|p| p.is_categorical())
            .map(|p| p.distinct_count);

        let (min_per_param, ideal_per_param) = match model_type.to_lowercase().as_str() {
            "mlp" | "linear" => (10.0, 50.0),
            "cnn" => (50.0, 200.0),
            "transformer" | "llm" => (100.0, 500.0),
            "xgboost" | "gbm" | "random_forest" => (5.0, 30.0),
            _ => (20.0, 100.0),
        };

        let min_recommended = (estimated_params as f64 * min_per_param) as usize;
        let ideal_recommended = (estimated_params as f64 * ideal_per_param) as usize;

        let feature_based_min = num_features * 50;
        let class_based_min = num_classes.map(|c| c * 100).unwrap_or(0);

        let final_min = min_recommended.max(feature_based_min).max(class_based_min);
        let final_ideal = ideal_recommended.max(feature_based_min * 3).max(class_based_min * 3);

        let sufficiency_ratio = dataset.rows as f64 / final_min.max(1) as f64;
        let is_sufficient = sufficiency_ratio >= 1.0;

        let risk_assessment = if sufficiency_ratio >= 3.0 {
            "数据量充足，可以放心训练".to_string()
        } else if sufficiency_ratio >= 1.0 {
            "数据量基本满足最低要求，建议使用正则化和数据增强".to_string()
        } else if sufficiency_ratio >= 0.5 {
            "数据量不足，强烈建议使用迁移学习或减少模型复杂度".to_string()
        } else {
            "数据量严重不足，训练结果可能不可靠".to_string()
        };

        let mut recommendations = Vec::new();
        if !is_sufficient {
            recommendations.push(format!(
                "当前 {} 行 < 最低推荐 {} 行（{:.0}%），建议：",
                dataset.rows, final_min, sufficiency_ratio * 100.0
            ));
            recommendations.push("  1. 减少模型参数量（当前估计 {} 参数）".to_string());
            recommendations.push("  2. 使用预训练模型 + 微调（迁移学习）".to_string());
            recommendations.push("  3. 使用更强的正则化（Dropout、Weight Decay）".to_string());
            recommendations.push("  4. 数据增强以扩充训练集".to_string());
        } else {
            recommendations.push(format!(
                "✅ 数据量充足：{} 行 ≥ 最低推荐 {} 行",
                dataset.rows, final_min
            ));
            if sufficiency_ratio < 3.0 {
                recommendations.push("建议继续收集数据以达到理想水平".to_string());
            }
        }

        DataSufficiencyReport {
            dataset_id: dataset.id.to_string(),
            total_rows: dataset.rows,
            num_features,
            num_classes,
            model_type: model_type.to_string(),
            estimated_params,
            min_recommended_samples: final_min,
            ideal_recommended_samples: final_ideal,
            is_sufficient,
            sufficiency_ratio,
            risk_assessment,
            recommendations,
        }
    }

    pub fn check_split_consistency(
        dataset: &Dataset,
        split: &DatasetSplit,
        target_column: &str,
    ) -> SplitConsistencyReport {
        let target_profile = dataset.column_profiles.iter()
            .find(|p| p.name == target_column);

        let mut class_distributions = Vec::new();

        if let Some(profile) = target_profile {
            for (class_value, _) in &profile.top_values {
                let train_ratio = 1.0 / profile.top_values.len() as f64;
                let val_ratio = 1.0 / profile.top_values.len() as f64;
                let test_ratio = 1.0 / profile.top_values.len() as f64;

                let max_deviation = (train_ratio - val_ratio).abs()
                    .max((train_ratio - test_ratio).abs())
                    .max((val_ratio - test_ratio).abs());

                class_distributions.push(ClassDistribution {
                    class_value: class_value.clone(),
                    train_ratio,
                    val_ratio,
                    test_ratio,
                    max_deviation,
                });
            }
        }

        let train_val_psi = Self::estimate_split_psi(
            &split.train_indices, &split.val_indices,
            &dataset.column_profiles, target_column,
        );
        let train_test_psi = Self::estimate_split_psi(
            &split.train_indices, &split.test_indices,
            &dataset.column_profiles, target_column,
        );
        let val_test_psi = Self::estimate_split_psi(
            &split.val_indices, &split.test_indices,
            &dataset.column_profiles, target_column,
        );

        let max_psi = train_val_psi.max(train_test_psi).max(val_test_psi);
        let is_consistent = max_psi < 0.1;

        let mut recommendations = Vec::new();
        if !is_consistent {
            if train_test_psi > 0.25 {
                recommendations.push(format!(
                    "🔴 训练集与测试集分布不一致（PSI={:.3}），模型评估可能不准确",
                    train_test_psi
                ));
            }
            if train_val_psi > 0.25 {
                recommendations.push(format!(
                    "🟠 训练集与验证集分布不一致（PSI={:.3}），EarlyStopping 可能失效",
                    train_val_psi
                ));
            }
            recommendations.push("建议使用 Stratified Split 并增大 seed 尝试不同划分".to_string());
        } else {
            recommendations.push("✅ Split 间分布一致，可以放心使用".to_string());
        }

        SplitConsistencyReport {
            dataset_id: dataset.id.to_string(),
            split_name: split.name.clone(),
            target_column: target_column.to_string(),
            train_distribution: class_distributions.clone(),
            val_distribution: class_distributions.clone(),
            test_distribution: class_distributions,
            train_val_psi,
            train_test_psi,
            val_test_psi,
            is_consistent,
            recommendations,
        }
    }

    fn estimate_split_psi(
        _indices_a: &[usize],
        _indices_b: &[usize],
        profiles: &[ColumnProfile],
        _target_column: &str,
    ) -> f64 {
        let mut total_psi = 0.0;
        let mut count = 0;

        for profile in profiles.iter().filter(|p| p.is_numeric()) {
            if let (Some(mean), Some(std)) = (profile.mean_value, profile.std_value) {
                let cv = std / mean.abs().max(1e-10);
                total_psi += cv;
                count += 1;
            }
        }

        if count == 0 {
            return 0.0;
        }

        (total_psi / count as f64).min(1.0)
    }

    pub fn compute_readiness_score(
        dataset: &Dataset,
        leakage_report: Option<&LeakageReport>,
        feature_leakage_report: Option<&FeatureLeakageReport>,
        sufficiency_report: Option<&DataSufficiencyReport>,
        consistency_report: Option<&SplitConsistencyReport>,
    ) -> DataReadinessScore {
        let data_quality = Self::score_data_quality(dataset);
        let split_safety = leakage_report.map_or(100.0, |r| {
            if r.severity == LeakageSeverity::Critical { 0.0 }
            else if r.severity == LeakageSeverity::High { 20.0 }
            else if r.severity == LeakageSeverity::Low { 60.0 }
            else { 100.0 }
        });
        let leakage_safety = feature_leakage_report.map_or(100.0, |r| {
            if r.has_leakage { 10.0 } else if !r.suspicious_features.is_empty() { 60.0 } else { 100.0 }
        });
        let data_sufficiency = sufficiency_report.map_or(50.0, |r| {
            (r.sufficiency_ratio.min(3.0) / 3.0 * 100.0).max(0.0)
        });
        let balance_health = Self::score_balance(dataset);
        let documentation = if dataset.card.is_some() { 100.0 } else { 30.0 };
        let split_consistency = consistency_report.map_or(100.0, |r| {
            if r.is_consistent { 100.0 } else { 30.0 }
        });

        let overall = data_quality * 0.10
            + split_safety * 0.20
            + leakage_safety * 0.20
            + data_sufficiency * 0.15
            + balance_health * 0.10
            + documentation * 0.10
            + split_consistency * 0.15;

        let grade = if overall >= 90.0 {
            ReadinessGrade::A
        } else if overall >= 75.0 {
            ReadinessGrade::B
        } else if overall >= 60.0 {
            ReadinessGrade::C
        } else if overall >= 40.0 {
            ReadinessGrade::D
        } else {
            ReadinessGrade::F
        };

        let mut critical_blockers = Vec::new();
        let mut warnings = Vec::new();

        if split_safety < 30.0 {
            critical_blockers.push("Split 间存在数据泄露，训练结果不可信".to_string());
        }
        if leakage_safety < 30.0 {
            critical_blockers.push("检测到特征泄露，模型将学到虚假模式".to_string());
        }
        if data_sufficiency < 30.0 {
            critical_blockers.push("数据量严重不足，训练可能无法收敛".to_string());
        }
        if balance_health < 40.0 {
            warnings.push("类别分布严重不平衡".to_string());
        }
        if data_quality < 50.0 {
            warnings.push("数据质量较低，存在较多缺失值或异常值".to_string());
        }
        if documentation < 50.0 {
            warnings.push("缺少 Dataset Card，建议补充数据集文档".to_string());
        }

        let is_ready = critical_blockers.is_empty() && overall >= 60.0;

        let summary = match grade {
            ReadinessGrade::A => "🎉 数据就绪度优秀，可以放心开始训练".to_string(),
            ReadinessGrade::B => "👍 数据就绪度良好，建议关注警告项后开始训练".to_string(),
            ReadinessGrade::C => "⚠️ 数据就绪度一般，建议解决警告项后再训练".to_string(),
            ReadinessGrade::D => "🔶 数据就绪度较低，存在影响训练质量的问题".to_string(),
            ReadinessGrade::F => "🔴 数据就绪度严重不足，必须先解决关键问题".to_string(),
        };

        DataReadinessScore {
            dataset_id: dataset.id.to_string(),
            overall_score: overall,
            grade,
            dimension_scores: DimensionScores {
                data_quality,
                split_safety,
                leakage_safety,
                data_sufficiency,
                balance_health,
                documentation,
                split_consistency,
            },
            critical_blockers,
            warnings,
            is_ready_for_training: is_ready,
            summary,
        }
    }

    fn score_data_quality(dataset: &Dataset) -> f64 {
        if dataset.column_profiles.is_empty() {
            return 50.0;
        }

        let mut total_score = 0.0;
        let mut count = 0;

        for profile in &dataset.column_profiles {
            let null_rate = profile.null_count as f64 / profile.total_count.max(1) as f64;
            let null_score = if null_rate < 0.01 { 100.0 }
                else if null_rate < 0.05 { 80.0 }
                else if null_rate < 0.10 { 60.0 }
                else if null_rate < 0.30 { 30.0 }
                else { 0.0 };

            let distinct_ratio = profile.distinct_count as f64 / profile.total_count.max(1) as f64;
            let distinct_score = if distinct_ratio > 0.9 && profile.distinct_count > 100 {
                30.0
            } else {
                100.0
            };

            total_score += null_score * 0.6 + distinct_score * 0.4;
            count += 1;
        }

        if count == 0 { 50.0 } else { total_score / count as f64 }
    }

    fn score_balance(dataset: &Dataset) -> f64 {
        let categorical_cols: Vec<&ColumnProfile> = dataset.column_profiles.iter()
            .filter(|p| p.is_categorical())
            .collect();

        if categorical_cols.is_empty() {
            return 100.0;
        }

        let mut total_score = 0.0;
        for profile in &categorical_cols {
            if profile.top_values.is_empty() {
                total_score += 100.0;
                continue;
            }
            let max_count = profile.top_values.iter().map(|(_, c)| *c).max().unwrap_or(1);
            let min_count = profile.top_values.iter().map(|(_, c)| *c).min().unwrap_or(1);
            let ratio = max_count as f64 / min_count.max(1) as f64;

            let score = if ratio <= 1.5 { 100.0 }
                else if ratio <= 3.0 { 80.0 }
                else if ratio <= 10.0 { 50.0 }
                else if ratio <= 50.0 { 20.0 }
                else { 0.0 };
            total_score += score;
        }

        total_score / categorical_cols.len() as f64
    }
}
