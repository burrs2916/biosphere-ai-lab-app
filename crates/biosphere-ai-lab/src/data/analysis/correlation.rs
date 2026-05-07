use serde::{Deserialize, Serialize};

use crate::domain::dataset::aggregate::ColumnProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureCorrelationReport {
    pub dataset_id: String,
    pub correlation_matrix: Vec<CorrelationEntry>,
    pub high_correlations: Vec<CorrelationPair>,
    pub collinear_groups: Vec<CollinearGroup>,
    pub target_correlations: Vec<TargetCorrelation>,
    pub feature_importance_hint: Vec<FeatureImportanceHint>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationEntry {
    pub column_a: String,
    pub column_b: String,
    pub correlation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationPair {
    pub column_a: String,
    pub column_b: String,
    pub correlation: f64,
    pub severity: CollinearitySeverity,
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollinearitySeverity {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for CollinearitySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollinearGroup {
    pub group_id: usize,
    pub columns: Vec<String>,
    pub avg_correlation: f64,
    pub suggested_retention: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetCorrelation {
    pub feature_name: String,
    pub correlation_with_target: f64,
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureImportanceHint {
    pub feature_name: String,
    pub score: f64,
    pub reason: String,
}

pub struct FeatureCorrelationAnalyzer;

impl FeatureCorrelationAnalyzer {
    pub fn analyze(
        dataset_id: &str,
        profiles: &[ColumnProfile],
        target_column: Option<&str>,
    ) -> FeatureCorrelationReport {
        let numeric_profiles: Vec<&ColumnProfile> = profiles.iter()
            .filter(|p| p.is_numeric())
            .collect();

        let correlation_matrix = Self::compute_correlations(&numeric_profiles);
        let high_correlations = Self::find_high_correlations(&correlation_matrix);
        let collinear_groups = Self::detect_collinear_groups(&high_correlations, &numeric_profiles);
        let target_correlations = Self::compute_target_correlations(&correlation_matrix, target_column);
        let feature_importance_hint = Self::compute_importance_hints(&target_correlations, &high_correlations, &numeric_profiles);
        let recommendations = Self::generate_recommendations(&high_correlations, &collinear_groups, &target_correlations, target_column);

        FeatureCorrelationReport {
            dataset_id: dataset_id.to_string(),
            correlation_matrix,
            high_correlations,
            collinear_groups,
            target_correlations,
            feature_importance_hint,
            recommendations,
        }
    }

    fn compute_correlations(profiles: &[&ColumnProfile]) -> Vec<CorrelationEntry> {
        let mut entries = Vec::new();

        for i in 0..profiles.len() {
            for j in (i + 1)..profiles.len() {
                let a = profiles[i];
                let b = profiles[j];

                let corr = Self::estimate_correlation(a, b);
                entries.push(CorrelationEntry {
                    column_a: a.name.clone(),
                    column_b: b.name.clone(),
                    correlation: corr,
                });
            }
        }

        entries
    }

    fn estimate_correlation(a: &ColumnProfile, b: &ColumnProfile) -> f64 {
        let mean_a = a.mean_value.unwrap_or(0.0);
        let mean_b = b.mean_value.unwrap_or(0.0);
        let std_a = a.std_value.unwrap_or(1.0).max(1e-10);
        let std_b = b.std_value.unwrap_or(1.0).max(1e-10);

        let range_a = match (&a.min_value, &a.max_value) {
            (Some(min), Some(max)) => {
                let lo = min.parse::<f64>().unwrap_or(0.0);
                let hi = max.parse::<f64>().unwrap_or(1.0);
                hi - lo
            }
            _ => std_a * 4.0,
        };
        let range_b = match (&b.min_value, &b.max_value) {
            (Some(min), Some(max)) => {
                let lo = min.parse::<f64>().unwrap_or(0.0);
                let hi = max.parse::<f64>().unwrap_or(1.0);
                hi - lo
            }
            _ => std_b * 4.0,
        };

        let mean_ratio = if range_a > 1e-10 && range_b > 1e-10 {
            let norm_a = (mean_a - a.min_value.as_ref().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0)) / range_a;
            let norm_b = (mean_b - b.min_value.as_ref().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0)) / range_b;
            1.0 - (norm_a - norm_b).abs().min(1.0)
        } else {
            0.0
        };

        let cv_a = std_a / mean_a.abs().max(1e-10);
        let cv_b = std_b / mean_b.abs().max(1e-10);
        let cv_similarity = 1.0 - (cv_a - cv_b).abs() / (cv_a + cv_b + 1e-10);

        let distinct_ratio_a = a.distinct_count as f64 / a.total_count.max(1) as f64;
        let distinct_ratio_b = b.distinct_count as f64 / b.total_count.max(1) as f64;
        let cardinality_similarity = 1.0 - (distinct_ratio_a - distinct_ratio_b).abs();

        let estimated = 0.3 * mean_ratio + 0.3 * cv_similarity + 0.2 * cardinality_similarity - 0.1;

        estimated.max(-1.0).min(1.0)
    }

    fn find_high_correlations(matrix: &[CorrelationEntry]) -> Vec<CorrelationPair> {
        matrix.iter()
            .filter(|e| e.correlation.abs() > 0.7)
            .map(|e| {
                let severity = if e.correlation.abs() > 0.95 {
                    CollinearitySeverity::High
                } else if e.correlation.abs() > 0.85 {
                    CollinearitySeverity::Medium
                } else {
                    CollinearitySeverity::Low
                };

                let recommendation = match severity {
                    CollinearitySeverity::High => format!(
                        "{} 和 {} 高度共线 (r={:.3})，强烈建议移除其中一个",
                        e.column_a, e.column_b, e.correlation
                    ),
                    CollinearitySeverity::Medium => format!(
                        "{} 和 {} 中度共线 (r={:.3})，建议考虑移除或使用正则化",
                        e.column_a, e.column_b, e.correlation
                    ),
                    CollinearitySeverity::Low => format!(
                        "{} 和 {} 存在一定相关性 (r={:.3})，可关注但暂无需处理",
                        e.column_a, e.column_b, e.correlation
                    ),
                };

                CorrelationPair {
                    column_a: e.column_a.clone(),
                    column_b: e.column_b.clone(),
                    correlation: e.correlation,
                    severity,
                    recommendation,
                }
            })
            .collect()
    }

    fn detect_collinear_groups(high_corrs: &[CorrelationPair], profiles: &[&ColumnProfile]) -> Vec<CollinearGroup> {
        use std::collections::{HashMap, HashSet};

        let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
        for pair in high_corrs {
            if pair.severity != CollinearitySeverity::Low {
                adjacency.entry(pair.column_a.clone()).or_default().insert(pair.column_b.clone());
                adjacency.entry(pair.column_b.clone()).or_default().insert(pair.column_a.clone());
            }
        }

        let mut visited: HashSet<String> = HashSet::new();
        let mut groups = Vec::new();
        let mut group_id = 0;

        for profile in profiles {
            let name = &profile.name;
            if visited.contains(name) {
                continue;
            }
            if !adjacency.contains_key(name) {
                continue;
            }

            let mut group_members = Vec::new();
            let mut stack = vec![name.clone()];
            while let Some(node) = stack.pop() {
                if visited.contains(&node) {
                    continue;
                }
                visited.insert(node.clone());
                group_members.push(node.clone());
                if let Some(neighbors) = adjacency.get(&node) {
                    for n in neighbors {
                        if !visited.contains(n) {
                            stack.push(n.clone());
                        }
                    }
                }
            }

            if group_members.len() >= 2 {
                let mut total_corr = 0.0;
                let mut count = 0;
                for pair in high_corrs {
                    if group_members.contains(&pair.column_a) && group_members.contains(&pair.column_b) {
                        total_corr += pair.correlation.abs();
                        count += 1;
                    }
                }
                let avg_corr = if count > 0 { total_corr / count as f64 } else { 0.0 };

                let suggested = group_members.first().cloned();

                groups.push(CollinearGroup {
                    group_id,
                    columns: group_members,
                    avg_correlation: avg_corr,
                    suggested_retention: suggested,
                });
                group_id += 1;
            }
        }

        groups
    }

    fn compute_target_correlations(matrix: &[CorrelationEntry], target_column: Option<&str>) -> Vec<TargetCorrelation> {
        let target = match target_column {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut corrs: Vec<TargetCorrelation> = matrix.iter()
            .filter(|e| e.column_a == target || e.column_b == target)
            .map(|e| {
                let feature = if e.column_a == target {
                    e.column_b.clone()
                } else {
                    e.column_a.clone()
                };
                TargetCorrelation {
                    feature_name: feature,
                    correlation_with_target: e.correlation,
                    rank: 0,
                }
            })
            .collect();

        corrs.sort_by(|a, b| b.correlation_with_target.abs().partial_cmp(&a.correlation_with_target.abs()).unwrap_or(std::cmp::Ordering::Equal));
        for (i, c) in corrs.iter_mut().enumerate() {
            c.rank = i + 1;
        }

        corrs
    }

    fn compute_importance_hints(
        target_corrs: &[TargetCorrelation],
        high_corrs: &[CorrelationPair],
        profiles: &[&ColumnProfile],
    ) -> Vec<FeatureImportanceHint> {
        let mut hints = Vec::new();

        let high_corr_features: std::collections::HashSet<&str> = high_corrs.iter()
            .filter(|p| p.severity == CollinearitySeverity::High)
            .flat_map(|p| [p.column_a.as_str(), p.column_b.as_str()])
            .collect();

        for tc in target_corrs {
            let is_collinear = high_corr_features.contains(tc.feature_name.as_str());
            let score = tc.correlation_with_target.abs() * if is_collinear { 0.7 } else { 1.0 };
            let reason = if is_collinear {
                format!("与目标相关 (r={:.3})，但存在共线性，实际贡献可能被稀释", tc.correlation_with_target)
            } else if tc.correlation_with_target.abs() > 0.5 {
                format!("与目标强相关 (r={:.3})，重要特征", tc.correlation_with_target)
            } else if tc.correlation_with_target.abs() > 0.2 {
                format!("与目标中等相关 (r={:.3})", tc.correlation_with_target)
            } else {
                format!("与目标弱相关 (r={:.3})，考虑移除", tc.correlation_with_target)
            };

            hints.push(FeatureImportanceHint {
                feature_name: tc.feature_name.clone(),
                score,
                reason,
            });
        }

        let target_names: std::collections::HashSet<&str> = target_corrs.iter().map(|t| t.feature_name.as_str()).collect();
        for profile in profiles {
            if !target_names.contains(profile.name.as_str()) {
                let null_rate = profile.null_count as f64 / profile.total_count as f64;
                let score = 0.1 * (1.0 - null_rate);
                hints.push(FeatureImportanceHint {
                    feature_name: profile.name.clone(),
                    score,
                    reason: format!("未计算与目标的相关性，null率 {:.1}%", null_rate * 100.0),
                });
            }
        }

        hints.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hints
    }

    fn generate_recommendations(
        high_corrs: &[CorrelationPair],
        collinear_groups: &[CollinearGroup],
        target_corrs: &[TargetCorrelation],
        target_column: Option<&str>,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        let high_count = high_corrs.iter().filter(|p| p.severity == CollinearitySeverity::High).count();
        if high_count > 0 {
            recs.push(format!("发现 {} 对高度共线特征，建议移除冗余特征或使用 L1/L2 正则化", high_count));
        }

        if !collinear_groups.is_empty() {
            recs.push(format!("检测到 {} 个共线特征组，每组保留 1 个特征即可", collinear_groups.len()));
            for group in collinear_groups {
                if let Some(ref retention) = group.suggested_retention {
                    recs.push(format!("  共线组 {}: {} → 建议保留 '{}'",
                        group.group_id, group.columns.join(", "), retention));
                }
            }
        }

        if let Some(target) = target_column {
            let weak_features: Vec<&str> = target_corrs.iter()
                .filter(|t| t.correlation_with_target.abs() < 0.1)
                .map(|t| t.feature_name.as_str())
                .collect();
            if !weak_features.is_empty() {
                recs.push(format!("与目标 '{}' 弱相关的特征: {}，考虑移除以减少噪声",
                    target, weak_features.join(", ")));
            }

            let strong_features: Vec<&str> = target_corrs.iter()
                .filter(|t| t.correlation_with_target.abs() > 0.5)
                .map(|t| t.feature_name.as_str())
                .collect();
            if !strong_features.is_empty() {
                recs.push(format!("与目标 '{}' 强相关的特征: {}，这些是模型的关键输入",
                    target, strong_features.join(", ")));
            }
        } else {
            recs.push("未指定目标列，无法计算特征重要性。建议在训练时指定 target_column".to_string());
        }

        recs
    }
}
