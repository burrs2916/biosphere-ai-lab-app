use serde::{Deserialize, Serialize};

use crate::domain::dataset::aggregate::{ColumnProfile, DatasetSummary};

use super::training_plan::{PlanType, TrainingPlan};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRecommendation {
    pub dataset_id: String,
    pub dataset_name: String,
    pub version: String,
    pub score: f64,
    pub reasons: Vec<String>,
    pub suitability: SuitabilityLevel,
    pub match_details: MatchDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SuitabilityLevel {
    Excellent,
    Good,
    Fair,
    Poor,
}

impl SuitabilityLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 80.0 {
            SuitabilityLevel::Excellent
        } else if score >= 60.0 {
            SuitabilityLevel::Good
        } else if score >= 40.0 {
            SuitabilityLevel::Fair
        } else {
            SuitabilityLevel::Poor
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDetails {
    pub type_match: f64,
    pub size_match: f64,
    pub quality_match: f64,
    pub feature_match: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationResult {
    pub plan_name: String,
    pub plan_type: String,
    pub recommendations: Vec<DatasetRecommendation>,
    pub total_datasets_available: usize,
    pub generated_at: String,
}

pub struct DatasetRecommender;

impl DatasetRecommender {
    pub fn recommend(
        plan: &TrainingPlan,
        available_datasets: &[DatasetSummary],
        quality_scores: &std::collections::HashMap<String, f64>,
    ) -> RecommendationResult {
        let mut recommendations: Vec<DatasetRecommendation> = available_datasets
            .iter()
            .map(|ds| {
                let type_match = Self::score_type_match(&plan.plan_type, ds);
                let size_match = Self::score_size_match(plan, ds);
                let quality_match = quality_scores.get(&ds.id.to_string()).copied().unwrap_or(50.0);
                let feature_match = Self::score_feature_match(plan, ds);

                let mut reasons: Vec<String> = Vec::new();

                if type_match >= 80.0 {
                    reasons.push(format!("数据集类型与训练计划 {} 高度匹配", plan.plan_type.plan_type_name()));
                } else if type_match >= 50.0 {
                    reasons.push("数据集类型与训练计划部分匹配".to_string());
                }

                if size_match >= 80.0 {
                    reasons.push(format!(
                        "数据规模 {} 行满足训练预算需求",
                        ds.rows
                    ));
                } else if size_match >= 50.0 {
                    reasons.push("数据规模基本满足需求".to_string());
                } else {
                    reasons.push("数据规模可能不足以支撑训练计划".to_string());
                }

                if quality_match >= 80.0 {
                    reasons.push(format!("数据质量评分 {:.0} 分，质量优秀", quality_match));
                } else if quality_match >= 60.0 {
                    reasons.push(format!("数据质量评分 {:.0} 分，质量良好", quality_match));
                }

                if feature_match >= 70.0 {
                    reasons.push("列特征与训练需求匹配良好".to_string());
                }

                let total_score = type_match * 0.35 + size_match * 0.30 + quality_match * 0.20 + feature_match * 0.15;

                DatasetRecommendation {
                    dataset_id: ds.id.to_string(),
                    dataset_name: ds.name.clone(),
                    version: ds.version.clone(),
                    score: total_score,
                    reasons,
                    suitability: SuitabilityLevel::from_score(total_score),
                    match_details: MatchDetails {
                        type_match,
                        size_match,
                        quality_match,
                        feature_match,
                    },
                }
            })
            .collect();

        recommendations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        RecommendationResult {
            plan_name: plan.name.clone(),
            plan_type: format!("{:?}", plan.plan_type),
            recommendations,
            total_datasets_available: available_datasets.len(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn score_type_match(plan_type: &PlanType, ds: &DatasetSummary) -> f64 {
        let ds_name_lower = ds.name.to_lowercase();
        let ds_tags_lower: Vec<String> = ds.tags.iter().map(|t| t.to_lowercase()).collect();

        match plan_type {
            PlanType::Pretraining => {
                let mut score = 50.0;
                if ds_name_lower.contains("pretrain") || ds_name_lower.contains("web") || ds_name_lower.contains("corpus") {
                    score += 30.0;
                }
                if ds_tags_lower.iter().any(|t| t.contains("pretraining") || t.contains("web") || t.contains("corpus")) {
                    score += 20.0;
                }
                if ds.rows > 1_000_000 {
                    score += 10.0;
                }
                (score as f64).min(100.0)
            }
            PlanType::FineTuning | PlanType::SFT => {
                let mut score = 50.0;
                if ds_name_lower.contains("instruction") || ds_name_lower.contains("sft") || ds_name_lower.contains("chat") {
                    score += 30.0;
                }
                if ds_tags_lower.iter().any(|t| t.contains("instruction") || t.contains("sft") || t.contains("chat") || t.contains("conversation")) {
                    score += 20.0;
                }
                if ds.rows >= 1_000 && ds.rows <= 1_000_000 {
                    score += 10.0;
                }
                (score as f64).min(100.0)
            }
            PlanType::RLHF | PlanType::DPO => {
                let mut score = 50.0;
                if ds_name_lower.contains("preference") || ds_name_lower.contains("rlhf") || ds_name_lower.contains("dpo") || ds_name_lower.contains("reward") {
                    score += 30.0;
                }
                if ds_tags_lower.iter().any(|t| t.contains("preference") || t.contains("rlhf") || t.contains("dpo") || t.contains("reward")) {
                    score += 20.0;
                }
                if ds.rows >= 1_000 && ds.rows <= 500_000 {
                    score += 10.0;
                }
                (score as f64).min(100.0)
            }
            PlanType::ContinuedPretraining => {
                let mut score = 50.0;
                if ds_name_lower.contains("domain") || ds_name_lower.contains("continued") || ds_name_lower.contains("specialized") {
                    score += 30.0;
                }
                if ds_tags_lower.iter().any(|t| t.contains("domain") || t.contains("specialized")) {
                    score += 20.0;
                }
                if ds.rows > 100_000 {
                    score += 10.0;
                }
                (score as f64).min(100.0)
            }
            PlanType::InstructionTuning => {
                let mut score = 50.0;
                if ds_name_lower.contains("instruction") || ds_name_lower.contains("task") || ds_name_lower.contains("qa") {
                    score += 30.0;
                }
                if ds_tags_lower.iter().any(|t| t.contains("instruction") || t.contains("task") || t.contains("qa")) {
                    score += 20.0;
                }
                if ds.rows >= 1_000 && ds.rows <= 500_000 {
                    score += 10.0;
                }
                (score as f64).min(100.0)
            }
            PlanType::Custom(_) => 50.0,
        }
    }

    fn score_size_match(plan: &TrainingPlan, ds: &DatasetSummary) -> f64 {
        let target_tokens = plan.data_budget.total_tokens_target as f64;
        let tokens_per_sample = plan.data_budget.tokens_per_sample_estimate.unwrap_or(2048) as f64;
        let target_samples = target_tokens / tokens_per_sample;

        let ds_rows = ds.rows as f64;

        if ds_rows >= target_samples * 2.0 {
            100.0
        } else if ds_rows >= target_samples {
            90.0
        } else if ds_rows >= target_samples * 0.5 {
            70.0
        } else if ds_rows >= target_samples * 0.2 {
            50.0
        } else if ds_rows >= target_samples * 0.1 {
            30.0
        } else {
            10.0
        }
    }

    fn score_feature_match(plan: &TrainingPlan, ds: &DatasetSummary) -> f64 {
        let mut score = 50.0;

        let has_text = ds.name.to_lowercase().contains("text")
            || ds.tags.iter().any(|t| t.to_lowercase().contains("text"));
        let has_image = ds.name.to_lowercase().contains("image")
            || ds.tags.iter().any(|t| t.to_lowercase().contains("image"));
        let has_multimodal = ds.name.to_lowercase().contains("multimodal")
            || ds.tags.iter().any(|t| t.to_lowercase().contains("multimodal"));

        match &plan.plan_type {
            PlanType::Pretraining | PlanType::ContinuedPretraining => {
                if has_text { score += 30.0; }
                if has_multimodal { score += 20.0; }
            }
            PlanType::SFT | PlanType::InstructionTuning => {
                if has_text { score += 40.0; }
            }
            PlanType::RLHF | PlanType::DPO => {
                if has_text { score += 40.0; }
            }
            _ => {
                if has_text { score += 20.0; }
                if has_image { score += 20.0; }
            }
        }

        if ds.columns >= 3 {
            score += 10.0;
        }

        (score as f64).min(100.0)
    }
}

impl PlanType {
    fn plan_type_name(&self) -> &str {
        match self {
            PlanType::Pretraining => "预训练",
            PlanType::FineTuning => "微调",
            PlanType::SFT => "监督微调",
            PlanType::RLHF => "RLHF",
            PlanType::DPO => "DPO",
            PlanType::ContinuedPretraining => "持续预训练",
            PlanType::InstructionTuning => "指令微调",
            PlanType::Custom(_) => "自定义",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dataset::aggregate::DatasetSummary;

    fn make_dataset(id: &str, name: &str, rows: usize, tags: Vec<&str>) -> DatasetSummary {
        DatasetSummary {
            id: crate::domain::dataset::aggregate::DatasetId::from_str(id),
            name: name.to_string(),
            version: "v1".to_string(),
            status: crate::domain::dataset::aggregate::DatasetStatus::Active,
            format: crate::types::DataFormat::Csv,
            rows,
            columns: 5,
            has_missing_values: false,
            memory_size_mb: 100.0,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            experiment_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_recommend_pretraining() {
        let plan = crate::data::training_plan::create_standard_llm_pretraining_plan();
        let datasets = vec![
            make_dataset("1", "web_corpus_pretrain", 10_000_000, vec!["pretraining", "web", "corpus"]),
            make_dataset("2", "instruction_sft_data", 10_000, vec!["instruction", "sft"]),
            make_dataset("3", "small_test", 100, vec!["test"]),
        ];
        let mut quality = std::collections::HashMap::new();
        quality.insert("1".to_string(), 85.0);
        quality.insert("2".to_string(), 90.0);
        quality.insert("3".to_string(), 60.0);

        let result = DatasetRecommender::recommend(&plan, &datasets, &quality);
        assert_eq!(result.recommendations.len(), 3);
        assert_eq!(result.recommendations[0].dataset_id, "1");
    }

    #[test]
    fn test_recommend_sft() {
        let plan = crate::data::training_plan::create_sft_training_plan();
        let datasets = vec![
            make_dataset("1", "web_corpus_pretrain", 10_000_000, vec!["pretraining"]),
            make_dataset("2", "instruction_sft_data", 10_000, vec!["instruction", "sft", "chat"]),
        ];
        let mut quality = std::collections::HashMap::new();
        quality.insert("1".to_string(), 85.0);
        quality.insert("2".to_string(), 90.0);

        let result = DatasetRecommender::recommend(&plan, &datasets, &quality);
        assert_eq!(result.recommendations[0].dataset_id, "2");
    }
}
