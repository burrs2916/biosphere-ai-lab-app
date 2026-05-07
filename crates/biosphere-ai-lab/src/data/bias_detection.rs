use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasDetectionReport {
    pub dataset_id: String,
    pub protected_attributes: Vec<String>,
    pub total_samples: usize,
    pub bias_metrics: Vec<AttributeBiasMetrics>,
    pub intersectional_bias: Option<IntersectionalBias>,
    pub overall_bias_level: BiasLevel,
    pub fairness_recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeBiasMetrics {
    pub attribute_name: String,
    pub groups: Vec<GroupMetrics>,
    pub statistical_parity_difference: f64,
    pub disparate_impact_ratio: f64,
    pub equal_opportunity_difference: Option<f64>,
    pub demographic_parity: DemographicParity,
    pub bias_detected: bool,
    pub bias_direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMetrics {
    pub group_value: String,
    pub sample_count: usize,
    pub sample_ratio: f64,
    pub positive_rate: Option<f64>,
    pub avg_prediction: Option<f64>,
    pub representation_gap: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemographicParity {
    pub max_positive_rate: f64,
    pub min_positive_rate: f64,
    pub difference: f64,
    pub ratio: f64,
    pub is_fair: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntersectionalBias {
    pub attribute_pairs: Vec<IntersectionalPair>,
    pub worst_case_disparity: f64,
    pub worst_case_groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntersectionalPair {
    pub attr_a: String,
    pub attr_b: String,
    pub combinations: Vec<IntersectionalGroup>,
    pub max_disparity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntersectionalGroup {
    pub group_a_value: String,
    pub group_b_value: String,
    pub count: usize,
    pub positive_rate: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiasLevel {
    None,
    Low,
    Medium,
    High,
    Severe,
}

impl std::fmt::Display for BiasLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Severe => write!(f, "severe"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasDetectionConfig {
    pub protected_attributes: Vec<String>,
    pub label_column: String,
    pub positive_label: String,
    pub fairness_threshold: f64,
    pub check_intersectional: bool,
}

impl Default for BiasDetectionConfig {
    fn default() -> Self {
        Self {
            protected_attributes: Vec::new(),
            label_column: "label".to_string(),
            positive_label: "1".to_string(),
            fairness_threshold: 0.8,
            check_intersectional: true,
        }
    }
}

pub struct BiasDetector;

impl BiasDetector {
    pub fn analyze(
        dataset_id: &str,
        rows: &[Vec<String>],
        column_names: &[String],
        config: &BiasDetectionConfig,
    ) -> BiasDetectionReport {
        let total_samples = rows.len();

        let label_col_idx = column_names.iter().position(|c| c == &config.label_column);

        let mut bias_metrics = Vec::new();

        for attr in &config.protected_attributes {
            let attr_col_idx = column_names.iter().position(|c| c == attr);

            if let (Some(aci), Some(lci)) = (attr_col_idx, label_col_idx) {
                let metrics = Self::compute_attribute_bias(
                    rows, aci, lci, attr, &config.positive_label, total_samples,
                );
                bias_metrics.push(metrics);
            }
        }

        let intersectional_bias = if config.check_intersectional && config.protected_attributes.len() >= 2 {
            Some(Self::compute_intersectional_bias(
                rows, column_names, &config.protected_attributes, label_col_idx, &config.positive_label,
            ))
        } else {
            None
        };

        let overall_bias_level = Self::determine_overall_level(&bias_metrics, &intersectional_bias);
        let fairness_recommendations = Self::generate_fairness_recommendations(
            &bias_metrics, &intersectional_bias, overall_bias_level,
        );

        BiasDetectionReport {
            dataset_id: dataset_id.to_string(),
            protected_attributes: config.protected_attributes.clone(),
            total_samples,
            bias_metrics,
            intersectional_bias,
            overall_bias_level,
            fairness_recommendations,
        }
    }

    fn compute_attribute_bias(
        rows: &[Vec<String>],
        attr_col_idx: usize,
        label_col_idx: usize,
        attr_name: &str,
        positive_label: &str,
        total_samples: usize,
    ) -> AttributeBiasMetrics {
        let mut group_data: HashMap<&str, (usize, usize)> = HashMap::new();

        for row in rows {
            if let (Some(attr_val), Some(label_val)) = (
                row.get(attr_col_idx),
                row.get(label_col_idx),
            ) {
                if attr_val.is_empty() {
                    continue;
                }
                let entry = group_data.entry(attr_val.as_str()).or_insert((0, 0));
                entry.0 += 1;
                if label_val == positive_label {
                    entry.1 += 1;
                }
            }
        }

        let mut groups = Vec::new();
        let mut positive_rates = Vec::new();

        for (group_val, (count, pos_count)) in &group_data {
            let sample_ratio = *count as f64 / total_samples as f64;
            let positive_rate = if *count > 0 {
                *pos_count as f64 / *count as f64
            } else {
                0.0
            };
            let expected_ratio = 1.0 / group_data.len() as f64;
            let representation_gap = sample_ratio - expected_ratio;

            groups.push(GroupMetrics {
                group_value: group_val.to_string(),
                sample_count: *count,
                sample_ratio,
                positive_rate: Some(positive_rate),
                avg_prediction: None,
                representation_gap,
            });

            positive_rates.push(positive_rate);
        }

        let max_pr = positive_rates.iter().cloned().fold(0.0, f64::max);
        let min_pr = positive_rates.iter().cloned().fold(1.0, f64::min);
        let spd = max_pr - min_pr;
        let dir = if min_pr > 0.0 {
            max_pr / min_pr
        } else {
            f64::MAX
        };

        let demographic_parity = DemographicParity {
            max_positive_rate: max_pr,
            min_positive_rate: min_pr,
            difference: spd,
            ratio: dir,
            is_fair: spd < 0.1 && dir >= 0.8 && dir <= 1.25,
        };

        let bias_detected = spd > 0.1 || dir < 0.8 || dir > 1.25;

        let bias_direction = if bias_detected {
            let max_group = groups.iter().max_by(|a, b| {
                a.positive_rate.unwrap_or(0.0).partial_cmp(&b.positive_rate.unwrap_or(0.0)).unwrap()
            });
            let min_group = groups.iter().min_by(|a, b| {
                a.positive_rate.unwrap_or(0.0).partial_cmp(&b.positive_rate.unwrap_or(0.0)).unwrap()
            });
            match (max_group, min_group) {
                (Some(max_g), Some(min_g)) => Some(format!(
                    "{} 的正样本率 ({:.1}%) 显著高于 {} ({:.1}%)",
                    max_g.group_value,
                    max_g.positive_rate.unwrap_or(0.0) * 100.0,
                    min_g.group_value,
                    min_g.positive_rate.unwrap_or(0.0) * 100.0,
                )),
                _ => None,
            }
        } else {
            None
        };

        AttributeBiasMetrics {
            attribute_name: attr_name.to_string(),
            groups,
            statistical_parity_difference: spd,
            disparate_impact_ratio: dir,
            equal_opportunity_difference: None,
            demographic_parity,
            bias_detected,
            bias_direction,
        }
    }

    fn compute_intersectional_bias(
        rows: &[Vec<String>],
        column_names: &[String],
        protected_attrs: &[String],
        label_col_idx: Option<usize>,
        positive_label: &str,
    ) -> IntersectionalBias {
        let mut attribute_pairs = Vec::new();
        let mut all_disparities: Vec<(f64, Vec<String>)> = Vec::new();

        for i in 0..protected_attrs.len() {
            for j in (i + 1)..protected_attrs.len() {
                let attr_a = &protected_attrs[i];
                let attr_b = &protected_attrs[j];

                let col_a = column_names.iter().position(|c| c == attr_a);
                let col_b = column_names.iter().position(|c| c == attr_b);

                if let (Some(ca), Some(cb)) = (col_a, col_b) {
                    let mut combo_data: HashMap<(&str, &str), (usize, usize)> = HashMap::new();

                    for row in rows {
                        if let (Some(va), Some(vb)) = (row.get(ca), row.get(cb)) {
                            if va.is_empty() || vb.is_empty() {
                                continue;
                            }
                            let is_pos = label_col_idx
                                .and_then(|lci| row.get(lci))
                                .map(|l| l == positive_label)
                                .unwrap_or(false);

                            let entry = combo_data.entry((va.as_str(), vb.as_str())).or_insert((0, 0));
                            entry.0 += 1;
                            if is_pos {
                                entry.1 += 1;
                            }
                        }
                    }

                    let mut combinations = Vec::new();
                    let mut prs = Vec::new();

                    for ((va, vb), (count, pos_count)) in &combo_data {
                        let pr = if *count > 0 {
                            *pos_count as f64 / *count as f64
                        } else {
                            0.0
                        };
                        combinations.push(IntersectionalGroup {
                            group_a_value: va.to_string(),
                            group_b_value: vb.to_string(),
                            count: *count,
                            positive_rate: Some(pr),
                        });
                        prs.push(pr);
                    }

                    let max_pr = prs.iter().cloned().fold(0.0, f64::max);
                    let min_pr = prs.iter().cloned().fold(1.0, f64::min);
                    let max_disparity = max_pr - min_pr;

                    let worst_groups: Vec<String> = combinations.iter()
                        .filter(|c| {
                            let pr = c.positive_rate.unwrap_or(0.0);
                            (pr - max_pr).abs() < 1e-6 || (pr - min_pr).abs() < 1e-6
                        })
                        .map(|c| format!("{}+{}", c.group_a_value, c.group_b_value))
                        .collect();

                    all_disparities.push((max_disparity, worst_groups));

                    attribute_pairs.push(IntersectionalPair {
                        attr_a: attr_a.clone(),
                        attr_b: attr_b.clone(),
                        combinations,
                        max_disparity,
                    });
                }
            }
        }

        let worst_case_disparity = all_disparities.iter()
            .map(|(d, _)| *d)
            .fold(0.0, f64::max);

        let worst_case_groups: Vec<String> = all_disparities.iter()
            .filter(|(d, _)| (*d - worst_case_disparity).abs() < 1e-6)
            .flat_map(|(_, groups)| groups.clone())
            .collect();

        IntersectionalBias {
            attribute_pairs,
            worst_case_disparity,
            worst_case_groups,
        }
    }

    fn determine_overall_level(
        metrics: &[AttributeBiasMetrics],
        intersectional: &Option<IntersectionalBias>,
    ) -> BiasLevel {
        let max_spd = metrics.iter()
            .map(|m| m.statistical_parity_difference)
            .fold(0.0, f64::max);

        let min_dir = metrics.iter()
            .map(|m| m.disparate_impact_ratio)
            .fold(f64::MAX, f64::min);

        let intersectional_factor = match intersectional {
            Some(ib) if ib.worst_case_disparity > 0.3 => 2,
            Some(ib) if ib.worst_case_disparity > 0.15 => 1,
            _ => 0,
        };

        match (max_spd, min_dir, intersectional_factor) {
            (spd, _, _) if spd > 0.3 => BiasLevel::Severe,
            (spd, _, 2) if spd > 0.15 => BiasLevel::Severe,
            (spd, _, _) if spd > 0.2 => BiasLevel::High,
            (_, dir, _) if dir < 0.5 => BiasLevel::High,
            (spd, _, _) if spd > 0.1 => BiasLevel::Medium,
            (_, dir, _) if dir < 0.8 => BiasLevel::Medium,
            (spd, _, _) if spd > 0.05 => BiasLevel::Low,
            _ => BiasLevel::None,
        }
    }

    fn generate_fairness_recommendations(
        metrics: &[AttributeBiasMetrics],
        intersectional: &Option<IntersectionalBias>,
        level: BiasLevel,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        match level {
            BiasLevel::None => {
                recs.push("✅ 未检测到显著偏差，数据集在受保护属性上表现公平".to_string());
            }
            BiasLevel::Low => {
                recs.push("🟢 检测到轻微偏差，建议在训练时使用公平性约束".to_string());
            }
            BiasLevel::Medium => {
                recs.push("🟡 中等偏差检测到，建议：".to_string());
                recs.push("  1. 使用重采样平衡各组样本量".to_string());
                recs.push("  2. 训练时添加公平性正则化项".to_string());
            }
            BiasLevel::High | BiasLevel::Severe => {
                recs.push("🔴 严重偏差！必须处理：".to_string());
                recs.push("  1. 检查数据采集过程是否存在系统性偏差".to_string());
                recs.push("  2. 对欠代表群体进行过采样或数据增强".to_string());
                recs.push("  3. 考虑使用对抗去偏方法".to_string());
                recs.push("  4. 部署前必须在各子群上独立评估".to_string());
            }
        }

        for m in metrics {
            if m.bias_detected {
                if let Some(ref direction) = m.bias_direction {
                    recs.push(format!("  [{}] {}", m.attribute_name, direction));
                }
            }
        }

        if let Some(ib) = intersectional {
            if ib.worst_case_disparity > 0.15 {
                recs.push(format!(
                    "⚠️ 交叉偏差严重（{:.1}%），最差组合: {}",
                    ib.worst_case_disparity * 100.0,
                    ib.worst_case_groups.join(", ")
                ));
            }
        }

        recs
    }
}
