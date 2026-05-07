use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::{LabError, Result};
use crate::data::data_trait::PreprocessType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub step_type: PreprocessType,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPipeline {
    pub steps: Vec<PipelineStep>,
}

impl DataPipeline {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(mut self, step: PipelineStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn normalize(self) -> Self {
        self.add_step(PipelineStep {
            step_type: PreprocessType::Normalize,
            params: serde_json::json!({}),
        })
    }

    pub fn standardize(self) -> Self {
        self.add_step(PipelineStep {
            step_type: PreprocessType::Standardize,
            params: serde_json::json!({}),
        })
    }

    pub fn fill_missing(self, strategy: &str) -> Self {
        self.add_step(PipelineStep {
            step_type: PreprocessType::FillMissing,
            params: serde_json::json!({ "strategy": strategy }),
        })
    }

    pub fn drop_missing(self) -> Self {
        self.add_step(PipelineStep {
            step_type: PreprocessType::DropMissing,
            params: serde_json::json!({}),
        })
    }

    pub fn execute(&self, data: &mut Vec<Vec<f32>>) -> Result<()> {
        for step in &self.steps {
            match step.step_type {
                PreprocessType::Normalize => self.normalize_data(data),
                PreprocessType::Standardize => self.standardize_data(data),
                PreprocessType::FillMissing => {
                    let strategy = step.params.get("strategy")
                        .and_then(|v| v.as_str())
                        .unwrap_or("mean");
                    self.fill_missing_data(data, strategy)?;
                }
                PreprocessType::DropMissing => self.drop_missing_data(data),
                PreprocessType::PadSequence => {
                    let max_len = step.params.get("max_length")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(128) as usize;
                    let pad_value = step.params.get("pad_value")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as f32;
                    self.pad_sequence_data(data, max_len, pad_value);
                }
                PreprocessType::Tokenize | PreprocessType::AugmentImage | PreprocessType::ResizeImage => {
                    return Err(LabError::InvalidConfig(format!(
                        "Preprocessing {:?} requires string-based data, use execute_on_records instead",
                        step.step_type
                    )));
                }
                PreprocessType::Custom(ref name) => {
                    return Err(LabError::InvalidConfig(format!(
                        "Custom preprocessing '{}' is not supported in numeric pipeline, use execute_on_records",
                        name
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn execute_on_records(
        &self,
        columns: &mut Vec<String>,
        records: &mut Vec<Vec<String>>,
    ) -> Result<()> {
        for step in &self.steps {
            match step.step_type {
                PreprocessType::Normalize => self.normalize_records(records)?,
                PreprocessType::Standardize => self.standardize_records(records)?,
                PreprocessType::FillMissing => {
                    let strategy = step.params.get("strategy")
                        .and_then(|v| v.as_str())
                        .unwrap_or("mean");
                    self.fill_missing_records(records, strategy)?;
                }
                PreprocessType::DropMissing => self.drop_missing_records(records),
                PreprocessType::OneHotEncode => {
                    let target_cols: Vec<String> = step.params.get("columns")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    self.one_hot_encode(columns, records, &target_cols)?;
                }
                PreprocessType::LabelEncode => {
                    let target_cols: Vec<String> = step.params.get("columns")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    self.label_encode(columns, records, &target_cols)?;
                }
                PreprocessType::Tokenize => {
                    let target_cols: Vec<String> = step.params.get("columns")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    self.tokenize_records(columns, records, &target_cols)?;
                }
                PreprocessType::PadSequence => {
                    let max_len = step.params.get("max_length")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(128) as usize;
                    let pad_token = step.params.get("pad_token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("<PAD>")
                        .to_string();
                    self.pad_sequence_records(records, max_len, &pad_token)?;
                }
                PreprocessType::AugmentImage => {
                    return Err(LabError::InvalidConfig(
                        "Image augmentation requires image data and is not supported in tabular pipeline".to_string()
                    ));
                }
                PreprocessType::ResizeImage => {
                    return Err(LabError::InvalidConfig(
                        "Image resizing requires image data and is not supported in tabular pipeline".to_string()
                    ));
                }
                PreprocessType::Custom(ref name) => {
                    return Err(LabError::InvalidConfig(format!(
                        "Custom preprocessing '{}' is not implemented. Register a custom preprocessor.", name
                    )));
                }
            }
        }
        Ok(())
    }

    fn normalize_data(&self, data: &mut Vec<Vec<f32>>) {
        if data.is_empty() {
            return;
        }
        let num_cols = data[0].len();
        if !data.iter().all(|row| row.len() == num_cols) {
            return;
        }
        for col in 0..num_cols {
            let col_values: Vec<f32> = data.iter().map(|row| row[col]).filter(|v| !v.is_nan()).collect();
            if col_values.is_empty() {
                continue;
            }
            let min = col_values.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = col_values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let range = max - min;
            if range.abs() < f32::EPSILON {
                for row in data.iter_mut() {
                    row[col] = 0.0;
                }
            } else {
                for row in data.iter_mut() {
                    if !row[col].is_nan() {
                        row[col] = (row[col] - min) / range;
                    }
                }
            }
        }
    }

    fn standardize_data(&self, data: &mut Vec<Vec<f32>>) {
        if data.is_empty() {
            return;
        }
        let num_cols = data[0].len();
        if !data.iter().all(|row| row.len() == num_cols) {
            return;
        }
        for col in 0..num_cols {
            let col_values: Vec<f32> = data.iter().map(|row| row[col]).filter(|v| !v.is_nan()).collect();
            if col_values.is_empty() {
                continue;
            }
            let mean: f32 = col_values.iter().sum::<f32>() / col_values.len() as f32;
            let variance: f32 = col_values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / col_values.len() as f32;
            let std = variance.sqrt();
            if std < f32::EPSILON {
                for row in data.iter_mut() {
                    row[col] = 0.0;
                }
            } else {
                for row in data.iter_mut() {
                    if !row[col].is_nan() {
                        row[col] = (row[col] - mean) / std;
                    }
                }
            }
        }
    }

    fn fill_missing_data(&self, data: &mut Vec<Vec<f32>>, strategy: &str) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        let num_cols = data[0].len();
        if !data.iter().all(|row| row.len() == num_cols) {
            return Ok(());
        }
        for col in 0..num_cols {
            let valid_values: Vec<f32> = data.iter().map(|row| row[col]).filter(|v| !v.is_nan()).collect();
            let fill_value = match strategy {
                "mean" => {
                    if valid_values.is_empty() {
                        0.0
                    } else {
                        valid_values.iter().sum::<f32>() / valid_values.len() as f32
                    }
                }
                "median" => {
                    if valid_values.is_empty() {
                        0.0
                    } else {
                        let mut sorted = valid_values.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        let mid = sorted.len() / 2;
                        if sorted.len() % 2 == 0 {
                            (sorted[mid - 1] + sorted[mid]) / 2.0
                        } else {
                            sorted[mid]
                        }
                    }
                }
                "zero" => 0.0,
                "min" => {
                    if valid_values.is_empty() {
                        0.0
                    } else {
                        valid_values.iter().cloned().fold(f32::INFINITY, f32::min)
                    }
                },
                "max" => {
                    if valid_values.is_empty() {
                        0.0
                    } else {
                        valid_values.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
                    }
                }
                _ => {
                    return Err(LabError::InvalidConfig(format!(
                        "Unknown fill_missing strategy: {}", strategy
                    )));
                }
            };
            for row in data.iter_mut() {
                if row[col].is_nan() {
                    row[col] = fill_value;
                }
            }
        }
        Ok(())
    }

    fn drop_missing_data(&self, data: &mut Vec<Vec<f32>>) {
        data.retain(|row| row.iter().all(|v| !v.is_nan()));
    }

    fn normalize_records(&self, records: &mut Vec<Vec<String>>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let num_cols = records[0].len();
        for col in 0..num_cols {
            let col_values: Vec<f64> = records.iter()
                .filter_map(|row| row.get(col).and_then(|v| v.parse::<f64>().ok()))
                .collect();
            if col_values.is_empty() {
                continue;
            }
            let min = col_values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = col_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let range = max - min;
            for row in records.iter_mut() {
                if let Some(val) = row.get(col).and_then(|v| v.parse::<f64>().ok()) {
                    if range.abs() < f64::EPSILON {
                        row[col] = "0.0".to_string();
                    } else {
                        row[col] = ((val - min) / range).to_string();
                    }
                }
            }
        }
        Ok(())
    }

    fn standardize_records(&self, records: &mut Vec<Vec<String>>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let num_cols = records[0].len();
        for col in 0..num_cols {
            let col_values: Vec<f64> = records.iter()
                .filter_map(|row| row.get(col).and_then(|v| v.parse::<f64>().ok()))
                .collect();
            if col_values.is_empty() {
                continue;
            }
            let mean = col_values.iter().sum::<f64>() / col_values.len() as f64;
            let variance = col_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / col_values.len() as f64;
            let std = variance.sqrt();
            for row in records.iter_mut() {
                if let Some(val) = row.get(col).and_then(|v| v.parse::<f64>().ok()) {
                    if std < f64::EPSILON {
                        row[col] = "0.0".to_string();
                    } else {
                        row[col] = ((val - mean) / std).to_string();
                    }
                }
            }
        }
        Ok(())
    }

    fn fill_missing_records(&self, records: &mut Vec<Vec<String>>, strategy: &str) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let num_cols = records[0].len();
        for col in 0..num_cols {
            let is_numeric = records.iter().any(|row| {
                row.get(col).map(|v| !v.trim().is_empty() && v.parse::<f64>().is_ok()).unwrap_or(false)
            });

            if is_numeric {
                let valid_values: Vec<f64> = records.iter()
                    .filter_map(|row| row.get(col).and_then(|v| {
                        let trimmed = v.trim();
                        if trimmed.is_empty() { None } else { trimmed.parse::<f64>().ok() }
                    }))
                    .collect();
                let fill_value = match strategy {
                    "mean" => {
                        if valid_values.is_empty() { 0.0 }
                        else { valid_values.iter().sum::<f64>() / valid_values.len() as f64 }
                    }
                    "median" => {
                        if valid_values.is_empty() { 0.0 }
                        else {
                            let mut sorted = valid_values.clone();
                            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                            let mid = sorted.len() / 2;
                            if sorted.len() % 2 == 0 { (sorted[mid - 1] + sorted[mid]) / 2.0 }
                            else { sorted[mid] }
                        }
                    }
                    "zero" => 0.0,
                    "min" => {
                        if valid_values.is_empty() { 0.0 }
                        else { valid_values.iter().cloned().fold(f64::INFINITY, f64::min) }
                    }
                    "max" => {
                        if valid_values.is_empty() { 0.0 }
                        else { valid_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) }
                    }
                    _ => return Err(LabError::InvalidConfig(format!("Unknown fill_missing strategy: {}", strategy))),
                };
                for row in records.iter_mut() {
                    if let Some(v) = row.get(col) {
                        if v.trim().is_empty() {
                            row[col] = fill_value.to_string();
                        }
                    }
                }
            } else {
                let mode = {
                    let mut counts: HashMap<String, usize> = HashMap::new();
                    for row in records.iter() {
                        if let Some(v) = row.get(col) {
                            let trimmed = v.trim();
                            if !trimmed.is_empty() {
                                *counts.entry(trimmed.to_string()).or_insert(0) += 1;
                            }
                        }
                    }
                    counts.into_iter().max_by_key(|(_, c)| *c).map(|(v, _)| v).unwrap_or_default()
                };
                for row in records.iter_mut() {
                    if let Some(v) = row.get(col) {
                        if v.trim().is_empty() {
                            row[col] = mode.clone();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn drop_missing_records(&self, records: &mut Vec<Vec<String>>) {
        records.retain(|row| row.iter().all(|v| !v.trim().is_empty()));
    }

    fn one_hot_encode(
        &self,
        columns: &mut Vec<String>,
        records: &mut Vec<Vec<String>>,
        target_cols: &[String],
    ) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let col_indices: Vec<usize> = if target_cols.is_empty() {
            columns.iter().enumerate()
                .filter(|(col_idx, _)| {
                    let is_numeric = records.iter().any(|row| {
                        row.get(*col_idx).map(|v| v.parse::<f64>().is_ok()).unwrap_or(false)
                    });
                    !is_numeric
                })
                .map(|(i, _)| i)
                .collect()
        } else {
            target_cols.iter()
                .filter_map(|name| columns.iter().position(|c| c == name))
                .collect()
        };

        if col_indices.is_empty() {
            return Ok(());
        }

        let mut new_columns = Vec::new();
        let mut col_mapping: Vec<(usize, Vec<String>, Vec<String>)> = Vec::new();

        for (_, &col_idx) in col_indices.iter().enumerate() {
            let col_name = &columns[col_idx];
            let unique_values: Vec<String> = records.iter()
                .filter_map(|row| row.get(col_idx).map(|v| v.trim().to_string()))
                .filter(|v| !v.is_empty())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();

            let mut sorted_values = unique_values;
            sorted_values.sort();

            if sorted_values.len() > 100 {
                return Err(LabError::InvalidConfig(format!(
                    "Column '{}' has {} unique values, which exceeds the one-hot encoding limit of 100. Use label_encode instead.",
                    col_name, sorted_values.len()
                )));
            }

            let one_hot_names: Vec<String> = sorted_values.iter()
                .map(|v| format!("{}_{}", col_name, v))
                .collect();

            col_mapping.push((col_idx, sorted_values.clone(), one_hot_names.clone()));
            new_columns.extend(one_hot_names);
        }

        let mut result_columns: Vec<String> = Vec::new();
        for (i, col_name) in columns.iter().enumerate() {
            if col_indices.contains(&i) {
                if let Some(mapping) = col_mapping.iter().find(|(idx, _, _)| *idx == i) {
                    result_columns.extend(mapping.2.clone());
                }
            } else {
                result_columns.push(col_name.clone());
            }
        }
        *columns = result_columns;

        for row in records.iter_mut() {
            let mut new_row = Vec::new();
            for (i, _) in row.iter().enumerate() {
                if let Some(mapping) = col_mapping.iter().find(|(idx, _, _)| *idx == i) {
                    let value = row.get(i).map(|v| v.trim().to_string()).unwrap_or_default();
                    for cat_value in &mapping.1 {
                        new_row.push(if value == *cat_value { "1".to_string() } else { "0".to_string() });
                    }
                } else {
                    new_row.push(row.get(i).cloned().unwrap_or_default());
                }
            }
            *row = new_row;
        }

        Ok(())
    }

    fn label_encode(
        &self,
        columns: &mut Vec<String>,
        records: &mut Vec<Vec<String>>,
        target_cols: &[String],
    ) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let col_indices: Vec<usize> = if target_cols.is_empty() {
            columns.iter().enumerate()
                .filter(|(col_idx, _)| {
                    let is_numeric = records.iter().any(|row| {
                        row.get(*col_idx).map(|v| v.parse::<f64>().is_ok()).unwrap_or(false)
                    });
                    !is_numeric
                })
                .map(|(i, _)| i)
                .collect()
        } else {
            target_cols.iter()
                .filter_map(|name| columns.iter().position(|c| c == name))
                .collect()
        };

        for &col_idx in &col_indices {
            let unique_values: Vec<String> = records.iter()
                .filter_map(|row| row.get(col_idx).map(|v| v.trim().to_string()))
                .filter(|v| !v.is_empty())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();

            let mut sorted_values = unique_values;
            sorted_values.sort();

            let label_map: HashMap<String, usize> = sorted_values.iter()
                .enumerate()
                .map(|(i, v)| (v.clone(), i))
                .collect();

            for row in records.iter_mut() {
                if let Some(v) = row.get(col_idx) {
                    let trimmed = v.trim().to_string();
                    if let Some(&label) = label_map.get(&trimmed) {
                        row[col_idx] = label.to_string();
                    }
                }
            }
        }

        Ok(())
    }

    fn pad_sequence_data(&self, data: &mut Vec<Vec<f32>>, max_len: usize, pad_value: f32) {
        for row in data.iter_mut() {
            if row.len() > max_len {
                row.truncate(max_len);
            } else {
                row.resize(max_len, pad_value);
            }
        }
    }

    fn tokenize_records(
        &self,
        columns: &mut Vec<String>,
        records: &mut Vec<Vec<String>>,
        target_cols: &[String],
    ) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let col_indices: Vec<usize> = if target_cols.is_empty() {
            columns.iter().enumerate()
                .filter(|(col_idx, _)| {
                    let is_numeric = records.iter().any(|row| {
                        row.get(*col_idx).map(|v| v.parse::<f64>().is_ok()).unwrap_or(false)
                    });
                    !is_numeric
                })
                .map(|(i, _)| i)
                .collect()
        } else {
            target_cols.iter()
                .filter_map(|name| columns.iter().position(|c| c == name))
                .collect()
        };

        for &col_idx in &col_indices {
            let col_name = &columns[col_idx];
            let mut vocab: HashMap<String, usize> = HashMap::new();
            vocab.insert("<UNK>".to_string(), 0);
            vocab.insert("<PAD>".to_string(), 1);

            for row in records.iter() {
                if let Some(v) = row.get(col_idx) {
                    let tokens = self.simple_tokenize(v);
                    for token in tokens {
                        if !vocab.contains_key(&token) {
                            let idx = vocab.len();
                            vocab.insert(token, idx);
                        }
                    }
                }
            }

            let new_col_name = format!("{}_tokenized", col_name);
            columns[col_idx] = new_col_name;

            for row in records.iter_mut() {
                if let Some(v) = row.get(col_idx) {
                    let tokens = self.simple_tokenize(v);
                    let token_ids: Vec<String> = tokens.iter()
                        .map(|t| vocab.get(t).unwrap_or(&0).to_string())
                        .collect();
                    row[col_idx] = token_ids.join(",");
                }
            }
        }

        Ok(())
    }

    fn simple_tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|word| {
                let cleaned: String = word.chars()
                    .filter(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                cleaned
            })
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn pad_sequence_records(
        &self,
        records: &mut Vec<Vec<String>>,
        max_len: usize,
        pad_token: &str,
    ) -> Result<()> {
        for row in records.iter_mut() {
            for col_idx in 0..row.len() {
                let value = &row[col_idx];
                if value.contains(',') {
                    let mut tokens: Vec<String> = value.split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                    if tokens.len() > max_len {
                        tokens.truncate(max_len);
                    } else {
                        tokens.resize(max_len, pad_token.to_string());
                    }
                    row[col_idx] = tokens.join(",");
                }
            }
        }
        Ok(())
    }
}

impl Default for DataPipeline {
    fn default() -> Self {
        Self::new()
    }
}
