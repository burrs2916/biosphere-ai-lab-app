use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use rand::SeedableRng;
use arrow::array::{Array, BooleanArray, Float64Array, Int64Array, StringArray};
use arrow::compute::filter_record_batch;
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;

use crate::data::arrow_table::ArrowTable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub operations: Vec<QueryOperation>,
    pub estimated_output_rows: Option<usize>,
    pub cost_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryOperation {
    Filter {
        column: String,
        operator: FilterOperator,
        value: serde_json::Value,
    },
    And {
        conditions: Vec<QueryOperation>,
    },
    Or {
        conditions: Vec<QueryOperation>,
    },
    Not {
        condition: Box<QueryOperation>,
    },
    Project {
        columns: Vec<String>,
    },
    Sort {
        column: String,
        ascending: bool,
    },
    Limit {
        count: usize,
        offset: usize,
    },
    Aggregate {
        group_by: Vec<String>,
        aggregations: Vec<Aggregation>,
    },
    Sample {
        fraction: f64,
        seed: Option<u64>,
    },
    Distinct {
        columns: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    StartsWith,
    EndsWith,
    IsNull,
    IsNotNull,
    In,
    NotIn,
    Between,
    Regex,
}

impl std::fmt::Display for FilterOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eq => write!(f, "="),
            Self::Neq => write!(f, "!="),
            Self::Gt => write!(f, ">"),
            Self::Gte => write!(f, ">="),
            Self::Lt => write!(f, "<"),
            Self::Lte => write!(f, "<="),
            Self::Contains => write!(f, "contains"),
            Self::StartsWith => write!(f, "starts_with"),
            Self::EndsWith => write!(f, "ends_with"),
            Self::IsNull => write!(f, "is_null"),
            Self::IsNotNull => write!(f, "is_not_null"),
            Self::In => write!(f, "in"),
            Self::NotIn => write!(f, "not_in"),
            Self::Between => write!(f, "between"),
            Self::Regex => write!(f, "regex"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregation {
    pub column: String,
    pub function: AggFunction,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggFunction {
    Count,
    Sum,
    Mean,
    Min,
    Max,
    StdDev,
    DistinctCount,
}

impl std::fmt::Display for AggFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Count => write!(f, "count"),
            Self::Sum => write!(f, "sum"),
            Self::Mean => write!(f, "mean"),
            Self::Min => write!(f, "min"),
            Self::Max => write!(f, "max"),
            Self::StdDev => write!(f, "stddev"),
            Self::DistinctCount => write!(f, "distinct_count"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub num_rows: usize,
    pub num_columns: usize,
    pub column_names: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub execution_time_ms: f64,
    pub plan: QueryPlan,
}

pub struct QueryEngine;

impl QueryEngine {
    pub fn execute(table: &ArrowTable, plan: &QueryPlan) -> Result<QueryResult, String> {
        let start = std::time::Instant::now();

        let single = table.to_single_batch()?;
        let mut current = single;
        let mut current_schema = table.schema.clone();

        for op in &plan.operations {
            let result = Self::execute_operation(&current, current_schema.clone(), op)?;
            current = result.0;
            current_schema = result.1;
        }

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        let column_names: Vec<String> = current_schema.fields().iter()
            .map(|f| f.name().clone())
            .collect();

        let mut rows = Vec::with_capacity(current.num_rows());
        for row_idx in 0..current.num_rows() {
            let mut row = Vec::with_capacity(current.num_columns());
            for col_idx in 0..current.num_columns() {
                let col = current.column(col_idx);
                row.push(ArrowTable::value_to_json(col.as_ref(), row_idx));
            }
            rows.push(row);
        }

        Ok(QueryResult {
            num_rows: current.num_rows(),
            num_columns: current.num_columns(),
            column_names,
            rows,
            execution_time_ms: elapsed,
            plan: plan.clone(),
        })
    }

    fn execute_operation(
        batch: &RecordBatch,
        schema: arrow::datatypes::SchemaRef,
        op: &QueryOperation,
    ) -> Result<(RecordBatch, arrow::datatypes::SchemaRef), String> {
        match op {
            QueryOperation::Filter { column, operator, value } => {
                let mask = Self::build_filter_mask(batch, column, *operator, value)?;
                let filtered = filter_record_batch(batch, &mask)
                    .map_err(|e| format!("Filter failed: {}", e))?;
                Ok((filtered, schema))
            }

            QueryOperation::And { conditions } => {
                let mut combined_mask: Option<BooleanArray> = None;
                for cond in conditions {
                    let (_, mask) = match cond {
                        QueryOperation::Filter { column, operator, value } => {
                            let m = Self::build_filter_mask(batch, column, *operator, value)?;
                            Ok::<_, String>(((), Some(m)))
                        }
                        _ => Ok(((), None)),
                    }?;
                    if let Some(m) = mask {
                        combined_mask = match combined_mask {
                            Some(existing) => {
                                Some(arrow::compute::and(&existing, &m)
                                    .map_err(|e| format!("And failed: {}", e))?)
                            }
                            None => Some(m),
                        };
                    }
                }
                if let Some(mask) = combined_mask {
                    let filtered = filter_record_batch(batch, &mask)
                        .map_err(|e| format!("And filter failed: {}", e))?;
                    Ok((filtered, schema))
                } else {
                    Ok((batch.clone(), schema))
                }
            }

            QueryOperation::Or { conditions } => {
                let mut combined_mask: Option<BooleanArray> = None;
                for cond in conditions {
                    let (_, mask) = match cond {
                        QueryOperation::Filter { column, operator, value } => {
                            let m = Self::build_filter_mask(batch, column, *operator, value)?;
                            Ok::<_, String>(((), Some(m)))
                        }
                        _ => Ok(((), None)),
                    }?;
                    if let Some(m) = mask {
                        combined_mask = match combined_mask {
                            Some(existing) => {
                                Some(arrow::compute::or(&existing, &m)
                                    .map_err(|e| format!("Or failed: {}", e))?)
                            }
                            None => Some(m),
                        };
                    }
                }
                if let Some(mask) = combined_mask {
                    let filtered = filter_record_batch(batch, &mask)
                        .map_err(|e| format!("Or filter failed: {}", e))?;
                    Ok((filtered, schema))
                } else {
                    Ok((batch.clone(), schema))
                }
            }

            QueryOperation::Not { condition } => {
                if let QueryOperation::Filter { column, operator, value } = condition.as_ref() {
                    let mask = Self::build_filter_mask(batch, column, *operator, value)?;
                    let negated = arrow::compute::not(&mask)
                        .map_err(|e| format!("Not failed: {}", e))?;
                    let filtered = filter_record_batch(batch, &negated)
                        .map_err(|e| format!("Not filter failed: {}", e))?;
                    Ok((filtered, schema))
                } else {
                    Ok((batch.clone(), schema))
                }
            }

            QueryOperation::Project { columns } => {
                let mut fields = Vec::new();
                let mut arrays: Vec<arrow::array::ArrayRef> = Vec::new();

                for col_name in columns {
                    let idx = schema.index_of(col_name)
                        .map_err(|_| format!("Column '{}' not found", col_name))?;
                    fields.push(schema.field(idx).clone());
                    arrays.push(batch.column(idx).clone());
                }

                let new_schema = std::sync::Arc::new(arrow::datatypes::Schema::new(fields));
                let projected = RecordBatch::try_new(new_schema.clone(), arrays)
                    .map_err(|e| format!("Project failed: {}", e))?;
                Ok((projected, new_schema))
            }

            QueryOperation::Sort { column, ascending } => {
                let idx = schema.index_of(column)
                    .map_err(|_| format!("Column '{}' not found", column))?;
                let col = batch.column(idx);

                let sort_indices = arrow::compute::sort_to_indices(col, None, Some(1))
                    .map_err(|e| format!("Sort failed: {}", e))?;

                let mut sorted_columns: Vec<arrow::array::ArrayRef> = Vec::new();
                for i in 0..batch.num_columns() {
                    let taken = arrow::compute::take(batch.column(i), &sort_indices, None)
                        .map_err(|e| format!("Take failed: {}", e))?;
                    sorted_columns.push(taken);
                }

                let sorted = RecordBatch::try_new(schema.clone(), sorted_columns)
                    .map_err(|e| format!("Sort batch failed: {}", e))?;

                if !*ascending {
                    let reversed_columns: Vec<arrow::array::ArrayRef> = (0..sorted.num_columns())
                        .map(|i| {
                            let col = sorted.column(i);
                            let len = col.len();
                            let indices: Vec<u32> = (0..len as u32).rev().collect();
                            arrow::compute::take(col, &arrow::array::UInt32Array::from(indices), None)
                                .unwrap_or_else(|_| col.clone())
                        })
                        .collect();
                    let reversed = RecordBatch::try_new(schema.clone(), reversed_columns)
                        .map_err(|e| format!("Reverse failed: {}", e))?;
                    return Ok((reversed, schema));
                }

                Ok((sorted, schema))
            }

            QueryOperation::Limit { count, offset } => {
                let start = *offset;
                let end = (start + count).min(batch.num_rows());
                let sliced = batch.slice(start, end - start);
                Ok((sliced, schema))
            }

            QueryOperation::Sample { fraction, seed } => {
                let n = batch.num_rows();
                let sample_size = (n as f64 * fraction).ceil() as usize;
                let mut rng: Box<dyn rand::RngCore> = if let Some(s) = seed {
                    Box::new(rand::rngs::StdRng::seed_from_u64(*s))
                } else {
                    Box::new(rand::rngs::StdRng::from_entropy())
                };

                let mut indices: Vec<usize> = (0..n).collect();
                for i in (1..n).rev() {
                    let j = (rng.next_u64() as usize) % (i + 1);
                    indices.swap(i, j);
                }
                indices.truncate(sample_size);
                indices.sort();

                let idx_array = arrow::array::UInt64Array::from(
                    indices.iter().map(|&i| i as u64).collect::<Vec<_>>()
                );

                let mut sampled_columns: Vec<arrow::array::ArrayRef> = Vec::new();
                for i in 0..batch.num_columns() {
                    let taken = arrow::compute::take(batch.column(i), &idx_array, None)
                        .map_err(|e| format!("Sample take failed: {}", e))?;
                    sampled_columns.push(taken);
                }

                let sampled = RecordBatch::try_new(schema.clone(), sampled_columns)
                    .map_err(|e| format!("Sample batch failed: {}", e))?;
                Ok((sampled, schema))
            }

            QueryOperation::Distinct { columns: _ } => {
                let mut seen = std::collections::HashSet::new();
                let mut keep = Vec::new();

                for row_idx in 0..batch.num_rows() {
                    let mut key = String::new();
                    for col_idx in 0..batch.num_columns() {
                        let col = batch.column(col_idx);
                        let val = ArrowTable::value_to_json(col.as_ref(), row_idx);
                        key.push_str(&val.to_string());
                        key.push('\0');
                    }
                    if seen.insert(key) {
                        keep.push(row_idx as u64);
                    }
                }

                let idx_array = arrow::array::UInt64Array::from(keep);
                let mut distinct_columns: Vec<arrow::array::ArrayRef> = Vec::new();
                for i in 0..batch.num_columns() {
                    let taken = arrow::compute::take(batch.column(i), &idx_array, None)
                        .map_err(|e| format!("Distinct take failed: {}", e))?;
                    distinct_columns.push(taken);
                }

                let distinct = RecordBatch::try_new(schema.clone(), distinct_columns)
                    .map_err(|e| format!("Distinct batch failed: {}", e))?;
                Ok((distinct, schema))
            }

            QueryOperation::Aggregate { group_by, aggregations } => {
                if group_by.is_empty() {
                    let result = Self::execute_global_aggregate(batch, aggregations)?;
                    Ok((result.0, result.1))
                } else {
                    let result = Self::execute_grouped_aggregate(batch, &schema, group_by, aggregations)?;
                    Ok((result.0, result.1))
                }
            }
        }
    }

    fn build_filter_mask(
        batch: &RecordBatch,
        column: &str,
        operator: FilterOperator,
        value: &serde_json::Value,
    ) -> Result<BooleanArray, String> {
        let idx = batch.schema().index_of(column)
            .map_err(|_| format!("Column '{}' not found", column))?;
        let col = batch.column(idx);

        match operator {
            FilterOperator::IsNull => {
                let mut builder = BooleanArray::builder(col.len());
                for i in 0..col.len() {
                    builder.append_value(col.is_null(i));
                }
                Ok(builder.finish())
            }
            FilterOperator::IsNotNull => {
                let mut builder = BooleanArray::builder(col.len());
                for i in 0..col.len() {
                    builder.append_value(!col.is_null(i));
                }
                Ok(builder.finish())
            }
            _ => {
                let val_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };

                match col.data_type() {
                    DataType::Utf8 | DataType::LargeUtf8 => {
                        let arr = col.as_any().downcast_ref::<StringArray>()
                            .ok_or("Failed to cast to StringArray")?;
                        Self::string_filter(arr, operator, &val_str, value)
                    }
                    DataType::Int64 => {
                        let arr = col.as_any().downcast_ref::<Int64Array>()
                            .ok_or("Failed to cast to Int64Array")?;
                        let val_i64 = val_str.parse::<i64>().unwrap_or(0);
                        Self::int_filter(arr, operator, val_i64)
                    }
                    DataType::Float64 => {
                        let arr = col.as_any().downcast_ref::<Float64Array>()
                            .ok_or("Failed to cast to Float64Array")?;
                        let val_f64 = val_str.parse::<f64>().unwrap_or(0.0);
                        Self::float_filter(arr, operator, val_f64)
                    }
                    _ => {
                        let mut builder = BooleanArray::builder(col.len());
                        for _ in 0..col.len() {
                            builder.append_value(false);
                        }
                        Ok(builder.finish())
                    }
                }
            }
        }
    }

    fn string_filter(
        arr: &StringArray,
        operator: FilterOperator,
        val_str: &str,
        value: &serde_json::Value,
    ) -> Result<BooleanArray, String> {
        let mut builder = BooleanArray::builder(arr.len());
        match operator {
            FilterOperator::Eq => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) == val_str);
                }
            }
            FilterOperator::Neq => {
                for i in 0..arr.len() {
                    builder.append_value(arr.is_null(i) || arr.value(i) != val_str);
                }
            }
            FilterOperator::Contains => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i).contains(val_str));
                }
            }
            FilterOperator::StartsWith => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i).starts_with(val_str));
                }
            }
            FilterOperator::EndsWith => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i).ends_with(val_str));
                }
            }
            FilterOperator::In => {
                let set: std::collections::HashSet<String> = if let Some(arr) = value.as_array() {
                    arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()
                } else {
                    std::collections::HashSet::new()
                };
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && set.contains(arr.value(i)));
                }
            }
            FilterOperator::NotIn => {
                let set: std::collections::HashSet<String> = if let Some(arr) = value.as_array() {
                    arr.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()
                } else {
                    std::collections::HashSet::new()
                };
                for i in 0..arr.len() {
                    builder.append_value(arr.is_null(i) || !set.contains(arr.value(i)));
                }
            }
            FilterOperator::Regex => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i).contains(val_str));
                }
            }
            _ => {
                for _ in 0..arr.len() {
                    builder.append_value(false);
                }
            }
        }
        Ok(builder.finish())
    }

    fn int_filter(
        arr: &Int64Array,
        operator: FilterOperator,
        val: i64,
    ) -> Result<BooleanArray, String> {
        let mut builder = BooleanArray::builder(arr.len());
        match operator {
            FilterOperator::Eq => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) == val);
                }
            }
            FilterOperator::Neq => {
                for i in 0..arr.len() {
                    builder.append_value(arr.is_null(i) || arr.value(i) != val);
                }
            }
            FilterOperator::Gt => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) > val);
                }
            }
            FilterOperator::Gte => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) >= val);
                }
            }
            FilterOperator::Lt => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) < val);
                }
            }
            FilterOperator::Lte => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) <= val);
                }
            }
            _ => {
                for _ in 0..arr.len() {
                    builder.append_value(false);
                }
            }
        }
        Ok(builder.finish())
    }

    fn float_filter(
        arr: &Float64Array,
        operator: FilterOperator,
        val: f64,
    ) -> Result<BooleanArray, String> {
        let mut builder = BooleanArray::builder(arr.len());
        match operator {
            FilterOperator::Eq => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && (arr.value(i) - val).abs() < 1e-10);
                }
            }
            FilterOperator::Neq => {
                for i in 0..arr.len() {
                    builder.append_value(arr.is_null(i) || (arr.value(i) - val).abs() >= 1e-10);
                }
            }
            FilterOperator::Gt => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) > val);
                }
            }
            FilterOperator::Gte => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) >= val);
                }
            }
            FilterOperator::Lt => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) < val);
                }
            }
            FilterOperator::Lte => {
                for i in 0..arr.len() {
                    builder.append_value(!arr.is_null(i) && arr.value(i) <= val);
                }
            }
            _ => {
                for _ in 0..arr.len() {
                    builder.append_value(false);
                }
            }
        }
        Ok(builder.finish())
    }

    fn execute_global_aggregate(
        batch: &RecordBatch,
        aggregations: &[Aggregation],
    ) -> Result<(RecordBatch, arrow::datatypes::SchemaRef), String> {
        let mut fields = Vec::new();
        let mut values: Vec<arrow::array::ArrayRef> = Vec::new();

        for agg in aggregations {
            let idx = batch.schema().index_of(&agg.column)
                .map_err(|_| format!("Column '{}' not found", agg.column))?;
            let col = batch.column(idx);
            let name = agg.alias.clone().unwrap_or_else(|| {
                format!("{}_{}", agg.function, agg.column)
            });

            let result: f64 = match agg.function {
                AggFunction::Count => col.len() as f64 - col.null_count() as f64,
                AggFunction::Sum => {
                    Self::sum_column(col.as_ref())?
                }
                AggFunction::Mean => {
                    let s = Self::sum_column(col.as_ref())?;
                    let n = col.len() - col.null_count();
                    if n > 0 { s / n as f64 } else { 0.0 }
                }
                AggFunction::Min => {
                    Self::min_column(col.as_ref())?
                }
                AggFunction::Max => {
                    Self::max_column(col.as_ref())?
                }
                AggFunction::StdDev => {
                    let mean = {
                        let s = Self::sum_column(col.as_ref())?;
                        let n = col.len() - col.null_count();
                        if n > 0 { s / n as f64 } else { 0.0 }
                    };
                    let mut sum_sq = 0.0;
                    let mut count = 0usize;
                    for i in 0..col.len() {
                        if !col.is_null(i) {
                            if let Some(v) = Self::get_numeric_value(col.as_ref(), i) {
                                sum_sq += (v - mean).powi(2);
                                count += 1;
                            }
                        }
                    }
                    if count > 1 { (sum_sq / (count - 1) as f64).sqrt() } else { 0.0 }
                }
                AggFunction::DistinctCount => {
                    let mut seen = std::collections::HashSet::new();
                    for i in 0..col.len() {
                        if !col.is_null(i) {
                            let key = ArrowTable::value_to_json(col.as_ref(), i).to_string();
                            seen.insert(key);
                        }
                    }
                    seen.len() as f64
                }
            };

            fields.push(arrow::datatypes::Field::new(&name, DataType::Float64, true));
            values.push(std::sync::Arc::new(Float64Array::from(vec![result])));
        }

        let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(fields));
        let result_batch = RecordBatch::try_new(schema.clone(), values)
            .map_err(|e| format!("Aggregate batch failed: {}", e))?;
        Ok((result_batch, schema))
    }

    fn execute_grouped_aggregate(
        batch: &RecordBatch,
        schema: &arrow::datatypes::SchemaRef,
        group_by: &[String],
        aggregations: &[Aggregation],
    ) -> Result<(RecordBatch, arrow::datatypes::SchemaRef), String> {
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();

        for row_idx in 0..batch.num_rows() {
            let mut key = String::new();
            for gb in group_by {
                let idx = schema.index_of(gb)
                    .map_err(|_| format!("Group column '{}' not found", gb))?;
                let col = batch.column(idx);
                let val = ArrowTable::value_to_json(col.as_ref(), row_idx);
                key.push_str(&val.to_string());
                key.push('\0');
            }
            groups.entry(key).or_default().push(row_idx);
        }

        let mut result_fields: Vec<arrow::datatypes::Field> = group_by.iter()
            .map(|gb| {
                let idx = schema.index_of(gb).unwrap_or(0);
                schema.field(idx).clone()
            })
            .collect();

        for agg in aggregations {
            let name = agg.alias.clone().unwrap_or_else(|| {
                format!("{}_{}", agg.function, agg.column)
            });
            result_fields.push(arrow::datatypes::Field::new(&name, DataType::Float64, true));
        }

        let result_schema = std::sync::Arc::new(arrow::datatypes::Schema::new(result_fields));
        let mut result_columns: Vec<Vec<serde_json::Value>> = vec![Vec::new(); result_schema.fields().len()];

        for (key, indices) in &groups {
            let key_parts: Vec<&str> = key.split('\0').filter(|s| !s.is_empty()).collect();

            for (gi, _gb) in group_by.iter().enumerate() {
                let val = key_parts.get(gi).map(|s| serde_json::Value::String(s.to_string()))
                    .unwrap_or(serde_json::Value::Null);
                result_columns[gi].push(val);
            }

            let col_offset = group_by.len();
            for (ai, agg) in aggregations.iter().enumerate() {
                let agg_idx = schema.index_of(&agg.column)
                    .map_err(|_| 0).unwrap_or(0);
                let col = batch.column(agg_idx);

                let result: f64 = match agg.function {
                    AggFunction::Count => indices.len() as f64,
                    AggFunction::Sum => {
                        indices.iter()
                            .filter_map(|&i| Self::get_numeric_value(col.as_ref(), i))
                            .sum()
                    }
                    AggFunction::Mean => {
                        let vals: Vec<f64> = indices.iter()
                            .filter_map(|&i| Self::get_numeric_value(col.as_ref(), i))
                            .collect();
                        if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 }
                    }
                    AggFunction::Min => {
                        indices.iter()
                            .filter_map(|&i| Self::get_numeric_value(col.as_ref(), i))
                            .fold(f64::INFINITY, |a, b| a.min(b))
                    }
                    AggFunction::Max => {
                        indices.iter()
                            .filter_map(|&i| Self::get_numeric_value(col.as_ref(), i))
                            .fold(f64::NEG_INFINITY, |a, b| a.max(b))
                    }
                    AggFunction::StdDev => {
                        let vals: Vec<f64> = indices.iter()
                            .filter_map(|&i| Self::get_numeric_value(col.as_ref(), i))
                            .collect();
                        if vals.len() <= 1 { 0.0 } else {
                            let mean = vals.iter().sum::<f64>() / vals.len() as f64;
                            let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (vals.len() - 1) as f64;
                            var.sqrt()
                        }
                    }
                    AggFunction::DistinctCount => {
                        let mut seen = std::collections::HashSet::new();
                        for &i in indices {
                            let val = ArrowTable::value_to_json(col.as_ref(), i);
                            seen.insert(val.to_string());
                        }
                        seen.len() as f64
                    }
                };

                result_columns[col_offset + ai].push(serde_json::Value::Number(
                    serde_json::Number::from_f64(result).unwrap_or(serde_json::Number::from(0))
                ));
            }
        }

        let mut arrays: Vec<arrow::array::ArrayRef> = Vec::new();
        for col_idx in 0..result_schema.fields().len() {
            let field = result_schema.field(col_idx);
            let vals = &result_columns[col_idx];

            let arr: arrow::array::ArrayRef = match field.data_type() {
                DataType::Float64 => {
                    let fvals: Vec<f64> = vals.iter().map(|v| {
                        v.as_f64().unwrap_or(0.0)
                    }).collect();
                    std::sync::Arc::new(Float64Array::from(fvals))
                }
                _ => {
                    let svals: Vec<String> = vals.iter().map(|v| {
                        v.as_str().unwrap_or("").to_string()
                    }).collect();
                    std::sync::Arc::new(StringArray::from(svals))
                }
            };
            arrays.push(arr);
        }

        let result_batch = RecordBatch::try_new(result_schema.clone(), arrays)
            .map_err(|e| format!("Group aggregate batch failed: {}", e))?;
        Ok((result_batch, result_schema))
    }

    fn get_numeric_value(col: &dyn Array, idx: usize) -> Option<f64> {
        if col.is_null(idx) {
            return None;
        }
        match col.data_type() {
            DataType::Int64 => {
                col.as_any().downcast_ref::<Int64Array>()
                    .map(|a| a.value(idx) as f64)
            }
            DataType::Float64 => {
                col.as_any().downcast_ref::<Float64Array>()
                    .map(|a| a.value(idx))
            }
            _ => None,
        }
    }

    fn sum_column(col: &dyn Array) -> Result<f64, String> {
        let mut sum = 0.0;
        for i in 0..col.len() {
            if !col.is_null(i) {
                if let Some(v) = Self::get_numeric_value(col, i) {
                    sum += v;
                }
            }
        }
        Ok(sum)
    }

    fn min_column(col: &dyn Array) -> Result<f64, String> {
        let mut min_val = f64::INFINITY;
        let mut found = false;
        for i in 0..col.len() {
            if !col.is_null(i) {
                if let Some(v) = Self::get_numeric_value(col, i) {
                    min_val = min_val.min(v);
                    found = true;
                }
            }
        }
        Ok(if found { min_val } else { 0.0 })
    }

    fn max_column(col: &dyn Array) -> Result<f64, String> {
        let mut max_val = f64::NEG_INFINITY;
        let mut found = false;
        for i in 0..col.len() {
            if !col.is_null(i) {
                if let Some(v) = Self::get_numeric_value(col, i) {
                    max_val = max_val.max(v);
                    found = true;
                }
            }
        }
        Ok(if found { max_val } else { 0.0 })
    }

    pub fn optimize_plan(plan: &mut QueryPlan) {
        plan.operations = Self::reorder_operations(plan.operations.clone());
    }

    fn reorder_operations(ops: Vec<QueryOperation>) -> Vec<QueryOperation> {
        let mut filters = Vec::new();
        let mut projections = Vec::new();
        let mut sorts = Vec::new();
        let mut limits = Vec::new();
        let mut others = Vec::new();

        for op in ops {
            match &op {
                QueryOperation::Filter { .. }
                | QueryOperation::And { .. }
                | QueryOperation::Or { .. }
                | QueryOperation::Not { .. } => filters.push(op),
                QueryOperation::Project { .. } => projections.push(op),
                QueryOperation::Sort { .. } => sorts.push(op),
                QueryOperation::Limit { .. } => limits.push(op),
                _ => others.push(op),
            }
        }

        let mut result = Vec::new();
        result.append(&mut filters);
        result.append(&mut projections);
        result.append(&mut sorts);
        result.append(&mut limits);
        result.append(&mut others);
        result
    }

    pub fn estimate_cost(plan: &QueryPlan, input_rows: usize) -> f64 {
        let mut cost = 0.0;
        for op in &plan.operations {
            cost += match op {
                QueryOperation::Filter { .. } => input_rows as f64 * 0.01,
                QueryOperation::Sort { .. } => input_rows as f64 * (input_rows as f64).ln(),
                QueryOperation::Aggregate { .. } => input_rows as f64 * 0.1,
                QueryOperation::Distinct { .. } => input_rows as f64 * 0.05,
                _ => input_rows as f64 * 0.001,
            };
        }
        cost
    }
}
