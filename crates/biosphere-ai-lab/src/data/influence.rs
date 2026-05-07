use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceAnalysisReport {
    pub dataset_id: String,
    pub model_id: Option<String>,
    pub total_samples: usize,
    pub influence_scores: Vec<SampleInfluence>,
    pub most_helpful: Vec<SampleInfluence>,
    pub most_harmful: Vec<SampleInfluence>,
    pub influence_distribution: InfluenceDistribution,
    pub class_level_influence: Vec<ClassInfluence>,
    pub data_attribution_summary: DataAttributionSummary,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleInfluence {
    pub index: usize,
    pub influence_score: f64,
    pub influence_type: InfluenceType,
    pub loss_change_estimate: f64,
    pub label: Option<String>,
    pub prediction: Option<String>,
    pub is_correct: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfluenceType {
    StronglyHelpful,
    Helpful,
    Neutral,
    Harmful,
    StronglyHarmful,
}

impl std::fmt::Display for InfluenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StronglyHelpful => write!(f, "strongly_helpful"),
            Self::Helpful => write!(f, "helpful"),
            Self::Neutral => write!(f, "neutral"),
            Self::Harmful => write!(f, "harmful"),
            Self::StronglyHarmful => write!(f, "strongly_harmful"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceDistribution {
    pub strongly_helpful_count: usize,
    pub helpful_count: usize,
    pub neutral_count: usize,
    pub harmful_count: usize,
    pub strongly_harmful_count: usize,
    pub mean_influence: f64,
    pub std_influence: f64,
    pub median_influence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfluence {
    pub class_name: String,
    pub avg_influence: f64,
    pub helpful_ratio: f64,
    pub harmful_ratio: f64,
    pub most_influential_indices: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAttributionSummary {
    pub total_positive_influence: f64,
    pub total_negative_influence: f64,
    pub net_influence: f64,
    pub data_efficiency_score: f64,
    pub estimated_pruning_benefit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceConfig {
    pub method: InfluenceMethod,
    pub top_k: usize,
    pub influence_threshold: f64,
    pub use_absolute: bool,
}

impl Default for InfluenceConfig {
    fn default() -> Self {
        Self {
            method: InfluenceMethod::TracIn,
            top_k: 50,
            influence_threshold: 0.01,
            use_absolute: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfluenceMethod {
    TracIn,
    LeaveOneOut,
    GradientSimilarity,
    LossDifference,
}

impl std::fmt::Display for InfluenceMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TracIn => write!(f, "tracin"),
            Self::LeaveOneOut => write!(f, "leave_one_out"),
            Self::GradientSimilarity => write!(f, "gradient_similarity"),
            Self::LossDifference => write!(f, "loss_difference"),
        }
    }
}

pub struct InfluenceAnalyzer;

impl InfluenceAnalyzer {
    pub fn analyze_tracin(
        dataset_id: &str,
        model_id: Option<&str>,
        sample_gradients: &[Vec<f64>],
        checkpoint_gradients: &[Vec<Vec<f64>>],
        labels: &[String],
        predictions: &[String],
        config: &InfluenceConfig,
    ) -> InfluenceAnalysisReport {
        let total = sample_gradients.len();
        let num_checkpoints = checkpoint_gradients.len();

        let mut influence_scores: Vec<f64> = vec![0.0; total];

        for (sample_idx, sample_grad) in sample_gradients.iter().enumerate() {
            let mut total_influence = 0.0;

            for ckpt_grads in checkpoint_gradients {
                if let Some(ckpt_grad) = ckpt_grads.get(sample_idx) {
                    let dot = Self::dot_product(sample_grad, ckpt_grad);
                    total_influence += dot;
                }
            }

            if num_checkpoints > 0 {
                total_influence /= num_checkpoints as f64;
            }

            influence_scores[sample_idx] = total_influence;
        }

        Self::build_report(
            dataset_id, model_id, &influence_scores, labels, predictions, total, config,
        )
    }

    pub fn analyze_loo(
        dataset_id: &str,
        model_id: Option<&str>,
        full_model_loss: f64,
        loo_losses: &[f64],
        labels: &[String],
        predictions: &[String],
        config: &InfluenceConfig,
    ) -> InfluenceAnalysisReport {
        let total = loo_losses.len();

        let influence_scores: Vec<f64> = loo_losses.iter()
            .map(|&loo_loss| full_model_loss - loo_loss)
            .collect();

        Self::build_report(
            dataset_id, model_id, &influence_scores, labels, predictions, total, config,
        )
    }

    pub fn analyze_gradient_similarity(
        dataset_id: &str,
        model_id: Option<&str>,
        sample_gradients: &[Vec<f64>],
        test_gradient: &[f64],
        labels: &[String],
        predictions: &[String],
        config: &InfluenceConfig,
    ) -> InfluenceAnalysisReport {
        let total = sample_gradients.len();

        let influence_scores: Vec<f64> = sample_gradients.iter()
            .map(|grad| Self::dot_product(grad, test_gradient))
            .collect();

        Self::build_report(
            dataset_id, model_id, &influence_scores, labels, predictions, total, config,
        )
    }

    pub fn analyze_loss_difference(
        dataset_id: &str,
        model_id: Option<&str>,
        per_sample_losses: &[f64],
        labels: &[String],
        predictions: &[String],
        config: &InfluenceConfig,
    ) -> InfluenceAnalysisReport {
        let total = per_sample_losses.len();
        let mean_loss = per_sample_losses.iter().sum::<f64>() / total.max(1) as f64;

        let influence_scores: Vec<f64> = per_sample_losses.iter()
            .map(|&loss| mean_loss - loss)
            .collect();

        Self::build_report(
            dataset_id, model_id, &influence_scores, labels, predictions, total, config,
        )
    }

    fn build_report(
        dataset_id: &str,
        model_id: Option<&str>,
        influence_scores: &[f64],
        labels: &[String],
        predictions: &[String],
        total: usize,
        config: &InfluenceConfig,
    ) -> InfluenceAnalysisReport {
        let mean = influence_scores.iter().sum::<f64>() / total.max(1) as f64;
        let variance = influence_scores.iter()
            .map(|s| (s - mean).powi(2))
            .sum::<f64>() / total.max(1) as f64;
        let std = variance.sqrt();

        let mut indexed: Vec<(usize, f64)> = influence_scores.iter()
            .enumerate()
            .map(|(i, &s)| (i, s))
            .collect();

        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut sorted_scores = influence_scores.to_vec();
        sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted_scores[total / 2];

        let mut sample_influences = Vec::new();
        let mut dist = InfluenceDistribution {
            strongly_helpful_count: 0,
            helpful_count: 0,
            neutral_count: 0,
            harmful_count: 0,
            strongly_harmful_count: 0,
            mean_influence: mean,
            std_influence: std,
            median_influence: median,
        };

        for (i, &score) in influence_scores.iter().enumerate() {
            let influence_type = if score > 2.0 * std {
                InfluenceType::StronglyHelpful
            } else if score > std {
                InfluenceType::Helpful
            } else if score < -2.0 * std {
                InfluenceType::StronglyHarmful
            } else if score < -std {
                InfluenceType::Harmful
            } else {
                InfluenceType::Neutral
            };

            match influence_type {
                InfluenceType::StronglyHelpful => dist.strongly_helpful_count += 1,
                InfluenceType::Helpful => dist.helpful_count += 1,
                InfluenceType::Neutral => dist.neutral_count += 1,
                InfluenceType::Harmful => dist.harmful_count += 1,
                InfluenceType::StronglyHarmful => dist.strongly_harmful_count += 1,
            }

            let is_correct = if i < labels.len() && i < predictions.len() {
                Some(labels[i] == predictions[i])
            } else {
                None
            };

            sample_influences.push(SampleInfluence {
                index: i,
                influence_score: score,
                influence_type,
                loss_change_estimate: -score,
                label: labels.get(i).cloned(),
                prediction: predictions.get(i).cloned(),
                is_correct,
            });
        }

        let most_helpful: Vec<SampleInfluence> = indexed.iter()
            .take(config.top_k)
            .map(|&(i, _)| sample_influences[i].clone())
            .collect();

        let most_harmful: Vec<SampleInfluence> = indexed.iter()
            .rev()
            .take(config.top_k)
            .map(|&(i, _)| sample_influences[i].clone())
            .collect();

        let class_level = Self::compute_class_influence(&sample_influences, labels);

        let total_pos: f64 = influence_scores.iter().filter(|&&s| s > 0.0).sum();
        let total_neg: f64 = influence_scores.iter().filter(|&&s| s < 0.0).sum();
        let net = total_pos + total_neg;

        let harmful_ratio = dist.harmful_count as f64 / total.max(1) as f64;
        let data_efficiency = 1.0 - harmful_ratio;

        let estimated_pruning_benefit = if dist.strongly_harmful_count > 0 {
            (dist.strongly_harmful_count as f64 / total as f64) * 0.05
        } else {
            0.0
        };

        let attribution = DataAttributionSummary {
            total_positive_influence: total_pos,
            total_negative_influence: total_neg,
            net_influence: net,
            data_efficiency_score: data_efficiency,
            estimated_pruning_benefit,
        };

        let recommendations = Self::generate_influence_recommendations(
            &dist, &class_level, &most_harmful, &attribution,
        );

        InfluenceAnalysisReport {
            dataset_id: dataset_id.to_string(),
            model_id: model_id.map(|s| s.to_string()),
            total_samples: total,
            influence_scores: sample_influences,
            most_helpful,
            most_harmful,
            influence_distribution: dist,
            class_level_influence: class_level,
            data_attribution_summary: attribution,
            recommendations,
        }
    }

    fn dot_product(a: &[f64], b: &[f64]) -> f64 {
        let len = a.len().min(b.len());
        let mut sum = 0.0;
        for i in 0..len {
            sum += a[i] * b[i];
        }
        sum
    }

    fn compute_class_influence(
        influences: &[SampleInfluence],
        _labels: &[String],
    ) -> Vec<ClassInfluence> {
        let mut class_data: HashMap<&str, (Vec<f64>, Vec<usize>)> = HashMap::new();

        for inf in influences {
            if let Some(ref label) = inf.label {
                let entry = class_data.entry(label.as_str()).or_default();
                entry.0.push(inf.influence_score);
                entry.1.push(inf.index);
            }
        }

        let mut results: Vec<ClassInfluence> = class_data.iter()
            .map(|(class, (scores, indices))| {
                let avg = scores.iter().sum::<f64>() / scores.len().max(1) as f64;
                let helpful = scores.iter().filter(|&&s| s > 0.0).count();
                let harmful = scores.iter().filter(|&&s| s < 0.0).count();
                let total = scores.len();

                let mut sorted_indices = indices.clone();
                sorted_indices.sort_by(|&a, &b| {
                    influences[b].influence_score.partial_cmp(&influences[a].influence_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                ClassInfluence {
                    class_name: class.to_string(),
                    avg_influence: avg,
                    helpful_ratio: helpful as f64 / total.max(1) as f64,
                    harmful_ratio: harmful as f64 / total.max(1) as f64,
                    most_influential_indices: sorted_indices.into_iter().take(10).collect(),
                }
            })
            .collect();

        results.sort_by(|a, b| b.avg_influence.partial_cmp(&a.avg_influence).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    fn generate_influence_recommendations(
        dist: &InfluenceDistribution,
        class_influence: &[ClassInfluence],
        most_harmful: &[SampleInfluence],
        attribution: &DataAttributionSummary,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        let total = dist.strongly_helpful_count + dist.helpful_count
            + dist.neutral_count + dist.harmful_count + dist.strongly_harmful_count;

        if dist.strongly_harmful_count == 0 && dist.harmful_count == 0 {
            recs.push("✅ 所有训练样本对模型性能有正面或中性影响".to_string());
        } else {
            let harmful_pct = (dist.harmful_count + dist.strongly_harmful_count) as f64
                / total.max(1) as f64 * 100.0;
            recs.push(format!(
                "⚠️ {:.1}% 的训练样本对模型有负面影响（{} 有害 + {} 严重有害）",
                harmful_pct, dist.harmful_count, dist.strongly_harmful_count
            ));
        }

        if dist.strongly_harmful_count > 0 {
            recs.push(format!(
                "🔴 发现 {} 个严重有害样本，建议移除或重新标注",
                dist.strongly_harmful_count
            ));
        }

        let worst_classes: Vec<&ClassInfluence> = class_influence.iter()
            .filter(|c| c.avg_influence < -0.01)
            .collect();

        if !worst_classes.is_empty() {
            let names: Vec<String> = worst_classes.iter()
                .map(|c| format!("{}（有害比 {:.1}%）", c.class_name, c.harmful_ratio * 100.0))
                .collect();
            recs.push(format!("📊 以下类别整体影响为负: {}", names.join(", ")));
        }

        if !most_harmful.is_empty() {
            let incorrect_harmful = most_harmful.iter()
                .filter(|s| s.is_correct == Some(false))
                .count();
            if incorrect_harmful > 0 {
                recs.push(format!(
                    "💡 最有害的 {} 个样本中，{} 个是模型预测错误的，可能是标注错误",
                    most_harmful.len(), incorrect_harmful
                ));
            }
        }

        if attribution.estimated_pruning_benefit > 0.01 {
            recs.push(format!(
                "📈 移除有害样本预计可提升准确率约 {:.1}%",
                attribution.estimated_pruning_benefit * 100.0
            ));
        }

        if attribution.data_efficiency_score < 0.7 {
            recs.push("💡 数据效率较低，建议：1) 移除有害样本 2) 增加高质量样本 3) 使用课程学习".to_string());
        }

        recs
    }
}
