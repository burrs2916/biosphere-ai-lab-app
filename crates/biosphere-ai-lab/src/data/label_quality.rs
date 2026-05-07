use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelQualityReport {
    pub dataset_id: String,
    pub label_column: String,
    pub total_samples: usize,
    pub num_annotators: usize,
    pub num_labels: usize,
    pub agreement: AgreementMetrics,
    pub annotator_stats: Vec<AnnotatorStats>,
    pub label_distribution: Vec<LabelDistribution>,
    pub confidence_scores: Option<ConfidenceAnalysis>,
    pub quality_grade: LabelQualityGrade,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgreementMetrics {
    pub fleiss_kappa: Option<f64>,
    pub cohens_kappa_avg: Option<f64>,
    pub percent_agreement: f64,
    pub krippendorff_alpha: Option<f64>,
    pub agreement_level: AgreementLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgreementLevel {
    Poor,
    Fair,
    Moderate,
    Substantial,
    AlmostPerfect,
}

impl std::fmt::Display for AgreementLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poor => write!(f, "poor"),
            Self::Fair => write!(f, "fair"),
            Self::Moderate => write!(f, "moderate"),
            Self::Substantial => write!(f, "substantial"),
            Self::AlmostPerfect => write!(f, "almost_perfect"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatorStats {
    pub annotator_id: String,
    pub total_annotations: usize,
    pub agreement_with_consensus: f64,
    pub avg_confidence: Option<f64>,
    pub annotation_speed: Option<f64>,
    pub bias_tendency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelDistribution {
    pub label_value: String,
    pub count: usize,
    pub ratio: f64,
    pub agreement_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceAnalysis {
    pub avg_confidence: f64,
    pub low_confidence_count: usize,
    pub low_confidence_rate: f64,
    pub confidence_by_label: Vec<LabelConfidence>,
    pub low_confidence_samples: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelConfidence {
    pub label_value: String,
    pub avg_confidence: f64,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelQualityGrade {
    Excellent,
    Good,
    Acceptable,
    Poor,
    Unusable,
}

impl std::fmt::Display for LabelQualityGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Excellent => write!(f, "excellent"),
            Self::Good => write!(f, "good"),
            Self::Acceptable => write!(f, "acceptable"),
            Self::Poor => write!(f, "poor"),
            Self::Unusable => write!(f, "unusable"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationRecord {
    pub sample_id: usize,
    pub annotator_id: String,
    pub label: String,
    pub confidence: Option<f64>,
    pub timestamp: Option<String>,
    pub duration_seconds: Option<f64>,
}

pub struct LabelQualityAnalyzer;

impl LabelQualityAnalyzer {
    pub fn analyze(
        dataset_id: &str,
        label_column: &str,
        annotations: &[AnnotationRecord],
        num_annotators: usize,
    ) -> LabelQualityReport {
        let total_samples = annotations.iter()
            .map(|a| a.sample_id)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);

        let unique_labels: Vec<String> = {
            let mut labels: Vec<String> = annotations.iter()
                .map(|a| a.label.clone())
                .collect();
            labels.sort();
            labels.dedup();
            labels
        };
        let num_labels = unique_labels.len();

        let agreement = Self::compute_agreement(annotations, num_annotators, total_samples);
        let annotator_stats = Self::compute_annotator_stats(annotations, num_annotators);
        let label_distribution = Self::compute_label_distribution(annotations, &unique_labels);
        let confidence_scores = Self::compute_confidence(annotations);

        let quality_grade = Self::determine_grade(&agreement, &confidence_scores);
        let recommendations = Self::generate_recommendations(&agreement, &annotator_stats, &confidence_scores, quality_grade);

        LabelQualityReport {
            dataset_id: dataset_id.to_string(),
            label_column: label_column.to_string(),
            total_samples,
            num_annotators,
            num_labels,
            agreement,
            annotator_stats,
            label_distribution,
            confidence_scores,
            quality_grade,
            recommendations,
        }
    }

    fn compute_agreement(
        annotations: &[AnnotationRecord],
        num_annotators: usize,
        total_samples: usize,
    ) -> AgreementMetrics {
        let mut sample_labels: HashMap<usize, Vec<&str>> = HashMap::new();
        for a in annotations {
            sample_labels.entry(a.sample_id).or_default().push(&a.label);
        }

        let mut total_pairs = 0;
        let mut agreeing_pairs = 0;

        for labels in sample_labels.values() {
            if labels.len() < 2 {
                continue;
            }
            for i in 0..labels.len() {
                for j in (i + 1)..labels.len() {
                    total_pairs += 1;
                    if labels[i] == labels[j] {
                        agreeing_pairs += 1;
                    }
                }
            }
        }

        let percent_agreement = if total_pairs > 0 {
            agreeing_pairs as f64 / total_pairs as f64
        } else {
            1.0
        };

        let fleiss_kappa = if num_annotators >= 2 && total_samples > 0 {
            Some(Self::compute_fleiss_kappa(annotations, num_annotators, total_samples))
        } else {
            None
        };

        let cohens_kappa_avg = if num_annotators == 2 {
            Some(Self::compute_cohens_kappa(annotations))
        } else {
            None
        };

        let krippendorff_alpha = if num_annotators >= 2 {
            Some(Self::compute_krippendorff_alpha(annotations, num_annotators))
        } else {
            None
        };

        let agreement_level = Self::classify_agreement(fleiss_kappa.unwrap_or(percent_agreement));

        AgreementMetrics {
            fleiss_kappa,
            cohens_kappa_avg,
            percent_agreement,
            krippendorff_alpha,
            agreement_level,
        }
    }

    fn compute_fleiss_kappa(
        annotations: &[AnnotationRecord],
        num_annotators: usize,
        total_samples: usize,
    ) -> f64 {
        let unique_labels: Vec<&str> = {
            let mut labels: Vec<&str> = annotations.iter().map(|a| a.label.as_str()).collect();
            labels.sort();
            labels.dedup();
            labels
        };

        let mut n_ij: HashMap<(usize, &str), usize> = HashMap::new();
        for a in annotations {
            *n_ij.entry((a.sample_id, a.label.as_str())).or_insert(0) += 1;
        }

        let n = num_annotators as f64;
        let _k = unique_labels.len() as f64;
        let big_n = total_samples as f64;

        let mut p_j_sum = 0.0;
        for label in &unique_labels {
            let total: usize = annotations.iter()
                .filter(|a| a.label == *label)
                .count();
            let p_j = total as f64 / (big_n * n);
            p_j_sum += p_j * p_j;
        }

        let mut p_i_sum = 0.0;
        for i in 0..total_samples {
            let mut sum_sq = 0.0;
            for label in &unique_labels {
                let n_ij_val = n_ij.get(&(i, label)).copied().unwrap_or(0) as f64;
                sum_sq += n_ij_val * n_ij_val;
            }
            let p_i = (sum_sq - n) / (n * (n - 1.0));
            p_i_sum += p_i;
        }

        let p_bar = p_i_sum / big_n;
        let p_e_bar = p_j_sum;

        if (1.0 - p_e_bar).abs() < 1e-10 {
            return 1.0;
        }

        (p_bar - p_e_bar) / (1.0 - p_e_bar)
    }

    fn compute_cohens_kappa(annotations: &[AnnotationRecord]) -> f64 {
        let mut pairs: HashMap<usize, (Option<&str>, Option<&str>)> = HashMap::new();
        for a in annotations {
            let entry = pairs.entry(a.sample_id).or_insert((None, None));
            if entry.0.is_none() {
                entry.0 = Some(&a.label);
            } else {
                entry.1 = Some(&a.label);
            }
        }

        let mut n = 0;
        let mut n_agree = 0;
        let mut counts_a: HashMap<&str, usize> = HashMap::new();
        let mut counts_b: HashMap<&str, usize> = HashMap::new();

        for (a_label, b_label) in pairs.values() {
            if let (Some(a), Some(b)) = (a_label, b_label) {
                n += 1;
                if a == b {
                    n_agree += 1;
                }
                *counts_a.entry(a).or_insert(0) += 1;
                *counts_b.entry(b).or_insert(0) += 1;
            }
        }

        if n == 0 {
            return 1.0;
        }

        let p_o = n_agree as f64 / n as f64;

        let mut p_e = 0.0;
        let all_labels: HashSet<&&str> = counts_a.keys().chain(counts_b.keys()).collect();
        for label in all_labels {
            let p_a = *counts_a.get(label).unwrap_or(&0) as f64 / n as f64;
            let p_b = *counts_b.get(label).unwrap_or(&0) as f64 / n as f64;
            p_e += p_a * p_b;
        }

        if (1.0 - p_e).abs() < 1e-10 {
            return 1.0;
        }

        (p_o - p_e) / (1.0 - p_e)
    }

    fn compute_krippendorff_alpha(
        annotations: &[AnnotationRecord],
        _num_annotators: usize,
    ) -> f64 {
        let mut sample_labels: HashMap<usize, Vec<&str>> = HashMap::new();
        for a in annotations {
            sample_labels.entry(a.sample_id).or_default().push(&a.label);
        }

        let mut total_d_o = 0.0;
        let mut total_pairs = 0;

        let samples: Vec<&Vec<&str>> = sample_labels.values().collect();

        for labels in &samples {
            if labels.len() < 2 {
                continue;
            }
            for i in 0..labels.len() {
                for j in (i + 1)..labels.len() {
                    total_pairs += 1;
                    if labels[i] != labels[j] {
                        total_d_o += 1.0;
                    }
                }
            }
        }

        let d_o = if total_pairs > 0 {
            total_d_o / total_pairs as f64
        } else {
            0.0
        };

        let mut all_labels_count: HashMap<&str, usize> = HashMap::new();
        let mut total_annotations = 0;
        for labels in &samples {
            for label in labels.iter() {
                *all_labels_count.entry(label).or_insert(0) += 1;
                total_annotations += 1;
            }
        }

        let mut d_e = 0.0;
        let labels: Vec<&&str> = all_labels_count.keys().collect();
        for &l1 in &labels {
            for &l2 in &labels {
                if l1 != l2 {
                    let p1 = *all_labels_count.get(l1).unwrap_or(&0) as f64 / total_annotations as f64;
                    let p2 = *all_labels_count.get(l2).unwrap_or(&0) as f64 / total_annotations as f64;
                    d_e += p1 * p2;
                }
            }
        }

        if d_e < 1e-10 {
            return 1.0;
        }

        1.0 - (d_o / d_e)
    }

    fn classify_agreement(kappa: f64) -> AgreementLevel {
        if kappa < 0.0 {
            AgreementLevel::Poor
        } else if kappa < 0.2 {
            AgreementLevel::Poor
        } else if kappa < 0.4 {
            AgreementLevel::Fair
        } else if kappa < 0.6 {
            AgreementLevel::Moderate
        } else if kappa < 0.8 {
            AgreementLevel::Substantial
        } else {
            AgreementLevel::AlmostPerfect
        }
    }

    fn compute_annotator_stats(
        annotations: &[AnnotationRecord],
        _num_annotators: usize,
    ) -> Vec<AnnotatorStats> {
        let mut annotator_groups: HashMap<&str, Vec<&AnnotationRecord>> = HashMap::new();
        for a in annotations {
            annotator_groups.entry(&a.annotator_id).or_default().push(a);
        }

        let mut consensus: HashMap<usize, &str> = HashMap::new();
        let mut sample_votes: HashMap<usize, HashMap<&str, usize>> = HashMap::new();
        for a in annotations {
            let votes = sample_votes.entry(a.sample_id).or_default();
            *votes.entry(&a.label).or_insert(0) += 1;
        }
        for (sample_id, votes) in &sample_votes {
            if let Some((label, _)) = votes.iter().max_by_key(|(_, &c)| c) {
                consensus.insert(*sample_id, label);
            }
        }

        let mut stats = Vec::new();
        for (annotator_id, records) in &annotator_groups {
            let total = records.len();
            let mut agree_count = 0;
            let mut conf_sum = 0.0;
            let mut conf_count = 0;

            for r in records.iter() {
                if let Some(consensus_label) = consensus.get(&r.sample_id) {
                    if r.label == **consensus_label {
                        agree_count += 1;
                    }
                }
                if let Some(c) = r.confidence {
                    conf_sum += c;
                    conf_count += 1;
                }
            }

            let agreement = if total > 0 {
                agree_count as f64 / total as f64
            } else {
                0.0
            };

            let avg_confidence = if conf_count > 0 {
                Some(conf_sum / conf_count as f64)
            } else {
                None
            };

            stats.push(AnnotatorStats {
                annotator_id: annotator_id.to_string(),
                total_annotations: total,
                agreement_with_consensus: agreement,
                avg_confidence,
                annotation_speed: None,
                bias_tendency: None,
            });
        }

        stats
    }

    fn compute_label_distribution(
        annotations: &[AnnotationRecord],
        unique_labels: &[String],
    ) -> Vec<LabelDistribution> {
        let mut label_counts: HashMap<&str, usize> = HashMap::new();
        let mut label_agreements: HashMap<&str, (usize, usize)> = HashMap::new();

        let mut sample_votes: HashMap<usize, HashMap<&str, usize>> = HashMap::new();
        for a in annotations {
            *label_counts.entry(&a.label).or_insert(0) += 1;
            let votes = sample_votes.entry(a.sample_id).or_default();
            *votes.entry(&a.label).or_insert(0) += 1;
        }

        let total: usize = label_counts.values().sum();

        let mut consensus: HashMap<usize, &str> = HashMap::new();
        for (sample_id, votes) in &sample_votes {
            if let Some((label, _)) = votes.iter().max_by_key(|(_, &c)| c) {
                consensus.insert(*sample_id, label);
            }
        }

        for a in annotations {
            let entry = label_agreements.entry(&a.label).or_insert((0, 0));
            entry.1 += 1;
            if let Some(consensus_label) = consensus.get(&a.sample_id) {
                if a.label == **consensus_label {
                    entry.0 += 1;
                }
            }
        }

        unique_labels.iter()
            .map(|label| {
                let count = *label_counts.get(label.as_str()).unwrap_or(&0);
                let ratio = if total > 0 { count as f64 / total as f64 } else { 0.0 };
                let (agree, total_for_label) = label_agreements.get(label.as_str()).copied().unwrap_or((0, 0));
                let agreement_rate = if total_for_label > 0 {
                    agree as f64 / total_for_label as f64
                } else {
                    0.0
                };

                LabelDistribution {
                    label_value: label.clone(),
                    count,
                    ratio,
                    agreement_rate,
                }
            })
            .collect()
    }

    fn compute_confidence(annotations: &[AnnotationRecord]) -> Option<ConfidenceAnalysis> {
        let confidences: Vec<f64> = annotations.iter()
            .filter_map(|a| a.confidence)
            .collect();

        if confidences.is_empty() {
            return None;
        }

        let avg_confidence = confidences.iter().sum::<f64>() / confidences.len() as f64;

        let low_confidence_samples: Vec<usize> = annotations.iter()
            .filter(|a| a.confidence.map(|c| c < 0.5).unwrap_or(false))
            .map(|a| a.sample_id)
            .collect();

        let low_confidence_count = low_confidence_samples.len();
        let low_confidence_rate = low_confidence_count as f64 / annotations.len() as f64;

        let mut label_confs: HashMap<&str, (f64, usize)> = HashMap::new();
        for a in annotations {
            if let Some(c) = a.confidence {
                let entry = label_confs.entry(&a.label).or_insert((0.0, 0));
                entry.0 += c;
                entry.1 += 1;
            }
        }

        let confidence_by_label: Vec<LabelConfidence> = label_confs.iter()
            .map(|(label, (sum, count))| LabelConfidence {
                label_value: label.to_string(),
                avg_confidence: sum / *count as f64,
                count: *count,
            })
            .collect();

        Some(ConfidenceAnalysis {
            avg_confidence,
            low_confidence_count,
            low_confidence_rate,
            confidence_by_label,
            low_confidence_samples,
        })
    }

    fn determine_grade(
        agreement: &AgreementMetrics,
        confidence: &Option<ConfidenceAnalysis>,
    ) -> LabelQualityGrade {
        let kappa = agreement.fleiss_kappa.unwrap_or(agreement.percent_agreement);

        let conf_ok = match confidence {
            Some(c) => c.avg_confidence >= 0.7 && c.low_confidence_rate < 0.1,
            None => true,
        };

        match (kappa, conf_ok) {
            (k, _) if k >= 0.8 => LabelQualityGrade::Excellent,
            (k, true) if k >= 0.6 => LabelQualityGrade::Good,
            (k, _) if k >= 0.4 => LabelQualityGrade::Acceptable,
            (k, _) if k >= 0.2 => LabelQualityGrade::Poor,
            _ => LabelQualityGrade::Unusable,
        }
    }

    fn generate_recommendations(
        agreement: &AgreementMetrics,
        annotator_stats: &[AnnotatorStats],
        confidence: &Option<ConfidenceAnalysis>,
        grade: LabelQualityGrade,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        match grade {
            LabelQualityGrade::Excellent => {
                recs.push("✅ 标注质量优秀，可直接用于训练".to_string());
            }
            LabelQualityGrade::Good => {
                recs.push("👍 标注质量良好，建议对低置信度样本进行复核".to_string());
            }
            LabelQualityGrade::Acceptable => {
                recs.push("⚠️ 标注质量一般，建议：".to_string());
                recs.push("  1. 对不一致样本进行第三方仲裁".to_string());
                recs.push("  2. 增加标注指南的详细程度".to_string());
            }
            LabelQualityGrade::Poor | LabelQualityGrade::Unusable => {
                recs.push("🔴 标注质量差，不建议直接用于训练：".to_string());
                recs.push("  1. 重新培训标注人员".to_string());
                recs.push("  2. 简化标注任务或增加示例".to_string());
                recs.push("  3. 考虑使用弱监督方法".to_string());
            }
        }

        if agreement.percent_agreement < 0.7 {
            recs.push(format!(
                "标注者一致性仅 {:.1}%，远低于 70% 的最低标准",
                agreement.percent_agreement * 100.0
            ));
        }

        let low_agreement_annotators: Vec<&str> = annotator_stats.iter()
            .filter(|s| s.agreement_with_consensus < 0.6)
            .map(|s| s.annotator_id.as_str())
            .collect();

        if !low_agreement_annotators.is_empty() {
            recs.push(format!(
                "标注者 {} 与共识一致性低，建议单独培训",
                low_agreement_annotators.join(", ")
            ));
        }

        if let Some(c) = confidence {
            if c.low_confidence_rate > 0.2 {
                recs.push(format!(
                    "{:.1}% 的标注置信度低，建议重点复核这些样本",
                    c.low_confidence_rate * 100.0
                ));
            }
        }

        recs
    }
}
