use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub dataset_id: String,
    pub dataset_name: String,
    pub overall_score: f64,
    pub grade: QualityGrade,
    pub dimensions: Vec<QualityDimension>,
    pub issues: Vec<QualityIssue>,
    pub recommendations: Vec<String>,
    pub scored_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityGrade {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical,
}

impl std::fmt::Display for QualityGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Excellent => write!(f, "优秀"),
            Self::Good => write!(f, "良好"),
            Self::Fair => write!(f, "一般"),
            Self::Poor => write!(f, "较差"),
            Self::Critical => write!(f, "严重"),
        }
    }
}

impl QualityGrade {
    pub fn from_score(score: f64) -> Self {
        if score >= 90.0 {
            Self::Excellent
        } else if score >= 75.0 {
            Self::Good
        } else if score >= 60.0 {
            Self::Fair
        } else if score >= 40.0 {
            Self::Poor
        } else {
            Self::Critical
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Self::Excellent => "#10b981",
            Self::Good => "#3b82f6",
            Self::Fair => "#f59e0b",
            Self::Poor => "#f97316",
            Self::Critical => "#ef4444",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::Excellent => "⭐",
            Self::Good => "✅",
            Self::Fair => "⚠️",
            Self::Poor => "🔶",
            Self::Critical => "🔴",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDimension {
    pub name: String,
    pub label: String,
    pub score: f64,
    pub weight: f64,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub severity: IssueSeverity,
    pub category: String,
    pub description: String,
    pub affected_columns: Vec<String>,
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

pub struct QualityScorer;

impl QualityScorer {
    pub fn score(
        dataset_id: &str,
        dataset_name: &str,
        rows: usize,
        columns: usize,
        column_profiles: &[crate::domain::dataset::aggregate::ColumnProfile],
        has_missing_values: bool,
    ) -> QualityScore {
        let mut dimensions = Vec::new();
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        let completeness = Self::score_completeness(column_profiles, rows, &mut issues);
        dimensions.push(QualityDimension {
            name: "completeness".to_string(),
            label: "完整性".to_string(),
            score: completeness,
            weight: 0.35,
            details: format!("数据缺失率评估，基于 {} 列的缺失值分析", column_profiles.len()),
        });

        let consistency = Self::score_consistency(column_profiles, &mut issues);
        dimensions.push(QualityDimension {
            name: "consistency".to_string(),
            label: "一致性".to_string(),
            score: consistency,
            weight: 0.25,
            details: "数据类型一致性和值域合理性评估".to_string(),
        });

        let information = Self::score_information(column_profiles, rows, &mut issues);
        dimensions.push(QualityDimension {
            name: "information".to_string(),
            label: "信息密度".to_string(),
            score: information,
            weight: 0.25,
            details: "数据信息量和区分度评估".to_string(),
        });

        let balance = Self::score_balance(column_profiles, rows, &mut issues);
        dimensions.push(QualityDimension {
            name: "balance".to_string(),
            label: "数据均衡性".to_string(),
            score: balance,
            weight: 0.15,
            details: "数据分布均衡性评估".to_string(),
        });

        let overall_score = dimensions.iter()
            .map(|d| d.score * d.weight)
            .sum::<f64>();

        let grade = QualityGrade::from_score(overall_score);

        if completeness < 60.0 {
            recommendations.push(format!(
                "数据缺失率较高（完整性 {:.0}分），建议使用数据工坊的填充缺失值功能处理",
                completeness
            ));
        }
        if consistency < 60.0 {
            recommendations.push("数据一致性较低，建议检查数据格式和值域范围".to_string());
        }
        if information < 50.0 {
            recommendations.push("信息密度不足，部分列可能包含大量重复值，建议检查数据质量".to_string());
        }
        if balance < 50.0 {
            recommendations.push("数据分布不均衡，可能影响模型训练效果，建议进行重采样".to_string());
        }
        if has_missing_values {
            recommendations.push("数据集包含缺失值，建议在训练前进行缺失值处理".to_string());
        }
        if rows < 1000 {
            recommendations.push(format!(
                "数据集仅 {} 行，数据量较小，建议收集更多数据或使用数据增强",
                rows
            ));
        }
        if columns < 3 {
            recommendations.push("特征列较少，可能不足以支撑复杂模型训练".to_string());
        }

        QualityScore {
            dataset_id: dataset_id.to_string(),
            dataset_name: dataset_name.to_string(),
            overall_score,
            grade,
            dimensions,
            issues,
            recommendations,
            scored_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn score_completeness(
        profiles: &[crate::domain::dataset::aggregate::ColumnProfile],
        total_rows: usize,
        issues: &mut Vec<QualityIssue>,
    ) -> f64 {
        if profiles.is_empty() || total_rows == 0 {
            return 0.0;
        }

        let mut total_missing = 0usize;
        let mut worst_col = String::new();
        let mut worst_ratio = 0.0;

        for col in profiles {
            total_missing += col.null_count;
            let ratio = col.null_count as f64 / col.total_count.max(1) as f64;
            if ratio > worst_ratio {
                worst_ratio = ratio;
                worst_col = col.name.clone();
            }
        }

        let overall_ratio = total_missing as f64 / (total_rows * profiles.len()).max(1) as f64;
        let score = ((1.0 - overall_ratio) * 100.0).max(0.0);

        if worst_ratio > 0.3 {
            issues.push(QualityIssue {
                severity: IssueSeverity::Error,
                category: "completeness".to_string(),
                description: format!("列 '{}' 缺失率高达 {:.1}%", worst_col, worst_ratio * 100.0),
                affected_columns: vec![worst_col],
                suggestion: "建议使用填充缺失值或删除该列".to_string(),
            });
        } else if worst_ratio > 0.1 {
            issues.push(QualityIssue {
                severity: IssueSeverity::Warning,
                category: "completeness".to_string(),
                description: format!("列 '{}' 缺失率 {:.1}%，建议处理", worst_col, worst_ratio * 100.0),
                affected_columns: vec![worst_col],
                suggestion: "可使用均值/中位数/众数填充".to_string(),
            });
        }

        score
    }

    fn score_consistency(
        profiles: &[crate::domain::dataset::aggregate::ColumnProfile],
        issues: &mut Vec<QualityIssue>,
    ) -> f64 {
        if profiles.is_empty() {
            return 100.0;
        }

        let mut penalty = 0.0;
        let max_penalty_per_col = 100.0 / profiles.len() as f64;

        for col in profiles {
            let mut col_penalty = 0.0;

            if col.column_type == crate::domain::dataset::aggregate::ColumnType::Unknown {
                col_penalty += max_penalty_per_col * 0.5;
                issues.push(QualityIssue {
                    severity: IssueSeverity::Warning,
                    category: "consistency".to_string(),
                    description: format!("列 '{}' 类型未知，可能包含混合类型数据", col.name),
                    affected_columns: vec![col.name.clone()],
                    suggestion: "建议检查并统一该列的数据类型".to_string(),
                });
            }

            if col.distinct_count == 0 && col.total_count > 0 {
                col_penalty += max_penalty_per_col * 0.3;
            }

            penalty += col_penalty.min(max_penalty_per_col);
        }

        (100.0 - penalty).max(0.0)
    }

    fn score_information(
        profiles: &[crate::domain::dataset::aggregate::ColumnProfile],
        total_rows: usize,
        issues: &mut Vec<QualityIssue>,
    ) -> f64 {
        if profiles.is_empty() || total_rows == 0 {
            return 0.0;
        }

        let mut total_score = 0.0;

        for col in profiles {
            let distinct_ratio = col.distinct_count as f64 / col.total_count.max(1) as f64;

            let col_score = if distinct_ratio < 0.01 && col.total_count > 100 {
                issues.push(QualityIssue {
                    severity: IssueSeverity::Warning,
                    category: "information".to_string(),
                    description: format!(
                        "列 '{}' 唯一值比例仅 {:.2}%，信息量极低",
                        col.name, distinct_ratio * 100.0
                    ),
                    affected_columns: vec![col.name.clone()],
                    suggestion: "该列可能为常量列，建议考虑删除".to_string(),
                });
                20.0
            } else if distinct_ratio < 0.05 {
                50.0
            } else if distinct_ratio > 0.9 {
                95.0
            } else {
                75.0 + distinct_ratio * 20.0
            };

            total_score += col_score;
        }

        (total_score / profiles.len() as f64).min(100.0)
    }

    fn score_balance(
        profiles: &[crate::domain::dataset::aggregate::ColumnProfile],
        total_rows: usize,
        issues: &mut Vec<QualityIssue>,
    ) -> f64 {
        if profiles.is_empty() || total_rows == 0 {
            return 100.0;
        }

        let mut score = 100.0;

        for col in profiles {
            if col.top_values.is_empty() {
                continue;
            }

            let top1_ratio = col.top_values[0].1 as f64 / col.total_count.max(1) as f64;

            if top1_ratio > 0.9 && col.total_count > 50 {
                score -= 20.0;
                issues.push(QualityIssue {
                    severity: IssueSeverity::Warning,
                    category: "balance".to_string(),
                    description: format!(
                        "列 '{}' 中 '{}' 占比 {:.1}%，分布严重不均衡",
                        col.name, col.top_values[0].0, top1_ratio * 100.0
                    ),
                    affected_columns: vec![col.name.clone()],
                    suggestion: "建议进行重采样或使用类别权重".to_string(),
                });
            } else if top1_ratio > 0.7 {
                score -= 10.0;
            }
        }

        (score as f64).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_grade_from_score() {
        assert_eq!(QualityGrade::from_score(95.0), QualityGrade::Excellent);
        assert_eq!(QualityGrade::from_score(80.0), QualityGrade::Good);
        assert_eq!(QualityGrade::from_score(65.0), QualityGrade::Fair);
        assert_eq!(QualityGrade::from_score(50.0), QualityGrade::Poor);
        assert_eq!(QualityGrade::from_score(30.0), QualityGrade::Critical);
    }
}
