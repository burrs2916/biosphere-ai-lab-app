use serde::{Deserialize, Serialize};

use crate::domain::dataset::aggregate::{ColumnProfile, DatasetVersionRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDriftReport {
    pub from_version: String,
    pub to_version: String,
    pub overall_severity: DriftSeverity,
    pub schema_drift: SchemaDrift,
    pub distribution_drifts: Vec<ColumnDrift>,
    pub volume_drift: VolumeDrift,
    pub summary: String,
    pub action_required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDrift {
    pub columns_added: Vec<String>,
    pub columns_removed: Vec<String>,
    pub type_changes: Vec<ColumnTypeDrift>,
    pub has_drift: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnTypeDrift {
    pub column_name: String,
    pub from_type: String,
    pub to_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDrift {
    pub column_name: String,
    pub drift_type: DriftType,
    pub severity: DriftSeverity,
    pub psi: Option<f64>,
    pub mean_shift: Option<f64>,
    pub std_shift: Option<f64>,
    pub null_rate_change: Option<f64>,
    pub distinct_count_change: Option<i64>,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftType {
    Schema,
    Distribution,
    Volume,
    NullRate,
    Cardinality,
}

impl std::fmt::Display for DriftType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema => write!(f, "schema"),
            Self::Distribution => write!(f, "distribution"),
            Self::Volume => write!(f, "volume"),
            Self::NullRate => write!(f, "null_rate"),
            Self::Cardinality => write!(f, "cardinality"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for DriftSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeDrift {
    pub from_rows: usize,
    pub to_rows: usize,
    pub change_ratio: f64,
    pub change_percent: f64,
    pub severity: DriftSeverity,
}

pub struct DataDriftAnalyzer;

impl DataDriftAnalyzer {
    pub fn analyze(from: &DatasetVersionRecord, to: &DatasetVersionRecord) -> DataDriftReport {
        let schema_drift = Self::detect_schema_drift(&from.column_profiles, &to.column_profiles);
        let distribution_drifts = Self::detect_distribution_drifts(&from.column_profiles, &to.column_profiles);
        let volume_drift = Self::detect_volume_drift(from.rows, to.rows);

        let overall_severity = Self::compute_overall_severity(&schema_drift, &distribution_drifts, &volume_drift);
        let summary = Self::generate_summary(&schema_drift, &distribution_drifts, &volume_drift);
        let action_required = Self::generate_actions(&schema_drift, &distribution_drifts, &volume_drift, &overall_severity);

        DataDriftReport {
            from_version: from.version.clone(),
            to_version: to.version.clone(),
            overall_severity,
            schema_drift,
            distribution_drifts,
            volume_drift,
            summary,
            action_required,
        }
    }

    fn detect_schema_drift(from: &[ColumnProfile], to: &[ColumnProfile]) -> SchemaDrift {
        use std::collections::HashMap;
        let from_map: HashMap<&str, &ColumnProfile> = from.iter().map(|p| (p.name.as_str(), p)).collect();
        let to_map: HashMap<&str, &ColumnProfile> = to.iter().map(|p| (p.name.as_str(), p)).collect();

        let columns_added: Vec<String> = to_map.keys()
            .filter(|k| !from_map.contains_key(*k))
            .map(|k| k.to_string())
            .collect();

        let columns_removed: Vec<String> = from_map.keys()
            .filter(|k| !to_map.contains_key(*k))
            .map(|k| k.to_string())
            .collect();

        let type_changes: Vec<ColumnTypeDrift> = from_map.iter()
            .filter_map(|(name, from_p)| {
                to_map.get(name).and_then(|to_p| {
                    if from_p.column_type != to_p.column_type {
                        Some(ColumnTypeDrift {
                            column_name: name.to_string(),
                            from_type: from_p.column_type.to_string(),
                            to_type: to_p.column_type.to_string(),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        let has_drift = !columns_added.is_empty() || !columns_removed.is_empty() || !type_changes.is_empty();

        SchemaDrift {
            columns_added,
            columns_removed,
            type_changes,
            has_drift,
        }
    }

    fn detect_distribution_drifts(from: &[ColumnProfile], to: &[ColumnProfile]) -> Vec<ColumnDrift> {
        use std::collections::HashMap;
        let from_map: HashMap<&str, &ColumnProfile> = from.iter().map(|p| (p.name.as_str(), p)).collect();
        let to_map: HashMap<&str, &ColumnProfile> = to.iter().map(|p| (p.name.as_str(), p)).collect();

        let mut drifts = Vec::new();

        for (name, to_p) in &to_map {
            if let Some(from_p) = from_map.get(name) {
                let mut column_drifts = Vec::new();

                let from_null_rate = from_p.null_count as f64 / from_p.total_count as f64;
                let to_null_rate = to_p.null_count as f64 / to_p.total_count as f64;
                let null_rate_change = (to_null_rate - from_null_rate).abs();
                if null_rate_change > 0.01 {
                    let severity = if null_rate_change < 0.05 {
                        DriftSeverity::Low
                    } else if null_rate_change < 0.15 {
                        DriftSeverity::Medium
                    } else {
                        DriftSeverity::High
                    };
                    column_drifts.push(ColumnDrift {
                        column_name: name.to_string(),
                        drift_type: DriftType::NullRate,
                        severity,
                        psi: None,
                        mean_shift: None,
                        std_shift: None,
                        null_rate_change: Some(null_rate_change),
                        distinct_count_change: None,
                        details: format!("空值率从 {:.2}% 变为 {:.2}% (变化 {:.2}%)",
                            from_null_rate * 100.0, to_null_rate * 100.0, null_rate_change * 100.0),
                    });
                }

                let distinct_change = to_p.distinct_count as i64 - from_p.distinct_count as i64;
                if distinct_change.abs() > 0 {
                    let change_ratio = distinct_change.abs() as f64 / from_p.distinct_count.max(1) as f64;
                    if change_ratio > 0.1 {
                        let severity = if change_ratio < 0.3 {
                            DriftSeverity::Low
                        } else if change_ratio < 0.5 {
                            DriftSeverity::Medium
                        } else {
                            DriftSeverity::High
                        };
                        column_drifts.push(ColumnDrift {
                            column_name: name.to_string(),
                            drift_type: DriftType::Cardinality,
                            severity,
                            psi: None,
                            mean_shift: None,
                            std_shift: None,
                            null_rate_change: None,
                            distinct_count_change: Some(distinct_change),
                            details: format!("唯一值数从 {} 变为 {} ({:+})", from_p.distinct_count, to_p.distinct_count, distinct_change),
                        });
                    }
                }

                if from_p.is_numeric() && to_p.is_numeric() {
                    let mean_shift = match (from_p.mean_value, to_p.mean_value) {
                        (Some(f_m), Some(t_m)) => {
                            let shift = (t_m - f_m).abs();
                            let relative = if f_m.abs() > 1e-10 { shift / f_m.abs() } else { shift };
                            Some(relative)
                        }
                        _ => None,
                    };

                    let std_shift = match (from_p.std_value, to_p.std_value) {
                        (Some(f_s), Some(t_s)) => {
                            let shift = (t_s - f_s).abs();
                            let relative = if f_s.abs() > 1e-10 { shift / f_s.abs() } else { shift };
                            Some(relative)
                        }
                        _ => None,
                    };

                    let psi = Self::estimate_psi(from_p, to_p);

                    let has_significant_shift = mean_shift.map_or(false, |s| s > 0.1)
                        || std_shift.map_or(false, |s| s > 0.2)
                        || psi.map_or(false, |p| p > 0.1);

                    if has_significant_shift {
                        let severity = if psi.map_or(f64::MAX, |p| p) < 0.1 {
                            DriftSeverity::None
                        } else if psi.map_or(f64::MAX, |p| p) < 0.25 {
                            DriftSeverity::Low
                        } else if psi.map_or(f64::MAX, |p| p) < 0.5 {
                            DriftSeverity::Medium
                        } else {
                            DriftSeverity::High
                        };

                        if severity != DriftSeverity::None {
                            column_drifts.push(ColumnDrift {
                                column_name: name.to_string(),
                                drift_type: DriftType::Distribution,
                                severity,
                                psi,
                                mean_shift,
                                std_shift,
                                null_rate_change: None,
                                distinct_count_change: None,
                                details: format!(
                                    "分布漂移: PSI={:.4}, 均值相对偏移={:.4}, 标准差相对偏移={:.4}",
                                    psi.unwrap_or(0.0),
                                    mean_shift.unwrap_or(0.0),
                                    std_shift.unwrap_or(0.0),
                                ),
                            });
                        }
                    }
                }

                drifts.extend(column_drifts);
            }
        }

        drifts
    }

    fn estimate_psi(from: &ColumnProfile, to: &ColumnProfile) -> Option<f64> {
        let from_mean = from.mean_value?;
        let to_mean = to.mean_value?;
        let from_std = from.std_value.unwrap_or(1.0).max(1e-10);
        let to_std = to.std_value.unwrap_or(1.0).max(1e-10);

        let mean_shift = ((to_mean - from_mean).abs() / from_std).min(10.0);
        let std_ratio = (to_std / from_std).ln().abs().min(10.0);

        let psi = 0.5 * mean_shift.powi(2) + 0.5 * std_ratio;
        Some(psi)
    }

    fn detect_volume_drift(from_rows: usize, to_rows: usize) -> VolumeDrift {
        let change_ratio = if from_rows > 0 {
            to_rows as f64 / from_rows as f64
        } else {
            1.0
        };
        let change_percent = (change_ratio - 1.0) * 100.0;

        let severity = if change_percent.abs() < 5.0 {
            DriftSeverity::None
        } else if change_percent.abs() < 20.0 {
            DriftSeverity::Low
        } else if change_percent.abs() < 50.0 {
            DriftSeverity::Medium
        } else {
            DriftSeverity::High
        };

        VolumeDrift {
            from_rows,
            to_rows,
            change_ratio,
            change_percent,
            severity,
        }
    }

    fn compute_overall_severity(schema: &SchemaDrift, distribution: &[ColumnDrift], volume: &VolumeDrift) -> DriftSeverity {
        let mut max_severity = DriftSeverity::None;

        if schema.has_drift {
            max_severity = DriftSeverity::High;
        }

        if !schema.columns_removed.is_empty() {
            max_severity = DriftSeverity::Critical;
        }

        for d in distribution {
            if d.severity as u8 > max_severity as u8 {
                max_severity = d.severity;
            }
        }

        if volume.severity as u8 > max_severity as u8 {
            max_severity = volume.severity;
        }

        max_severity
    }

    fn generate_summary(schema: &SchemaDrift, distribution: &[ColumnDrift], volume: &VolumeDrift) -> String {
        let mut parts = Vec::new();

        if !schema.columns_added.is_empty() {
            parts.push(format!("新增 {} 列", schema.columns_added.len()));
        }
        if !schema.columns_removed.is_empty() {
            parts.push(format!("移除 {} 列", schema.columns_removed.len()));
        }
        if !schema.type_changes.is_empty() {
            parts.push(format!("{} 列类型变更", schema.type_changes.len()));
        }
        if !distribution.is_empty() {
            parts.push(format!("{} 列存在分布漂移", distribution.len()));
        }
        if volume.severity != DriftSeverity::None {
            parts.push(format!("数据量变化 {:.1}%", volume.change_percent));
        }

        if parts.is_empty() {
            "未检测到显著数据漂移".to_string()
        } else {
            parts.join("，")
        }
    }

    fn generate_actions(schema: &SchemaDrift, distribution: &[ColumnDrift], volume: &VolumeDrift, severity: &DriftSeverity) -> Vec<String> {
        let mut actions = Vec::new();

        if !schema.columns_removed.is_empty() {
            actions.push(format!("⚠️ 列被移除: {}，依赖这些列的训练任务将失败", schema.columns_removed.join(", ")));
        }
        if !schema.columns_added.is_empty() {
            actions.push(format!("新增列: {}，考虑是否需要更新特征工程", schema.columns_added.join(", ")));
        }
        if !schema.type_changes.is_empty() {
            actions.push("列类型变更可能导致数据解析错误，建议检查预处理管道".to_string());
        }

        let high_drift_cols: Vec<&str> = distribution.iter()
            .filter(|d| d.severity as u8 >= DriftSeverity::Medium as u8)
            .map(|d| d.column_name.as_str())
            .collect();
        if !high_drift_cols.is_empty() {
            actions.push(format!("高漂移列: {}，建议重新训练模型并评估性能变化", high_drift_cols.join(", ")));
        }

        if volume.change_percent.abs() > 50.0 {
            actions.push("数据量变化超过 50%，建议重新评估模型是否需要调整容量".to_string());
        }

        match severity {
            DriftSeverity::Critical => actions.insert(0, "🔴 严重漂移！建议暂停使用该数据集进行训练，直到问题解决".to_string()),
            DriftSeverity::High => actions.insert(0, "🟠 显著漂移检测到，建议重新训练并验证模型性能".to_string()),
            DriftSeverity::Medium => actions.insert(0, "🟡 中等漂移，建议监控模型性能指标".to_string()),
            _ => {}
        }

        actions
    }
}
