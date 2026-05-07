use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceAnalysisReport {
    pub dataset_id: String,
    pub total_rows: usize,
    pub slices: Vec<DataSlice>,
    pub cross_slice_comparison: Option<CrossSliceComparison>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSlice {
    pub slice_name: String,
    pub slice_condition: String,
    pub row_count: usize,
    pub row_ratio: f64,
    pub column_stats: Vec<SliceColumnStat>,
    pub label_distribution: Option<Vec<SliceLabelDist>>,
    pub quality_metrics: SliceQualityMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceColumnStat {
    pub column_name: String,
    pub null_count: usize,
    pub null_rate: f64,
    pub distinct_count: usize,
    pub mean_value: Option<f64>,
    pub std_value: Option<f64>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceLabelDist {
    pub label_value: String,
    pub count: usize,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceQualityMetrics {
    pub completeness_score: f64,
    pub balance_score: f64,
    pub representativeness_score: f64,
    pub overall_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSliceComparison {
    pub slice_pairs: Vec<SlicePairComparison>,
    pub most_divergent_slices: Vec<String>,
    pub distribution_shift_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicePairComparison {
    pub slice_a: String,
    pub slice_b: String,
    pub row_count_ratio: f64,
    pub column_diffs: Vec<ColumnDiff>,
    pub label_distribution_shift: Option<f64>,
    pub overall_similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDiff {
    pub column_name: String,
    pub mean_diff: Option<f64>,
    pub null_rate_diff: f64,
    pub distinct_count_ratio: f64,
    pub significant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceDefinition {
    pub name: String,
    pub condition: String,
    pub column: String,
    pub operator: SliceOperator,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SliceOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Contains,
    In,
    Between,
    IsNull,
    IsNotNull,
}

impl std::fmt::Display for SliceOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equals => write!(f, "equals"),
            Self::NotEquals => write!(f, "not_equals"),
            Self::GreaterThan => write!(f, "greater_than"),
            Self::LessThan => write!(f, "less_than"),
            Self::GreaterOrEqual => write!(f, "greater_or_equal"),
            Self::LessOrEqual => write!(f, "less_or_equal"),
            Self::Contains => write!(f, "contains"),
            Self::In => write!(f, "in"),
            Self::Between => write!(f, "between"),
            Self::IsNull => write!(f, "is_null"),
            Self::IsNotNull => write!(f, "is_not_null"),
        }
    }
}

pub struct SliceAnalyzer;

impl SliceAnalyzer {
    pub fn analyze(
        dataset_id: &str,
        rows: &[Vec<String>],
        column_names: &[String],
        label_column: Option<&str>,
        slice_definitions: &[SliceDefinition],
    ) -> SliceAnalysisReport {
        let total_rows = rows.len();
        let mut slices = Vec::new();

        for def in slice_definitions {
            let col_idx = column_names.iter().position(|c| c == &def.column);

            let slice_rows: Vec<&Vec<String>> = rows.iter()
                .filter(|row| {
                    if let Some(ci) = col_idx {
                        if ci < row.len() {
                            Self::match_condition(&row[ci], def.operator, &def.value)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .collect();

            let row_count = slice_rows.len();
            let row_ratio = if total_rows > 0 {
                row_count as f64 / total_rows as f64
            } else {
                0.0
            };

            let column_stats = Self::compute_slice_column_stats(&slice_rows, column_names);
            let label_distribution = label_column.map(|lc| {
                Self::compute_slice_label_dist(&slice_rows, column_names, lc)
            });
            let quality_metrics = Self::compute_slice_quality(&column_stats, &label_distribution, row_count, total_rows);

            slices.push(DataSlice {
                slice_name: def.name.clone(),
                slice_condition: def.condition.clone(),
                row_count,
                row_ratio,
                column_stats,
                label_distribution,
                quality_metrics,
            });
        }

        let cross_slice_comparison = if slices.len() >= 2 {
            Some(Self::compare_slices(&slices))
        } else {
            None
        };

        let recommendations = Self::generate_recommendations(&slices, &cross_slice_comparison);

        SliceAnalysisReport {
            dataset_id: dataset_id.to_string(),
            total_rows,
            slices,
            cross_slice_comparison,
            recommendations,
        }
    }

    fn match_condition(value: &str, operator: SliceOperator, target: &str) -> bool {
        match operator {
            SliceOperator::Equals => value == target,
            SliceOperator::NotEquals => value != target,
            SliceOperator::GreaterThan => {
                if let (Ok(v), Ok(t)) = (value.parse::<f64>(), target.parse::<f64>()) {
                    v > t
                } else {
                    value > target
                }
            }
            SliceOperator::LessThan => {
                if let (Ok(v), Ok(t)) = (value.parse::<f64>(), target.parse::<f64>()) {
                    v < t
                } else {
                    value < target
                }
            }
            SliceOperator::GreaterOrEqual => {
                if let (Ok(v), Ok(t)) = (value.parse::<f64>(), target.parse::<f64>()) {
                    v >= t
                } else {
                    value >= target
                }
            }
            SliceOperator::LessOrEqual => {
                if let (Ok(v), Ok(t)) = (value.parse::<f64>(), target.parse::<f64>()) {
                    v <= t
                } else {
                    value <= target
                }
            }
            SliceOperator::Contains => value.contains(target),
            SliceOperator::In => target.split(',').any(|t| value == t.trim()),
            SliceOperator::Between => {
                let parts: Vec<&str> = target.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(v), Ok(lo), Ok(hi)) = (
                        value.parse::<f64>(),
                        parts[0].trim().parse::<f64>(),
                        parts[1].trim().parse::<f64>(),
                    ) {
                        v >= lo && v <= hi
                    } else {
                        value >= parts[0].trim() && value <= parts[1].trim()
                    }
                } else {
                    false
                }
            }
            SliceOperator::IsNull => value.is_empty() || value == "null" || value == "NULL",
            SliceOperator::IsNotNull => !value.is_empty() && value != "null" && value != "NULL",
        }
    }

    fn compute_slice_column_stats(
        rows: &[&Vec<String>],
        column_names: &[String],
    ) -> Vec<SliceColumnStat> {
        column_names.iter().enumerate().map(|(ci, name)| {
            let values: Vec<&str> = rows.iter()
                .filter_map(|row| row.get(ci).map(|v| v.as_str()))
                .collect();

            let total = values.len();
            let null_count = values.iter().filter(|v| v.is_empty()).count();
            let null_rate = if total > 0 { null_count as f64 / total as f64 } else { 0.0 };

            let non_null: Vec<&str> = values.iter()
                .filter(|v| !v.is_empty())
                .copied()
                .collect();

            let distinct_count = {
                let mut s: Vec<&&str> = non_null.iter().collect();
                s.sort();
                s.dedup();
                s.len()
            };

            let nums: Vec<f64> = non_null.iter()
                .filter_map(|v| v.parse::<f64>().ok())
                .collect();

            let mean_value = if !nums.is_empty() {
                Some(nums.iter().sum::<f64>() / nums.len() as f64)
            } else {
                None
            };

            let std_value = if nums.len() > 1 {
                let mean = mean_value.unwrap();
                let variance = nums.iter()
                    .map(|x| (x - mean) * (x - mean))
                    .sum::<f64>() / (nums.len() - 1) as f64;
                Some(variance.sqrt())
            } else {
                None
            };

            let min_value = non_null.iter().min().map(|s| s.to_string());
            let max_value = non_null.iter().max().map(|s| s.to_string());

            SliceColumnStat {
                column_name: name.clone(),
                null_count,
                null_rate,
                distinct_count,
                mean_value,
                std_value,
                min_value,
                max_value,
            }
        }).collect()
    }

    fn compute_slice_label_dist(
        rows: &[&Vec<String>],
        column_names: &[String],
        label_column: &str,
    ) -> Vec<SliceLabelDist> {
        let col_idx = column_names.iter().position(|c| c == label_column);
        if col_idx.is_none() {
            return Vec::new();
        }
        let ci = col_idx.unwrap();

        let mut counts: HashMap<&str, usize> = HashMap::new();
        let mut total = 0;

        for row in rows {
            if let Some(val) = row.get(ci) {
                if !val.is_empty() {
                    *counts.entry(val.as_str()).or_insert(0) += 1;
                    total += 1;
                }
            }
        }

        counts.into_iter()
            .map(|(label, count)| SliceLabelDist {
                label_value: label.to_string(),
                count,
                ratio: if total > 0 { count as f64 / total as f64 } else { 0.0 },
            })
            .collect()
    }

    fn compute_slice_quality(
        column_stats: &[SliceColumnStat],
        label_distribution: &Option<Vec<SliceLabelDist>>,
        row_count: usize,
        total_rows: usize,
    ) -> SliceQualityMetrics {
        let completeness_score = if column_stats.is_empty() {
            1.0
        } else {
            let avg_null_rate: f64 = column_stats.iter()
                .map(|s| s.null_rate)
                .sum::<f64>() / column_stats.len() as f64;
            (1.0 - avg_null_rate).max(0.0)
        };

        let balance_score = match label_distribution {
            Some(dist) if !dist.is_empty() => {
                let ratios: Vec<f64> = dist.iter().map(|d| d.ratio).collect();
                let max_ratio = ratios.iter().cloned().fold(0.0, f64::max);
                let min_ratio = ratios.iter().cloned().fold(1.0, f64::min);
                1.0 - (max_ratio - min_ratio)
            }
            _ => 1.0,
        };

        let representativeness_score = if total_rows > 0 {
            let ratio = row_count as f64 / total_rows as f64;
            if ratio < 0.01 {
                0.3
            } else if ratio < 0.05 {
                0.6
            } else if ratio < 0.1 {
                0.8
            } else {
                1.0
            }
        } else {
            0.0
        };

        let overall_score = completeness_score * 0.4 + balance_score * 0.3 + representativeness_score * 0.3;

        SliceQualityMetrics {
            completeness_score,
            balance_score,
            representativeness_score,
            overall_score,
        }
    }

    fn compare_slices(slices: &[DataSlice]) -> CrossSliceComparison {
        let mut slice_pairs = Vec::new();
        let mut divergence_scores: HashMap<String, f64> = HashMap::new();

        for i in 0..slices.len() {
            for j in (i + 1)..slices.len() {
                let a = &slices[i];
                let b = &slices[j];

                let row_count_ratio = if b.row_count > 0 {
                    a.row_count as f64 / b.row_count as f64
                } else {
                    0.0
                };

                let column_diffs: Vec<ColumnDiff> = a.column_stats.iter()
                    .zip(b.column_stats.iter())
                    .map(|(sa, sb)| {
                        let mean_diff = match (sa.mean_value, sb.mean_value) {
                            (Some(ma), Some(mb)) => Some(ma - mb),
                            _ => None,
                        };
                        let null_rate_diff = (sa.null_rate - sb.null_rate).abs();
                        let distinct_count_ratio = if sb.distinct_count > 0 {
                            sa.distinct_count as f64 / sb.distinct_count as f64
                        } else {
                            0.0
                        };
                        let significant = null_rate_diff > 0.1
                            || mean_diff.map(|d| d.abs() > 0.5).unwrap_or(false)
                            || (distinct_count_ratio - 1.0).abs() > 0.3;

                        ColumnDiff {
                            column_name: sa.column_name.clone(),
                            mean_diff,
                            null_rate_diff,
                            distinct_count_ratio,
                            significant,
                        }
                    })
                    .collect();

                let label_distribution_shift = match (&a.label_distribution, &b.label_distribution) {
                    (Some(da), Some(db)) => {
                        let mut shift = 0.0;
                        let all_labels: std::collections::HashSet<&str> = da.iter()
                            .map(|d| d.label_value.as_str())
                            .chain(db.iter().map(|d| d.label_value.as_str()))
                            .collect();
                        for label in all_labels {
                            let ra = da.iter().find(|d| d.label_value == label).map(|d| d.ratio).unwrap_or(0.0);
                            let rb = db.iter().find(|d| d.label_value == label).map(|d| d.ratio).unwrap_or(0.0);
                            shift += (ra - rb).abs();
                        }
                        Some(shift / 2.0)
                    }
                    _ => None,
                };

                let significant_diffs = column_diffs.iter().filter(|d| d.significant).count();
                let overall_similarity = 1.0 - (significant_diffs as f64 / column_diffs.len().max(1) as f64);

                let divergence = 1.0 - overall_similarity;
                *divergence_scores.entry(a.slice_name.clone()).or_insert(0.0) += divergence;
                *divergence_scores.entry(b.slice_name.clone()).or_insert(0.0) += divergence;

                slice_pairs.push(SlicePairComparison {
                    slice_a: a.slice_name.clone(),
                    slice_b: b.slice_name.clone(),
                    row_count_ratio,
                    column_diffs,
                    label_distribution_shift,
                    overall_similarity,
                });
            }
        }

        let mut sorted_divergence: Vec<(String, f64)> = divergence_scores.into_iter().collect();
        sorted_divergence.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let most_divergent_slices: Vec<String> = sorted_divergence.iter()
            .take(3)
            .map(|(name, _)| name.clone())
            .collect();

        let distribution_shift_detected = slice_pairs.iter()
            .any(|p| p.label_distribution_shift.unwrap_or(0.0) > 0.1);

        CrossSliceComparison {
            slice_pairs,
            most_divergent_slices,
            distribution_shift_detected,
        }
    }

    fn generate_recommendations(
        slices: &[DataSlice],
        cross_comparison: &Option<CrossSliceComparison>,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        let small_slices: Vec<&str> = slices.iter()
            .filter(|s| s.row_ratio < 0.05)
            .map(|s| s.slice_name.as_str())
            .collect();

        if !small_slices.is_empty() {
            recs.push(format!(
                "⚠️ 切片 {} 样本量不足（<5%），模型在这些子群上可能表现不佳",
                small_slices.join(", ")
            ));
        }

        let low_quality_slices: Vec<&str> = slices.iter()
            .filter(|s| s.quality_metrics.overall_score < 0.5)
            .map(|s| s.slice_name.as_str())
            .collect();

        if !low_quality_slices.is_empty() {
            recs.push(format!(
                "切片 {} 质量评分低，建议检查数据采集过程",
                low_quality_slices.join(", ")
            ));
        }

        if let Some(cc) = cross_comparison {
            if cc.distribution_shift_detected {
                recs.push("🔴 检测到切片间标签分布显著偏移，模型在不同子群上可能表现不一致".to_string());
            }

            if !cc.most_divergent_slices.is_empty() {
                recs.push(format!(
                    "差异最大的切片: {}，建议分别评估模型在这些切片上的性能",
                    cc.most_divergent_slices.join(", ")
                ));
            }
        }

        if recs.is_empty() {
            recs.push("✅ 所有切片质量良好，分布一致".to_string());
        }

        recs
    }
}
