use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{LabError, Result};
use crate::types::DataFormat;
use super::accessor::{
    DataAccessor, DataPage, DataSampleConfig, DataStatistics, DataValidationResult,
    SampleStrategy, ValueDistribution,
};

pub struct CsvDataAccessor {
    delimiter: u8,
    has_header: bool,
}

impl CsvDataAccessor {
    pub fn new() -> Self {
        Self {
            delimiter: b',',
            has_header: true,
        }
    }

    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    pub fn with_header(mut self, has_header: bool) -> Self {
        self.has_header = has_header;
        self
    }

    fn read_all_records(&self, path: &str) -> Result<(Vec<String>, Vec<csv::StringRecord>)> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file {}: {}", path, e)))?;

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.has_header)
            .from_reader(content.as_bytes());

        let headers = reader.headers()
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot parse CSV headers: {}", e)))?
            .clone();

        let column_names: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
        let records: Vec<csv::StringRecord> = reader.records()
            .filter_map(|r| r.ok())
            .collect();

        Ok((column_names, records))
    }

    fn infer_column_type(values: &[&str]) -> String {
        let mut int_count = 0usize;
        let mut float_count = 0usize;
        let mut bool_count = 0usize;
        let mut null_count = 0usize;
        let mut total = 0usize;

        for v in values {
            total += 1;
            let trimmed = v.trim();
            if trimmed.is_empty() {
                null_count += 1;
            } else if trimmed == "true" || trimmed == "false" || trimmed == "TRUE" || trimmed == "FALSE" {
                bool_count += 1;
            } else if trimmed.parse::<i64>().is_ok() {
                int_count += 1;
            } else if trimmed.parse::<f64>().is_ok() {
                float_count += 1;
            }
        }

        let non_null = total - null_count;
        if non_null == 0 {
            return "unknown".to_string();
        }
        if int_count as f64 / non_null as f64 > 0.8 {
            return "integer".to_string();
        }
        if (int_count + float_count) as f64 / non_null as f64 > 0.8 {
            return "float".to_string();
        }
        if bool_count as f64 / non_null as f64 > 0.8 {
            return "boolean".to_string();
        }
        let distinct: std::collections::HashSet<&str> = values.iter().copied().filter(|v| !v.is_empty()).collect();
        if distinct.len() <= 20 && distinct.len() < non_null / 2 {
            return "categorical".to_string();
        }
        "string".to_string()
    }

    fn record_to_json_values(record: &csv::StringRecord) -> Vec<Value> {
        record.iter().map(|field| {
            let trimmed = field.trim();
            if trimmed.is_empty() {
                Value::Null
            } else if let Ok(n) = trimmed.parse::<i64>() {
                Value::from(n)
            } else if let Ok(f) = trimmed.parse::<f64>() {
                Value::from(f)
            } else if trimmed == "true" || trimmed == "TRUE" {
                Value::from(true)
            } else if trimmed == "false" || trimmed == "FALSE" {
                Value::from(false)
            } else {
                Value::from(trimmed)
            }
        }).collect()
    }

    fn infer_column_types(column_names: &[String], records: &[csv::StringRecord]) -> Vec<String> {
        let sample_size = 100.min(records.len());
        (0..column_names.len()).map(|col_idx| {
            let values: Vec<&str> = records.iter()
                .take(sample_size)
                .filter_map(|r| r.get(col_idx))
                .collect();
            Self::infer_column_type(&values)
        }).collect()
    }
}

impl Default for CsvDataAccessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataAccessor for CsvDataAccessor {
    fn format(&self) -> DataFormat {
        DataFormat::Csv
    }

    async fn validate(&self, path: &str, expected_digest: Option<&str>, expected_rows: Option<usize>, expected_columns: Option<usize>, expected_column_names: Option<&[String]>) -> Result<DataValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let file_exists = std::path::Path::new(path).exists();
        let mut file_readable = false;
        let mut file_size_bytes = 0u64;
        let mut actual_digest: Option<String> = None;
        let mut actual_rows: Option<usize> = None;
        let mut actual_columns: Option<usize> = None;
        let mut digest_matches: Option<bool> = None;
        let mut row_count_matches: Option<bool> = None;
        let mut column_count_matches: Option<bool> = None;
        let mut schema_matches: Option<bool> = None;
        let mut missing_columns = Vec::new();
        let mut extra_columns = Vec::new();
        let type_mismatches: Vec<(String, String, String)> = Vec::new();

        if !file_exists {
            errors.push(format!("File does not exist: {}", path));
        } else {
            let metadata = std::fs::metadata(path)
                .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file metadata: {}", e)))?;
            file_size_bytes = metadata.len();

            let content = std::fs::read(path)
                .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file: {}", e)))?;
            file_readable = true;

            let digest = crate::domain::dataset::aggregate::Dataset::compute_digest(&content);
            actual_digest = Some(digest.clone());

            if let Some(expected) = expected_digest {
                let matches = digest == expected;
                digest_matches = Some(matches);
                if !matches {
                    errors.push(format!("Digest mismatch: expected {}, got {}", expected, digest));
                }
            }

            match self.read_all_records(path) {
                Ok((col_names, records)) => {
                    actual_rows = Some(records.len());
                    actual_columns = Some(col_names.len());

                    if let Some(expected) = expected_rows {
                        let matches = records.len() == expected;
                        row_count_matches = Some(matches);
                        if !matches {
                            warnings.push(format!("Row count mismatch: expected {}, got {}", expected, records.len()));
                        }
                    }

                    if let Some(expected) = expected_columns {
                        let matches = col_names.len() == expected;
                        column_count_matches = Some(matches);
                        if !matches {
                            warnings.push(format!("Column count mismatch: expected {}, got {}", expected, col_names.len()));
                        }
                    }

                    if let Some(expected_names) = expected_column_names {
                        let expected_set: std::collections::HashSet<&str> = expected_names.iter().map(|s| s.as_str()).collect();
                        let actual_set: std::collections::HashSet<&str> = col_names.iter().map(|s| s.as_str()).collect();

                        for name in &expected_set {
                            if !actual_set.contains(name) {
                                missing_columns.push(name.to_string());
                            }
                        }
                        for name in &actual_set {
                            if !expected_set.contains(name) {
                                extra_columns.push(name.to_string());
                            }
                        }

                        schema_matches = Some(missing_columns.is_empty() && type_mismatches.is_empty());
                        if !missing_columns.is_empty() {
                            errors.push(format!("Missing columns: {}", missing_columns.join(", ")));
                        }
                        if !extra_columns.is_empty() {
                            warnings.push(format!("Extra columns: {}", extra_columns.join(", ")));
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Cannot parse CSV: {}", e));
                }
            }
        }

        let is_valid = errors.is_empty();

        Ok(DataValidationResult {
            path: path.to_string(),
            format: DataFormat::Csv,
            file_exists,
            file_readable,
            file_size_bytes,
            digest_matches,
            expected_digest: expected_digest.map(|s| s.to_string()),
            actual_digest,
            row_count_matches,
            expected_rows,
            actual_rows,
            column_count_matches,
            expected_columns,
            actual_columns,
            schema_matches,
            missing_columns,
            extra_columns,
            type_mismatches,
            is_valid,
            errors,
            warnings,
        })
    }

    async fn page(&self, path: &str, offset: usize, limit: usize) -> Result<DataPage> {
        let (column_names, records) = self.read_all_records(path)?;

        let total_rows = records.len();
        let column_types = Self::infer_column_types(&column_names, &records);

        let rows: Vec<Vec<Value>> = records.iter()
            .skip(offset)
            .take(limit)
            .map(|r| Self::record_to_json_values(r))
            .collect();

        Ok(DataPage {
            columns: column_names,
            column_types,
            rows,
            total_rows,
            offset,
            limit,
        })
    }

    async fn sample(&self, path: &str, config: &DataSampleConfig) -> Result<DataPage> {
        let (column_names, records) = self.read_all_records(path)?;
        let total_rows = records.len();
        let n = config.n.min(total_rows);

        let selected_indices: Vec<usize> = match config.strategy {
            SampleStrategy::First => (0..n).collect(),
            SampleStrategy::Random => {
                let seed = config.seed.unwrap_or(42);
                let mut indices: Vec<usize> = (0..total_rows).collect();
                let mut state = seed;
                for i in (1..indices.len()).rev() {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let j = (state >> 33) as usize % (i + 1);
                    indices.swap(i, j);
                }
                indices.truncate(n);
                indices.sort();
                indices
            }
            SampleStrategy::Stratified => {
                (0..n).collect()
            }
        };

        let selected: Vec<&csv::StringRecord> = selected_indices.iter()
            .filter_map(|&i| records.get(i))
            .collect();

        let column_types = Self::infer_column_types(&column_names, &records);

        let rows: Vec<Vec<Value>> = selected.iter()
            .map(|r| Self::record_to_json_values(r))
            .collect();

        Ok(DataPage {
            columns: column_names,
            column_types,
            rows,
            total_rows,
            offset: 0,
            limit: n,
        })
    }

    async fn statistics(&self, path: &str, column_name: &str) -> Result<DataStatistics> {
        let (column_names, records) = self.read_all_records(path)?;

        let col_idx = column_names.iter().position(|n| n == column_name)
            .ok_or_else(|| LabError::Custom(format!("Column '{}' not found", column_name)))?;

        let raw_values: Vec<Option<&str>> = records.iter()
            .map(|r| r.get(col_idx).map(|s| s.trim()))
            .collect();

        let total_count = raw_values.len();
        let null_count = raw_values.iter().filter(|v| v.is_none() || v.map(|s| s.is_empty()).unwrap_or(true)).count();
        let non_null: Vec<&str> = raw_values.iter()
            .filter_map(|v| *v)
            .filter(|v| !v.is_empty())
            .collect();

        let distinct_count = non_null.iter().collect::<std::collections::HashSet<_>>().len();

        let mut value_counts: HashMap<String, usize> = HashMap::new();
        for v in &non_null {
            *value_counts.entry(v.to_string()).or_insert(0) += 1;
        }
        let mut top_values: Vec<(String, usize)> = value_counts.into_iter().collect();
        top_values.sort_by(|a, b| b.1.cmp(&a.1));
        top_values.truncate(10);

        let column_type = {
            let sample: Vec<&str> = non_null.iter().take(100).copied().collect();
            Self::infer_column_type(&sample)
        };

        let mut min_value = None;
        let mut max_value = None;
        let mut mean_value = None;
        let mut std_value = None;
        let mut median_value = None;
        let mut q25_value = None;
        let mut q75_value = None;
        let mut value_distribution = None;

        if column_type == "integer" || column_type == "float" {
            let nums: Vec<f64> = non_null.iter()
                .filter_map(|v| v.parse::<f64>().ok())
                .collect();

            if !nums.is_empty() {
                let mut sorted = nums.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                min_value = Some(serde_json::json!(sorted[0]));
                max_value = Some(serde_json::json!(sorted[sorted.len() - 1]));
                mean_value = Some(nums.iter().sum::<f64>() / nums.len() as f64);

                let mean = mean_value.unwrap();
                let variance = nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / nums.len() as f64;
                std_value = Some(variance.sqrt());

                median_value = Some(if sorted.len() % 2 == 0 {
                    (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
                } else {
                    sorted[sorted.len() / 2]
                });

                q25_value = Some(sorted[sorted.len() / 4]);
                q75_value = Some(sorted[sorted.len() * 3 / 4]);

                let num_bins = 20usize;
                let min_val = sorted[0];
                let max_val = sorted[sorted.len() - 1];
                let range = max_val - min_val;
                if range > 0.0 {
                    let bin_width = range / num_bins as f64;
                    let mut bins = Vec::with_capacity(num_bins);
                    let mut counts = vec![0usize; num_bins];
                    for i in 0..num_bins {
                        bins.push(min_val + bin_width * (i as f64 + 0.5));
                    }
                    for &v in &sorted {
                        let bin_idx = ((v - min_val) / bin_width).floor() as usize;
                        let bin_idx = bin_idx.min(num_bins - 1);
                        counts[bin_idx] += 1;
                    }
                    value_distribution = Some(ValueDistribution {
                        bins,
                        counts,
                        is_categorical: false,
                        category_counts: Vec::new(),
                    });
                }
            }
        } else {
            let mut cat_counts: Vec<(String, usize)> = top_values.clone();
            cat_counts.sort_by(|a, b| b.1.cmp(&a.1));
            cat_counts.truncate(20);

            if !non_null.is_empty() {
                min_value = Some(serde_json::json!(cat_counts.first().map(|(k, _)| k.as_str()).unwrap_or("")));
                max_value = Some(serde_json::json!(cat_counts.last().map(|(k, _)| k.as_str()).unwrap_or("")));
            }

            value_distribution = Some(ValueDistribution {
                bins: Vec::new(),
                counts: Vec::new(),
                is_categorical: true,
                category_counts: cat_counts,
            });
        }

        Ok(DataStatistics {
            column_name: column_name.to_string(),
            column_type,
            total_count,
            null_count,
            distinct_count,
            min_value,
            max_value,
            mean_value,
            std_value,
            median_value,
            q25_value,
            q75_value,
            top_values,
            value_distribution,
        })
    }

    async fn row_count(&self, path: &str) -> Result<usize> {
        let (_, records) = self.read_all_records(path)?;
        Ok(records.len())
    }

    async fn compute_digest(&self, path: &str) -> Result<String> {
        let content = std::fs::read(path)
            .map_err(|e| LabError::DataLoadFailed(format!("Cannot read file: {}", e)))?;
        Ok(crate::domain::dataset::aggregate::Dataset::compute_digest(&content))
    }

    async fn read_rows_by_indices(&self, path: &str, indices: &[usize]) -> Result<DataPage> {
        let (column_names, records) = self.read_all_records(path)?;
        let total_rows = records.len();

        let sorted_indices = {
            let mut s = indices.to_vec();
            s.sort();
            s
        };

        let selected: Vec<&csv::StringRecord> = sorted_indices.iter()
            .filter(|&&i| i < total_rows)
            .filter_map(|&i| records.get(i))
            .collect();

        let column_types = Self::infer_column_types(&column_names, &records);

        let rows: Vec<Vec<Value>> = selected.iter()
            .map(|r| Self::record_to_json_values(r))
            .collect();

        Ok(DataPage {
            columns: column_names,
            column_types,
            rows,
            total_rows,
            offset: 0,
            limit: indices.len(),
        })
    }
}
