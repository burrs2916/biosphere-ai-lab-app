use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowDiffReport {
    pub dataset_id: String,
    pub from_version: String,
    pub to_version: String,
    pub from_rows: usize,
    pub to_rows: usize,
    pub added_rows: usize,
    pub removed_rows: usize,
    pub modified_rows: usize,
    pub unchanged_rows: usize,
    pub change_rate: f64,
    pub severity: RowDiffSeverity,
    pub added_indices: Vec<usize>,
    pub removed_indices: Vec<usize>,
    pub modified_details: Vec<RowModification>,
    pub column_changes: Vec<ColumnChangeSummary>,
    pub summary: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowDiffSeverity {
    Minor,
    Moderate,
    Major,
    Breaking,
}

impl std::fmt::Display for RowDiffSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Minor => write!(f, "minor"),
            Self::Moderate => write!(f, "moderate"),
            Self::Major => write!(f, "major"),
            Self::Breaking => write!(f, "breaking"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowModification {
    pub row_index: usize,
    pub changed_columns: Vec<String>,
    pub change_type: ModificationType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModificationType {
    ValueChanged,
    NullBecameValue,
    ValueBecameNull,
    TypeChanged,
}

impl std::fmt::Display for ModificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValueChanged => write!(f, "value_changed"),
            Self::NullBecameValue => write!(f, "null_became_value"),
            Self::ValueBecameNull => write!(f, "value_became_null"),
            Self::TypeChanged => write!(f, "type_changed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnChangeSummary {
    pub column_name: String,
    pub values_changed: usize,
    pub nulls_added: usize,
    pub nulls_removed: usize,
    pub types_changed: usize,
    pub change_rate: f64,
}

pub struct RowDiffAnalyzer;

impl RowDiffAnalyzer {
    pub fn diff(
        dataset_id: &str,
        from_version: &str,
        to_version: &str,
        from_rows: &[Vec<String>],
        to_rows: &[Vec<String>],
        column_names: &[String],
    ) -> RowDiffReport {
        let from_count = from_rows.len();
        let to_count = to_rows.len();

        let from_hashes: HashMap<String, usize> = from_rows.iter()
            .enumerate()
            .map(|(i, row)| (Self::hash_row(row), i))
            .collect();

        let to_hashes: HashMap<String, usize> = to_rows.iter()
            .enumerate()
            .map(|(i, row)| (Self::hash_row(row), i))
            .collect();

        let from_hash_set: HashSet<&String> = from_hashes.keys().collect();
        let to_hash_set: HashSet<&String> = to_hashes.keys().collect();

        let added_hashes: Vec<&String> = to_hash_set.difference(&from_hash_set).copied().collect();
        let removed_hashes: Vec<&String> = from_hash_set.difference(&to_hash_set).copied().collect();
        let unchanged_hashes: Vec<&String> = from_hash_set.intersection(&to_hash_set).copied().collect();

        let added_rows_count = added_hashes.len();
        let removed_rows_count = removed_hashes.len();
        let unchanged_rows_count = unchanged_hashes.len();

        let added_indices: Vec<usize> = added_hashes.iter()
            .filter_map(|h| to_hashes.get(*h))
            .copied()
            .collect();

        let removed_indices: Vec<usize> = removed_hashes.iter()
            .filter_map(|h| from_hashes.get(*h))
            .copied()
            .collect();

        let mut modified_details = Vec::new();
        let mut column_changes_map: HashMap<String, ColumnChangeSummary> = HashMap::new();

        for hash in &unchanged_hashes {
            if let (Some(&from_idx), Some(&to_idx)) = (from_hashes.get(*hash), to_hashes.get(*hash)) {
                let from_row = &from_rows[from_idx];
                let to_row = &to_rows[to_idx];

                let mut changed_cols = Vec::new();
                let mut change_type = ModificationType::ValueChanged;

                for (col_idx, col_name) in column_names.iter().enumerate() {
                    let from_val = from_row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                    let to_val = to_row.get(col_idx).map(|s| s.as_str()).unwrap_or("");

                    if from_val != to_val {
                        changed_cols.push(col_name.clone());

                        let entry = column_changes_map.entry(col_name.clone()).or_insert_with(|| {
                            ColumnChangeSummary {
                                column_name: col_name.clone(),
                                values_changed: 0,
                                nulls_added: 0,
                                nulls_removed: 0,
                                types_changed: 0,
                                change_rate: 0.0,
                            }
                        });

                        if from_val.is_empty() && !to_val.is_empty() {
                            entry.nulls_removed += 1;
                            change_type = ModificationType::NullBecameValue;
                        } else if !from_val.is_empty() && to_val.is_empty() {
                            entry.nulls_added += 1;
                            change_type = ModificationType::ValueBecameNull;
                        } else {
                            entry.values_changed += 1;
                        }
                    }
                }

                if !changed_cols.is_empty() {
                    modified_details.push(RowModification {
                        row_index: to_idx,
                        changed_columns: changed_cols,
                        change_type,
                    });
                }
            }
        }

        let modified_rows_count = modified_details.len();

        for summary in column_changes_map.values_mut() {
            let total_changes = summary.values_changed + summary.nulls_added + summary.nulls_removed;
            summary.change_rate = total_changes as f64 / to_count.max(1) as f64;
        }

        let mut column_changes: Vec<ColumnChangeSummary> = column_changes_map.into_values().collect();
        column_changes.sort_by(|a, b| b.change_rate.partial_cmp(&a.change_rate).unwrap_or(std::cmp::Ordering::Equal));

        let total_changes = added_rows_count + removed_rows_count + modified_rows_count;
        let change_rate = total_changes as f64 / from_count.max(1) as f64;

        let severity = if change_rate > 0.5 {
            RowDiffSeverity::Breaking
        } else if change_rate > 0.2 {
            RowDiffSeverity::Major
        } else if change_rate > 0.05 {
            RowDiffSeverity::Moderate
        } else {
            RowDiffSeverity::Minor
        };

        let summary = format!(
            "版本 {} → {}: 新增 {} 行, 删除 {} 行, 修改 {} 行, 不变 {} 行 (变化率 {:.1}%)",
            from_version, to_version,
            added_rows_count, removed_rows_count, modified_rows_count, unchanged_rows_count,
            change_rate * 100.0
        );

        let mut recommendations = Vec::new();
        match severity {
            RowDiffSeverity::Breaking => {
                recommendations.push("🔴 数据变化极大，建议重新进行完整的数据分析和模型训练".to_string());
            }
            RowDiffSeverity::Major => {
                recommendations.push("🟠 数据变化较大，建议重新评估模型性能".to_string());
            }
            RowDiffSeverity::Moderate => {
                recommendations.push("🟡 数据有中等程度变化，建议检查关键列的变化".to_string());
            }
            RowDiffSeverity::Minor => {
                recommendations.push("✅ 数据变化较小，可以增量更新模型".to_string());
            }
        }

        if !column_changes.is_empty() {
            let top_changed = &column_changes[0];
            if top_changed.change_rate > 0.1 {
                recommendations.push(format!(
                    "列 '{}' 变化最大（{:.1}%），请确认是否符合预期",
                    top_changed.column_name, top_changed.change_rate * 100.0
                ));
            }
        }

        RowDiffReport {
            dataset_id: dataset_id.to_string(),
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            from_rows: from_count,
            to_rows: to_count,
            added_rows: added_rows_count,
            removed_rows: removed_rows_count,
            modified_rows: modified_rows_count,
            unchanged_rows: unchanged_rows_count,
            change_rate,
            severity,
            added_indices,
            removed_indices,
            modified_details,
            column_changes,
            summary,
            recommendations,
        }
    }

    fn hash_row(row: &[String]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for val in row {
            val.hash(&mut hasher);
        }
        format!("{:x}", hasher.finish())
    }
}
