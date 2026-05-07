use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray,
};
use arrow::compute::{
    filter_record_batch, max, min,
};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::record_batch::RecordBatch;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrowTableInfo {
    pub name: String,
    pub num_rows: usize,
    pub num_columns: usize,
    pub column_names: Vec<String>,
    pub column_types: Vec<String>,
    pub memory_size_bytes: usize,
    pub is_empty: bool,
}

#[derive(Debug, Clone)]
pub struct ArrowTable {
    pub schema: SchemaRef,
    pub batches: Vec<RecordBatch>,
    pub name: String,
    pub metadata: HashMap<String, String>,
}

impl ArrowTable {
    pub fn new(name: &str, schema: SchemaRef) -> Self {
        Self {
            schema,
            batches: Vec::new(),
            name: name.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn from_batches(name: &str, schema: SchemaRef, batches: Vec<RecordBatch>) -> Self {
        Self {
            schema,
            batches,
            name: name.to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_batch(&mut self, batch: RecordBatch) -> Result<(), String> {
        if batch.schema() != self.schema {
            return Err(format!(
                "Batch schema mismatch: expected {:?}, got {:?}",
                self.schema, batch.schema()
            ));
        }
        self.batches.push(batch);
        Ok(())
    }

    pub fn num_rows(&self) -> usize {
        self.batches.iter().map(|b| b.num_rows()).sum()
    }

    pub fn num_columns(&self) -> usize {
        self.schema.fields().len()
    }

    pub fn column_names(&self) -> Vec<String> {
        self.schema.fields().iter().map(|f| f.name().clone()).collect()
    }

    pub fn column_types(&self) -> Vec<String> {
        self.schema.fields().iter()
            .map(|f| format!("{:?}", f.data_type()))
            .collect()
    }

    pub fn memory_size_bytes(&self) -> usize {
        self.batches.iter()
            .map(|b| b.get_array_memory_size())
            .sum()
    }

    pub fn info(&self) -> ArrowTableInfo {
        ArrowTableInfo {
            name: self.name.clone(),
            num_rows: self.num_rows(),
            num_columns: self.num_columns(),
            column_names: self.column_names(),
            column_types: self.column_types(),
            memory_size_bytes: self.memory_size_bytes(),
            is_empty: self.num_rows() == 0,
        }
    }

    pub fn to_single_batch(&self) -> Result<RecordBatch, String> {
        if self.batches.is_empty() {
            return Err("No batches in table".to_string());
        }
        if self.batches.len() == 1 {
            return Ok(self.batches[0].clone());
        }
        arrow::compute::concat_batches(&self.schema, &self.batches)
            .map_err(|e| format!("Failed to concat batches: {}", e))
    }

    pub fn slice(&self, offset: usize, length: usize) -> Result<ArrowTable, String> {
        let single = self.to_single_batch()?;
        let sliced = single.slice(offset, length);
        Ok(ArrowTable::from_batches(
            &format!("{}[{}..{}]", self.name, offset, offset + length),
            self.schema.clone(),
            vec![sliced],
        ))
    }

    pub fn select_columns(&self, column_names: &[&str]) -> Result<ArrowTable, String> {
        let single = self.to_single_batch()?;

        let mut fields = Vec::new();
        let mut columns: Vec<ArrayRef> = Vec::new();

        for &col_name in column_names {
            let idx = self.schema.index_of(col_name)
                .map_err(|_| format!("Column '{}' not found", col_name))?;
            fields.push(self.schema.field(idx).clone());
            columns.push(single.column(idx).clone());
        }

        let new_schema = Arc::new(Schema::new(fields));
        let new_batch = RecordBatch::try_new(new_schema.clone(), columns)
            .map_err(|e| format!("Failed to create projected batch: {}", e))?;

        Ok(ArrowTable::from_batches(
            &format!("{}[projected]", self.name),
            new_schema,
            vec![new_batch],
        ))
    }

    pub fn filter_by_mask(&self, mask: &BooleanArray) -> Result<ArrowTable, String> {
        let single = self.to_single_batch()?;
        let filtered = filter_record_batch(&single, mask)
            .map_err(|e| format!("Filter failed: {}", e))?;

        Ok(ArrowTable::from_batches(
            &format!("{}[filtered]", self.name),
            self.schema.clone(),
            vec![filtered],
        ))
    }

    pub fn get_column(&self, column_name: &str) -> Result<ArrayRef, String> {
        let single = self.to_single_batch()?;
        let idx = self.schema.index_of(column_name)
            .map_err(|_| format!("Column '{}' not found", column_name))?;
        Ok(single.column(idx).clone())
    }

    pub fn get_column_as_strings(&self, column_name: &str) -> Result<Vec<String>, String> {
        let col = self.get_column(column_name)?;
        let mut result = Vec::with_capacity(col.len());

        match col.data_type() {
            DataType::Utf8 | DataType::LargeUtf8 => {
                let arr = col.as_any().downcast_ref::<StringArray>()
                    .ok_or("Failed to cast to StringArray")?;
                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        result.push(String::new());
                    } else {
                        result.push(arr.value(i).to_string());
                    }
                }
            }
            DataType::Int64 => {
                let arr = col.as_any().downcast_ref::<Int64Array>()
                    .ok_or("Failed to cast to Int64Array")?;
                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        result.push(String::new());
                    } else {
                        result.push(arr.value(i).to_string());
                    }
                }
            }
            DataType::Float64 => {
                let arr = col.as_any().downcast_ref::<Float64Array>()
                    .ok_or("Failed to cast to Float64Array")?;
                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        result.push(String::new());
                    } else {
                        result.push(arr.value(i).to_string());
                    }
                }
            }
            DataType::Boolean => {
                let arr = col.as_any().downcast_ref::<BooleanArray>()
                    .ok_or("Failed to cast to BooleanArray")?;
                for i in 0..arr.len() {
                    if arr.is_null(i) {
                        result.push(String::new());
                    } else {
                        result.push(arr.value(i).to_string());
                    }
                }
            }
            _ => {
                for i in 0..col.len() {
                    if col.is_null(i) {
                        result.push(String::new());
                    } else {
                        result.push(format!("{:?}", col));
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn column_stats(&self, column_name: &str) -> Result<ColumnStats, String> {
        let col = self.get_column(column_name)?;
        let total = col.len();
        let null_count = col.null_count();

        let (min_val, max_val, mean_val, median_val) = match col.data_type() {
            DataType::Int64 => {
                let arr = col.as_any().downcast_ref::<Int64Array>()
                    .ok_or("Failed to cast")?;
                let min_v = min(arr).unwrap_or(0);
                let max_v = max(arr).unwrap_or(0);
                let sum_v: i64 = (0..arr.len()).filter_map(|i| {
                    if arr.is_valid(i) { Some(arr.value(i)) } else { None }
                }).sum();
                let mean = if total > null_count {
                    sum_v as f64 / (total - null_count) as f64
                } else { 0.0 };

                let mut vals: Vec<i64> = (0..arr.len())
                    .filter_map(|i| if arr.is_valid(i) { Some(arr.value(i)) } else { None })
                    .collect();
                vals.sort();
                let median = if vals.is_empty() { 0.0 } else { vals[vals.len() / 2] as f64 };

                (Some(min_v.to_string()), Some(max_v.to_string()), Some(mean), Some(median))
            }
            DataType::Float64 => {
                let arr = col.as_any().downcast_ref::<Float64Array>()
                    .ok_or("Failed to cast")?;
                let min_v = min(arr).unwrap_or(0.0);
                let max_v = max(arr).unwrap_or(0.0);
                let sum_v: f64 = (0..arr.len()).filter_map(|i| {
                    if arr.is_valid(i) { Some(arr.value(i)) } else { None }
                }).sum();
                let mean = if total > null_count {
                    sum_v / (total - null_count) as f64
                } else { 0.0 };

                let mut vals: Vec<f64> = (0..arr.len())
                    .filter_map(|i| if arr.is_valid(i) { Some(arr.value(i)) } else { None })
                    .collect();
                vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let median = if vals.is_empty() { 0.0 } else { vals[vals.len() / 2] };

                (Some(min_v.to_string()), Some(max_v.to_string()), Some(mean), Some(median))
            }
            _ => (None, None, None, None),
        };

        let distinct = Self::count_distinct(&col);

        Ok(ColumnStats {
            column_name: column_name.to_string(),
            data_type: format!("{:?}", col.data_type()),
            total_count: total,
            null_count,
            distinct_count: distinct,
            min_value: min_val,
            max_value: max_val,
            mean_value: mean_val,
            median_value: median_val,
        })
    }

    fn count_distinct(col: &dyn Array) -> usize {
        let mut seen = std::collections::HashSet::new();
        for i in 0..col.len() {
            if col.is_null(i) {
                continue;
            }
            let key = match col.data_type() {
                DataType::Utf8 | DataType::LargeUtf8 => {
                    col.as_any().downcast_ref::<StringArray>()
                        .map(|a| a.value(i).to_string())
                        .unwrap_or_default()
                }
                DataType::Int64 => {
                    col.as_any().downcast_ref::<Int64Array>()
                        .map(|a| a.value(i).to_string())
                        .unwrap_or_default()
                }
                DataType::Float64 => {
                    col.as_any().downcast_ref::<Float64Array>()
                        .map(|a| a.value(i).to_string())
                        .unwrap_or_default()
                }
                _ => format!("{:?}", col),
            };
            seen.insert(key);
        }
        seen.len()
    }

    pub fn to_json_rows(&self) -> Result<Vec<serde_json::Value>, String> {
        let single = self.to_single_batch()?;
        let mut rows = Vec::with_capacity(single.num_rows());

        for row_idx in 0..single.num_rows() {
            let mut row = serde_json::Map::new();
            for col_idx in 0..single.num_columns() {
                let col = single.column(col_idx);
                let field = self.schema.field(col_idx);
                let val = Self::value_to_json(col.as_ref(), row_idx);
                row.insert(field.name().clone(), val);
            }
            rows.push(serde_json::Value::Object(row));
        }

        Ok(rows)
    }

    pub fn value_to_json(col: &dyn Array, idx: usize) -> serde_json::Value {
        if col.is_null(idx) {
            return serde_json::Value::Null;
        }
        match col.data_type() {
            DataType::Utf8 | DataType::LargeUtf8 => {
                col.as_any().downcast_ref::<StringArray>()
                    .map(|a| serde_json::Value::String(a.value(idx).to_string()))
                    .unwrap_or(serde_json::Value::Null)
            }
            DataType::Int64 => {
                col.as_any().downcast_ref::<Int64Array>()
                    .map(|a| serde_json::json!(a.value(idx)))
                    .unwrap_or(serde_json::Value::Null)
            }
            DataType::Float64 => {
                col.as_any().downcast_ref::<Float64Array>()
                    .map(|a| serde_json::json!(a.value(idx)))
                    .unwrap_or(serde_json::Value::Null)
            }
            DataType::Boolean => {
                col.as_any().downcast_ref::<BooleanArray>()
                    .map(|a| serde_json::json!(a.value(idx)))
                    .unwrap_or(serde_json::Value::Null)
            }
            _ => serde_json::Value::String(format!("{:?}", col)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    pub column_name: String,
    pub data_type: String,
    pub total_count: usize,
    pub null_count: usize,
    pub distinct_count: usize,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub mean_value: Option<f64>,
    pub median_value: Option<f64>,
}

impl ColumnStats {
    pub fn null_rate(&self) -> f64 {
        if self.total_count == 0 { 0.0 }
        else { self.null_count as f64 / self.total_count as f64 }
    }
}

pub struct ArrowTableBuilder {
    schema: SchemaRef,
    name: String,
    string_columns: Vec<Vec<String>>,
    int_columns: Vec<Vec<i64>>,
    float_columns: Vec<Vec<f64>>,
    bool_columns: Vec<Vec<bool>>,
    column_indices: HashMap<String, (usize, ColumnKind)>,
}

#[derive(Debug, Clone, Copy)]
enum ColumnKind {
    String,
    Int,
    Float,
    Bool,
}

impl ArrowTableBuilder {
    pub fn new(name: &str, schema: SchemaRef) -> Self {
        let mut column_indices = HashMap::new();
        let mut string_columns = Vec::new();
        let mut int_columns = Vec::new();
        let mut float_columns = Vec::new();
        let mut bool_columns = Vec::new();

        for (_i, field) in schema.fields().iter().enumerate() {
            let kind = match field.data_type() {
                DataType::Utf8 | DataType::LargeUtf8 => {
                    string_columns.push(Vec::new());
                    let idx = string_columns.len() - 1;
                    column_indices.insert(field.name().clone(), (idx, ColumnKind::String));
                    ColumnKind::String
                }
                DataType::Int64 => {
                    int_columns.push(Vec::new());
                    let idx = int_columns.len() - 1;
                    column_indices.insert(field.name().clone(), (idx, ColumnKind::Int));
                    ColumnKind::Int
                }
                DataType::Float64 => {
                    float_columns.push(Vec::new());
                    let idx = float_columns.len() - 1;
                    column_indices.insert(field.name().clone(), (idx, ColumnKind::Float));
                    ColumnKind::Float
                }
                DataType::Boolean => {
                    bool_columns.push(Vec::new());
                    let idx = bool_columns.len() - 1;
                    column_indices.insert(field.name().clone(), (idx, ColumnKind::Bool));
                    ColumnKind::Bool
                }
                _ => ColumnKind::String,
            };
            let _ = kind;
        }

        Self {
            schema,
            name: name.to_string(),
            string_columns,
            int_columns,
            float_columns,
            bool_columns,
            column_indices,
        }
    }

    pub fn push_string(&mut self, column_name: &str, value: &str) {
        if let Some(&(idx, ColumnKind::String)) = self.column_indices.get(column_name) {
            if idx < self.string_columns.len() {
                self.string_columns[idx].push(value.to_string());
            }
        }
    }

    pub fn push_int(&mut self, column_name: &str, value: i64) {
        if let Some(&(idx, ColumnKind::Int)) = self.column_indices.get(column_name) {
            if idx < self.int_columns.len() {
                self.int_columns[idx].push(value);
            }
        }
    }

    pub fn push_float(&mut self, column_name: &str, value: f64) {
        if let Some(&(idx, ColumnKind::Float)) = self.column_indices.get(column_name) {
            if idx < self.float_columns.len() {
                self.float_columns[idx].push(value);
            }
        }
    }

    pub fn push_bool(&mut self, column_name: &str, value: bool) {
        if let Some(&(idx, ColumnKind::Bool)) = self.column_indices.get(column_name) {
            if idx < self.bool_columns.len() {
                self.bool_columns[idx].push(value);
            }
        }
    }

    pub fn push_null(&mut self, column_name: &str) {
        if let Some(&(idx, kind)) = self.column_indices.get(column_name) {
            match kind {
                ColumnKind::String => {
                    if idx < self.string_columns.len() {
                        self.string_columns[idx].push(String::new());
                    }
                }
                ColumnKind::Int => {
                    if idx < self.int_columns.len() {
                        self.int_columns[idx].push(0);
                    }
                }
                ColumnKind::Float => {
                    if idx < self.float_columns.len() {
                        self.float_columns[idx].push(0.0);
                    }
                }
                ColumnKind::Bool => {
                    if idx < self.bool_columns.len() {
                        self.bool_columns[idx].push(false);
                    }
                }
            }
        }
    }

    pub fn build(self) -> Result<ArrowTable, String> {
        let num_rows = self.schema.fields().iter()
            .filter_map(|f| self.column_indices.get(f.name()))
            .map(|&(idx, kind)| match kind {
                ColumnKind::String => self.string_columns.get(idx).map(|c| c.len()).unwrap_or(0),
                ColumnKind::Int => self.int_columns.get(idx).map(|c| c.len()).unwrap_or(0),
                ColumnKind::Float => self.float_columns.get(idx).map(|c| c.len()).unwrap_or(0),
                ColumnKind::Bool => self.bool_columns.get(idx).map(|c| c.len()).unwrap_or(0),
            })
            .max()
            .unwrap_or(0);

        let mut columns: Vec<ArrayRef> = Vec::new();

        for field in self.schema.fields().iter() {
            let col: ArrayRef = match self.column_indices.get(field.name()) {
                Some(&(idx, ColumnKind::String)) => {
                    let vals = self.string_columns.get(idx)
                        .cloned()
                        .unwrap_or_else(|| vec![String::new(); num_rows]);
                    Arc::new(StringArray::from(vals))
                }
                Some(&(idx, ColumnKind::Int)) => {
                    let vals = self.int_columns.get(idx)
                        .cloned()
                        .unwrap_or_else(|| vec![0i64; num_rows]);
                    Arc::new(Int64Array::from(vals))
                }
                Some(&(idx, ColumnKind::Float)) => {
                    let vals = self.float_columns.get(idx)
                        .cloned()
                        .unwrap_or_else(|| vec![0.0f64; num_rows]);
                    Arc::new(Float64Array::from(vals))
                }
                Some(&(idx, ColumnKind::Bool)) => {
                    let vals = self.bool_columns.get(idx)
                        .cloned()
                        .unwrap_or_else(|| vec![false; num_rows]);
                    Arc::new(BooleanArray::from(vals))
                }
                None => {
                    Arc::new(StringArray::from(vec![""; num_rows]))
                }
            };
            columns.push(col);
        }

        let batch = RecordBatch::try_new(self.schema.clone(), columns)
            .map_err(|e| format!("Failed to create RecordBatch: {}", e))?;

        Ok(ArrowTable::from_batches(&self.name, self.schema, vec![batch]))
    }
}

pub fn infer_arrow_schema(
    column_names: &[String],
    sample_values: &[Vec<String>],
) -> SchemaRef {
    let mut fields = Vec::new();

    for (col_idx, name) in column_names.iter().enumerate() {
        let data_type = if col_idx < sample_values.len() && !sample_values[col_idx].is_empty() {
            infer_column_type(&sample_values[col_idx])
        } else {
            DataType::Utf8
        };
        fields.push(Field::new(name.as_str(), data_type, true));
    }

    Arc::new(Schema::new(fields))
}

fn infer_column_type(values: &[String]) -> DataType {
    let non_empty: Vec<&String> = values.iter().filter(|v| !v.is_empty()).collect();
    if non_empty.is_empty() {
        return DataType::Utf8;
    }

    let all_int = non_empty.iter().all(|v| v.parse::<i64>().is_ok());
    if all_int {
        return DataType::Int64;
    }

    let all_float = non_empty.iter().all(|v| v.parse::<f64>().is_ok());
    if all_float {
        return DataType::Float64;
    }

    let all_bool = non_empty.iter().all(|v| {
        matches!(v.to_lowercase().as_str(), "true" | "false" | "1" | "0")
    });
    if all_bool {
        return DataType::Boolean;
    }

    DataType::Utf8
}

pub fn csv_to_arrow_table(
    name: &str,
    csv_path: &str,
    has_header: bool,
) -> Result<ArrowTable, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(has_header)
        .from_path(csv_path)
        .map_err(|e| format!("Failed to open CSV: {}", e))?;

    let headers: Vec<String> = if has_header {
        reader.headers()
            .map_err(|e| format!("Failed to read headers: {}", e))?
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        let first = reader.records().next()
            .ok_or("Empty CSV file")?
            .map_err(|e| format!("Failed to read first row: {}", e))?;
        (0..first.len()).map(|i| format!("column_{}", i)).collect()
    };

    let mut sample_values: Vec<Vec<String>> = vec![Vec::new(); headers.len()];
    let mut all_rows: Vec<Vec<String>> = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| format!("CSV read error: {}", e))?;
        let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
        for (i, val) in row.iter().enumerate() {
            if i < sample_values.len() {
                sample_values[i].push(val.clone());
            }
        }
        all_rows.push(row);
    }

    let schema = infer_arrow_schema(&headers, &sample_values);
    let mut builder = ArrowTableBuilder::new(name, schema.clone());

    for row in &all_rows {
        for (col_idx, field) in schema.fields().iter().enumerate() {
            let val = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
            if val.is_empty() {
                builder.push_null(field.name());
                continue;
            }
            match field.data_type() {
                DataType::Int64 => {
                    if let Ok(v) = val.parse::<i64>() {
                        builder.push_int(field.name(), v);
                    } else {
                        builder.push_null(field.name());
                    }
                }
                DataType::Float64 => {
                    if let Ok(v) = val.parse::<f64>() {
                        builder.push_float(field.name(), v);
                    } else {
                        builder.push_null(field.name());
                    }
                }
                DataType::Boolean => {
                    match val.to_lowercase().as_str() {
                        "true" | "1" => builder.push_bool(field.name(), true),
                        "false" | "0" => builder.push_bool(field.name(), false),
                        _ => builder.push_null(field.name()),
                    }
                }
                _ => {
                    builder.push_string(field.name(), val);
                }
            }
        }
    }

    builder.build()
}
