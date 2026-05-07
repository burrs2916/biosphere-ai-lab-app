use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelErrorReport {
    pub dataset_id: String,
    pub total_samples: usize,
    pub num_classes: usize,
    pub class_names: Vec<String>,
    pub label_quality_score: f64,
    pub estimated_noise_rate: f64,
    pub noise_indices: Vec<usize>,
    pub noise_count: usize,
    pub noise_ratio: f64,
    pub per_class_errors: Vec<ClassErrorStats>,
    pub confident_joint: Vec<Vec<usize>>,
    pub label_issue_details: Vec<LabelIssueDetail>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassErrorStats {
    pub class_name: String,
    pub total_samples: usize,
    pub estimated_errors: usize,
    pub error_rate: f64,
    pub most_confused_with: Vec<ConfusionTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfusionTarget {
    pub target_class: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelIssueDetail {
    pub index: usize,
    pub given_label: String,
    pub given_label_idx: usize,
    pub predicted_label: String,
    pub predicted_label_idx: usize,
    pub confidence: f64,
    pub issue_type: LabelIssueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelIssueType {
    LikelyMislabeled,
    Ambiguous,
    OutlierInClass,
    BoundaryCase,
}

impl std::fmt::Display for LabelIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LikelyMislabeled => write!(f, "likely_mislabeled"),
            Self::Ambiguous => write!(f, "ambiguous"),
            Self::OutlierInClass => write!(f, "outlier_in_class"),
            Self::BoundaryCase => write!(f, "boundary_case"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentLearningConfig {
    pub label_column: String,
    pub class_names: Option<Vec<String>>,
    pub threshold_method: ThresholdMethod,
    pub prune_method: PruneMethod,
    pub min_confidence: f64,
    pub max_label_issues: usize,
}

impl Default for ConfidentLearningConfig {
    fn default() -> Self {
        Self {
            label_column: "label".to_string(),
            class_names: None,
            threshold_method: ThresholdMethod::ConfidentJoint,
            prune_method: PruneMethod::ByNoiseRate,
            min_confidence: 0.5,
            max_label_issues: 1000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdMethod {
    ConfidentJoint,
    Calibrated,
    SelfConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PruneMethod {
    ByClass,
    ByNoiseRate,
    Both,
}

pub struct ConfidentLearning;

impl ConfidentLearning {
    pub fn analyze(
        dataset_id: &str,
        labels: &[usize],
        probabilities: &[Vec<f64>],
        config: &ConfidentLearningConfig,
    ) -> LabelErrorReport {
        let total_samples = labels.len();
        let num_classes = probabilities.first().map(|p| p.len()).unwrap_or(0);

        let class_names: Vec<String> = match &config.class_names {
            Some(names) if names.len() == num_classes => names.clone(),
            _ => (0..num_classes).map(|i| format!("class_{}", i)).collect(),
        };

        let confident_joint = Self::compute_confident_joint(
            labels, probabilities, num_classes, &config.threshold_method,
        );

        let (noise_indices, label_issue_details) = Self::identify_label_issues(
            labels, probabilities, &confident_joint, num_classes, &class_names, config,
        );

        let noise_count = noise_indices.len();
        let noise_ratio = noise_count as f64 / total_samples.max(1) as f64;

        let estimated_noise_rate = Self::estimate_noise_rate(&confident_joint, total_samples);

        let per_class_errors = Self::compute_per_class_errors(
            &confident_joint, &class_names, labels,
        );

        let label_quality_score = Self::compute_quality_score(
            noise_ratio, estimated_noise_rate, &per_class_errors,
        );

        let recommendations = Self::generate_recommendations(
            noise_ratio, estimated_noise_rate, &per_class_errors, &label_issue_details,
        );

        LabelErrorReport {
            dataset_id: dataset_id.to_string(),
            total_samples,
            num_classes,
            class_names,
            label_quality_score,
            estimated_noise_rate,
            noise_indices,
            noise_count,
            noise_ratio,
            per_class_errors,
            confident_joint,
            label_issue_details,
            recommendations,
        }
    }

    fn compute_confident_joint(
        labels: &[usize],
        probabilities: &[Vec<f64>],
        num_classes: usize,
        threshold_method: &ThresholdMethod,
    ) -> Vec<Vec<usize>> {
        let mut joint = vec![vec![0usize; num_classes]; num_classes];

        for (i, probs) in probabilities.iter().enumerate() {
            let given_label = labels[i];
            let predicted_label = probs.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx)
                .unwrap_or(0);

            let threshold = match threshold_method {
                ThresholdMethod::ConfidentJoint => {
                    let avg_per_class = 1.0 / num_classes as f64;
                    avg_per_class
                }
                ThresholdMethod::Calibrated => {
                    let pred_conf = probs[predicted_label];
                    let given_conf = probs[given_label];
                    (pred_conf + given_conf) / 2.0
                }
                ThresholdMethod::SelfConfidence => {
                    probs[predicted_label]
                }
            };

            let is_confident = probs[predicted_label] >= threshold;

            if is_confident {
                joint[given_label][predicted_label] += 1;
            }
        }

        joint
    }

    fn identify_label_issues(
        labels: &[usize],
        probabilities: &[Vec<f64>],
        confident_joint: &[Vec<usize>],
        num_classes: usize,
        class_names: &[String],
        config: &ConfidentLearningConfig,
    ) -> (Vec<usize>, Vec<LabelIssueDetail>) {
        let mut class_thresholds: Vec<f64> = vec![0.0; num_classes];

        for i in 0..num_classes {
            let total_in_class: usize = confident_joint[i].iter().sum();
            let self_count = confident_joint[i][i];
            if total_in_class > 0 {
                class_thresholds[i] = self_count as f64 / total_in_class.max(1) as f64;
            }
        }

        let mut noise_indices = Vec::new();
        let mut issue_details = Vec::new();

        for (i, probs) in probabilities.iter().enumerate() {
            let given_label = labels[i];
            let predicted_label = probs.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx)
                .unwrap_or(0);

            if given_label == predicted_label {
                continue;
            }

            let pred_conf = probs[predicted_label];
            let given_conf = probs[given_label];

            if pred_conf < config.min_confidence {
                continue;
            }

            let margin = pred_conf - given_conf;

            let issue_type = if margin > 0.3 {
                LabelIssueType::LikelyMislabeled
            } else if margin > 0.15 {
                LabelIssueType::Ambiguous
            } else if class_thresholds[given_label] < 0.5 {
                LabelIssueType::OutlierInClass
            } else {
                LabelIssueType::BoundaryCase
            };

            let should_include = match config.prune_method {
                PruneMethod::ByClass => {
                    margin > 0.1
                }
                PruneMethod::ByNoiseRate => {
                    margin > 0.05
                }
                PruneMethod::Both => {
                    margin > 0.05
                }
            };

            if should_include {
                noise_indices.push(i);

                if issue_details.len() < config.max_label_issues {
                    issue_details.push(LabelIssueDetail {
                        index: i,
                        given_label: class_names[given_label].clone(),
                        given_label_idx: given_label,
                        predicted_label: class_names[predicted_label].clone(),
                        predicted_label_idx: predicted_label,
                        confidence: pred_conf,
                        issue_type,
                    });
                }
            }
        }

        (noise_indices, issue_details)
    }

    fn estimate_noise_rate(confident_joint: &[Vec<usize>], total_samples: usize) -> f64 {
        if total_samples == 0 {
            return 0.0;
        }

        let mut off_diagonal = 0usize;
        let mut total_confident = 0usize;

        for i in 0..confident_joint.len() {
            for j in 0..confident_joint[i].len() {
                total_confident += confident_joint[i][j];
                if i != j {
                    off_diagonal += confident_joint[i][j];
                }
            }
        }

        if total_confident == 0 {
            return 0.0;
        }

        off_diagonal as f64 / total_confident as f64
    }

    fn compute_per_class_errors(
        confident_joint: &[Vec<usize>],
        class_names: &[String],
        labels: &[usize],
    ) -> Vec<ClassErrorStats> {
        let num_classes = confident_joint.len();
        let mut class_counts = vec![0usize; num_classes];
        for &label in labels {
            if label < num_classes {
                class_counts[label] += 1;
            }
        }

        let mut stats = Vec::new();

        for i in 0..num_classes {
            let total = class_counts[i];
            let mut errors = 0usize;
            let mut confusion: HashMap<usize, usize> = HashMap::new();

            for j in 0..num_classes {
                if i != j {
                    errors += confident_joint[i][j];
                    if confident_joint[i][j] > 0 {
                        confusion.insert(j, confident_joint[i][j]);
                    }
                }
            }

            let error_rate = if total > 0 {
                errors as f64 / total as f64
            } else {
                0.0
            };

            let mut confused_with: Vec<(usize, usize)> = confusion.into_iter().collect();
            confused_with.sort_by(|a, b| b.1.cmp(&a.1));
            let most_confused_with: Vec<ConfusionTarget> = confused_with.iter()
                .take(3)
                .map(|(idx, count)| ConfusionTarget {
                    target_class: class_names[*idx].clone(),
                    count: *count,
                })
                .collect();

            stats.push(ClassErrorStats {
                class_name: class_names[i].clone(),
                total_samples: total,
                estimated_errors: errors,
                error_rate,
                most_confused_with,
            });
        }

        stats
    }

    fn compute_quality_score(
        noise_ratio: f64,
        estimated_noise_rate: f64,
        per_class_errors: &[ClassErrorStats],
    ) -> f64 {
        let noise_penalty = (noise_ratio * 0.5 + estimated_noise_rate * 0.5).min(1.0);

        let max_class_error = per_class_errors.iter()
            .map(|c| c.error_rate)
            .fold(0.0, f64::max);

        let avg_class_error = if per_class_errors.is_empty() {
            0.0
        } else {
            per_class_errors.iter().map(|c| c.error_rate).sum::<f64>()
                / per_class_errors.len() as f64
        };

        let class_imbalance_penalty = if max_class_error > 0.0 && avg_class_error > 0.0 {
            (max_class_error - avg_class_error).abs()
        } else {
            0.0
        };

        let raw_score = 1.0 - (noise_penalty * 0.6 + class_imbalance_penalty * 0.2 + max_class_error * 0.2);

        raw_score.max(0.0).min(1.0)
    }

    fn generate_recommendations(
        noise_ratio: f64,
        estimated_noise_rate: f64,
        per_class_errors: &[ClassErrorStats],
        issue_details: &[LabelIssueDetail],
    ) -> Vec<String> {
        let mut recs = Vec::new();

        if estimated_noise_rate < 0.02 {
            recs.push("✅ 标签质量优秀，估计噪声率 < 2%".to_string());
        } else if estimated_noise_rate < 0.05 {
            recs.push("🟢 标签质量良好，估计噪声率 < 5%".to_string());
        } else if estimated_noise_rate < 0.10 {
            recs.push("🟡 标签存在一定噪声（~{:.0}%），建议人工复核高置信度错误".to_string());
        } else if estimated_noise_rate < 0.20 {
            recs.push(format!(
                "🟠 标签噪声较高（~{:.0}%），建议：1) 对疑似错误样本重新标注 2) 使用标签平滑训练",
                estimated_noise_rate * 100.0
            ));
        } else {
            recs.push(format!(
                "🔴 标签噪声严重（~{:.0}%），强烈建议：1) 重新标注数据集 2) 使用噪声鲁棒损失函数 3) 考虑主动学习修正",
                estimated_noise_rate * 100.0
            ));
        }

        let likely_mislabeled = issue_details.iter()
            .filter(|d| d.issue_type == LabelIssueType::LikelyMislabeled)
            .count();

        if likely_mislabeled > 0 {
            recs.push(format!(
                "⚠️ 发现 {} 个高置信度标签错误（likely mislabeled），建议优先复核",
                likely_mislabeled
            ));
        }

        let worst_classes: Vec<&ClassErrorStats> = per_class_errors.iter()
            .filter(|c| c.error_rate > 0.1)
            .collect();

        if !worst_classes.is_empty() {
            let class_list: Vec<String> = worst_classes.iter()
                .map(|c| format!("{}（错误率 {:.1}%）", c.class_name, c.error_rate * 100.0))
                .collect();
            recs.push(format!("📊 以下类别标签质量较差: {}", class_list.join(", ")));
        }

        for class_stat in per_class_errors.iter().filter(|c| c.error_rate > 0.15) {
            if let Some(top_confusion) = class_stat.most_confused_with.first() {
                recs.push(format!(
                    "  - {} 最易与 {} 混淆（{} 例）",
                    class_stat.class_name, top_confusion.target_class, top_confusion.count
                ));
            }
        }

        if noise_ratio > 0.15 {
            recs.push("💡 建议使用 Cleanlab 式训练策略：1) Co-teaching 2) 置信度加权损失 3) 标签修正".to_string());
        }

        recs
    }

    pub fn compute_label_quality_summary(
        labels: &[usize],
        num_classes: usize,
    ) -> LabelQualitySummary {
        let mut class_counts = vec![0usize; num_classes];
        for &label in labels {
            if label < num_classes {
                class_counts[label] += 1;
            }
        }

        let total = labels.len();
        let max_count = class_counts.iter().max().copied().unwrap_or(0);
        let min_count = class_counts.iter().min().copied().unwrap_or(0);

        let balance_ratio = if max_count > 0 {
            min_count as f64 / max_count as f64
        } else {
            0.0
        };

        let mut distribution = Vec::new();
        for (i, &count) in class_counts.iter().enumerate() {
            distribution.push(ClassCount {
                class_index: i,
                count,
                ratio: if total > 0 { count as f64 / total as f64 } else { 0.0 },
            });
        }

        LabelQualitySummary {
            total_samples: total,
            num_classes,
            class_distribution: distribution,
            balance_ratio,
            is_balanced: balance_ratio > 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelQualitySummary {
    pub total_samples: usize,
    pub num_classes: usize,
    pub class_distribution: Vec<ClassCount>,
    pub balance_ratio: f64,
    pub is_balanced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassCount {
    pub class_index: usize,
    pub count: usize,
    pub ratio: f64,
}
