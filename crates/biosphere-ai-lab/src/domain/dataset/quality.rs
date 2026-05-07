use crate::domain::dataset::aggregate::{
    ColumnProfile, DataExpectation, DataQualityReport, Dataset, ExpectationResult,
    ExpectationSeverity, ExpectationType,
};

pub struct QualityEngine;

impl QualityEngine {
    pub fn validate(dataset: &Dataset, expectations: &[DataExpectation]) -> DataQualityReport {
        let results: Vec<ExpectationResult> = expectations
            .iter()
            .filter(|e| e.enabled)
            .map(|expectation| Self::check_expectation(dataset, expectation))
            .collect();

        DataQualityReport::new(
            dataset.id.to_string(),
            dataset.version.to_string(),
            results,
        )
    }

    fn check_expectation(dataset: &Dataset, expectation: &DataExpectation) -> ExpectationResult {
        match &expectation.expectation_type {
            ExpectationType::NotNull => {
                let col_name = match &expectation.column {
                    Some(c) => c,
                    None => {
                        return ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: None,
                            severity: expectation.severity,
                            passed: false,
                            message: "NotNull expectation requires a column name".to_string(),
                            details: None,
                        };
                    }
                };

                match Self::find_profile(&dataset.column_profiles, col_name) {
                    Some(profile) => {
                        let passed = profile.null_count == 0;
                        ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: Some(col_name.clone()),
                            severity: expectation.severity,
                            passed,
                            message: if passed {
                                format!("Column '{}' has no null values", col_name)
                            } else {
                                format!("Column '{}' has {} null values", col_name, profile.null_count)
                            },
                            details: Some(serde_json::json!({
                                "null_count": profile.null_count,
                                "total_rows": dataset.rows,
                            })),
                        }
                    }
                    None => ExpectationResult {
                        expectation_id: expectation.id.clone(),
                        expectation_name: expectation.name.clone(),
                        expectation_type: expectation.expectation_type.clone(),
                        column: Some(col_name.clone()),
                        severity: expectation.severity,
                        passed: false,
                        message: format!("Column '{}' not found in dataset", col_name),
                        details: None,
                    },
                }
            }

            ExpectationType::Unique => {
                let col_name = match &expectation.column {
                    Some(c) => c,
                    None => {
                        return ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: None,
                            severity: expectation.severity,
                            passed: false,
                            message: "Unique expectation requires a column name".to_string(),
                            details: None,
                        };
                    }
                };

                match Self::find_profile(&dataset.column_profiles, col_name) {
                    Some(profile) => {
                        let unique = profile.distinct_count;
                        let passed = unique == dataset.rows;
                        ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: Some(col_name.clone()),
                            severity: expectation.severity,
                            passed,
                            message: if passed {
                                format!("Column '{}' has all unique values", col_name)
                            } else {
                                format!("Column '{}' has {} unique values out of {} rows", col_name, unique, dataset.rows)
                            },
                            details: Some(serde_json::json!({
                                "unique_values": unique,
                                "total_rows": dataset.rows,
                                "duplicate_count": dataset.rows.saturating_sub(unique),
                            })),
                        }
                    }
                    None => ExpectationResult {
                        expectation_id: expectation.id.clone(),
                        expectation_name: expectation.name.clone(),
                        expectation_type: expectation.expectation_type.clone(),
                        column: Some(col_name.clone()),
                        severity: expectation.severity,
                        passed: false,
                        message: format!("Column '{}' not found in dataset", col_name),
                        details: None,
                    },
                }
            }

            ExpectationType::InRange { min, max } => {
                let col_name = match &expectation.column {
                    Some(c) => c,
                    None => {
                        return ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: None,
                            severity: expectation.severity,
                            passed: false,
                            message: "InRange expectation requires a column name".to_string(),
                            details: None,
                        };
                    }
                };

                match Self::find_profile(&dataset.column_profiles, col_name) {
                    Some(profile) => {
                        let col_min = profile.min_value.as_ref().and_then(|v| v.parse::<f64>().ok());
                        let col_max = profile.max_value.as_ref().and_then(|v| v.parse::<f64>().ok());

                        match (col_min, col_max) {
                            (Some(c_min), Some(c_max)) => {
                                let passed = c_min >= *min && c_max <= *max;
                                ExpectationResult {
                                    expectation_id: expectation.id.clone(),
                                    expectation_name: expectation.name.clone(),
                                    expectation_type: expectation.expectation_type.clone(),
                                    column: Some(col_name.clone()),
                                    severity: expectation.severity,
                                    passed,
                                    message: if passed {
                                        format!("Column '{}' values in range [{}, {}]", col_name, c_min, c_max)
                                    } else {
                                        format!("Column '{}' values [{}, {}] exceed expected range [{}, {}]", col_name, c_min, c_max, min, max)
                                    },
                                    details: Some(serde_json::json!({
                                        "actual_min": c_min,
                                        "actual_max": c_max,
                                        "expected_min": min,
                                        "expected_max": max,
                                    })),
                                }
                            }
                            _ => ExpectationResult {
                                expectation_id: expectation.id.clone(),
                                expectation_name: expectation.name.clone(),
                                expectation_type: expectation.expectation_type.clone(),
                                column: Some(col_name.clone()),
                                severity: expectation.severity,
                                passed: false,
                                message: format!("Column '{}' does not have numeric min/max values", col_name),
                                details: None,
                            },
                        }
                    }
                    None => ExpectationResult {
                        expectation_id: expectation.id.clone(),
                        expectation_name: expectation.name.clone(),
                        expectation_type: expectation.expectation_type.clone(),
                        column: Some(col_name.clone()),
                        severity: expectation.severity,
                        passed: false,
                        message: format!("Column '{}' not found in dataset", col_name),
                        details: None,
                    },
                }
            }

            ExpectationType::InSet { values } => {
                let col_name = match &expectation.column {
                    Some(c) => c,
                    None => {
                        return ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: None,
                            severity: expectation.severity,
                            passed: false,
                            message: "InSet expectation requires a column name".to_string(),
                            details: None,
                        };
                    }
                };

                match Self::find_profile(&dataset.column_profiles, col_name) {
                    Some(profile) => {
                        let unique = profile.distinct_count;
                        let passed = unique <= values.len();
                        ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: Some(col_name.clone()),
                            severity: expectation.severity,
                            passed,
                            message: if passed {
                                format!("Column '{}' values are within the allowed set", col_name)
                            } else {
                                format!("Column '{}' has {} unique values but only {} allowed", col_name, unique, values.len())
                            },
                            details: Some(serde_json::json!({
                                "unique_values": unique,
                                "allowed_count": values.len(),
                            })),
                        }
                    }
                    None => ExpectationResult {
                        expectation_id: expectation.id.clone(),
                        expectation_name: expectation.name.clone(),
                        expectation_type: expectation.expectation_type.clone(),
                        column: Some(col_name.clone()),
                        severity: expectation.severity,
                        passed: false,
                        message: format!("Column '{}' not found in dataset", col_name),
                        details: None,
                    },
                }
            }

            ExpectationType::TypeMatch { expected_type } => {
                let col_name = match &expectation.column {
                    Some(c) => c,
                    None => {
                        return ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: None,
                            severity: expectation.severity,
                            passed: false,
                            message: "TypeMatch expectation requires a column name".to_string(),
                            details: None,
                        };
                    }
                };

                match Self::find_profile(&dataset.column_profiles, col_name) {
                    Some(profile) => {
                        let passed = profile.column_type.to_string() == *expected_type;
                        ExpectationResult {
                            expectation_id: expectation.id.clone(),
                            expectation_name: expectation.name.clone(),
                            expectation_type: expectation.expectation_type.clone(),
                            column: Some(col_name.clone()),
                            severity: expectation.severity,
                            passed,
                            message: if passed {
                                format!("Column '{}' type matches '{}'", col_name, expected_type)
                            } else {
                                format!("Column '{}' type is '{}' but expected '{}'", col_name, profile.column_type, expected_type)
                            },
                            details: Some(serde_json::json!({
                                "actual_type": profile.column_type.to_string(),
                                "expected_type": expected_type,
                            })),
                        }
                    }
                    None => ExpectationResult {
                        expectation_id: expectation.id.clone(),
                        expectation_name: expectation.name.clone(),
                        expectation_type: expectation.expectation_type.clone(),
                        column: Some(col_name.clone()),
                        severity: expectation.severity,
                        passed: false,
                        message: format!("Column '{}' not found in dataset", col_name),
                        details: None,
                    },
                }
            }

            ExpectationType::RowCountBetween { min, max } => {
                let passed = dataset.rows >= *min && dataset.rows <= *max;
                ExpectationResult {
                    expectation_id: expectation.id.clone(),
                    expectation_name: expectation.name.clone(),
                    expectation_type: expectation.expectation_type.clone(),
                    column: None,
                    severity: expectation.severity,
                    passed,
                    message: if passed {
                        format!("Row count {} is within [{}, {}]", dataset.rows, min, max)
                    } else {
                        format!("Row count {} is outside [{}, {}]", dataset.rows, min, max)
                    },
                    details: Some(serde_json::json!({
                        "actual_rows": dataset.rows,
                        "min": min,
                        "max": max,
                    })),
                }
            }

            ExpectationType::NoDuplicateColumns => {
                let mut seen = std::collections::HashSet::new();
                let mut duplicates = Vec::new();
                for profile in &dataset.column_profiles {
                    if !seen.insert(&profile.name) {
                        duplicates.push(profile.name.clone());
                    }
                }
                let passed = duplicates.is_empty();
                ExpectationResult {
                    expectation_id: expectation.id.clone(),
                    expectation_name: expectation.name.clone(),
                    expectation_type: expectation.expectation_type.clone(),
                    column: None,
                    severity: expectation.severity,
                    passed,
                    message: if passed {
                        "No duplicate columns found".to_string()
                    } else {
                        format!("Duplicate columns found: {}", duplicates.join(", "))
                    },
                    details: if passed {
                        None
                    } else {
                        Some(serde_json::json!({ "duplicates": duplicates }))
                    },
                }
            }

            ExpectationType::SchemaMatch { expected_columns } => {
                let actual_columns: Vec<&str> = dataset.column_profiles.iter().map(|p| p.name.as_str()).collect();
                let expected_set: std::collections::HashSet<&str> = expected_columns.iter().map(|s| s.as_str()).collect();
                let actual_set: std::collections::HashSet<&str> = actual_columns.iter().copied().collect();

                let missing: Vec<&str> = expected_set.difference(&actual_set).copied().collect();
                let extra: Vec<&str> = actual_set.difference(&expected_set).copied().collect();

                let passed = missing.is_empty() && extra.is_empty();
                ExpectationResult {
                    expectation_id: expectation.id.clone(),
                    expectation_name: expectation.name.clone(),
                    expectation_type: expectation.expectation_type.clone(),
                    column: None,
                    severity: expectation.severity,
                    passed,
                    message: if passed {
                        "Schema matches expected columns".to_string()
                    } else {
                        let mut parts = Vec::new();
                        if !missing.is_empty() {
                            parts.push(format!("missing: {}", missing.join(", ")));
                        }
                        if !extra.is_empty() {
                            parts.push(format!("extra: {}", extra.join(", ")));
                        }
                        format!("Schema mismatch: {}", parts.join("; "))
                    },
                    details: if passed {
                        None
                    } else {
                        Some(serde_json::json!({
                            "missing_columns": missing,
                            "extra_columns": extra,
                        }))
                    },
                }
            }
        }
    }

    fn find_profile<'a>(profiles: &'a [ColumnProfile], name: &str) -> Option<&'a ColumnProfile> {
        profiles.iter().find(|p| p.name == name)
    }

    pub fn auto_generate_expectations(dataset: &Dataset) -> Vec<DataExpectation> {
        let mut expectations = Vec::new();

        expectations.push(DataExpectation::row_count_between(
            "minimum_row_count".to_string(),
            1,
            10_000_000,
            ExpectationSeverity::Error,
        ));

        for profile in &dataset.column_profiles {
            if profile.null_count > 0 {
                expectations.push(DataExpectation::not_null(
                    profile.name.clone(),
                    format!("{}_not_null", profile.name),
                    ExpectationSeverity::Warning,
                ));
            }

            if profile.min_value.is_some() && profile.max_value.is_some() {
                if let (Some(min_str), Some(max_str)) = (&profile.min_value, &profile.max_value) {
                    if let (Ok(min), Ok(max)) = (min_str.parse::<f64>(), max_str.parse::<f64>()) {
                        let expanded_min = min - (max - min).abs() * 0.1;
                        let expanded_max = max + (max - min).abs() * 0.1;
                        expectations.push(DataExpectation::in_range(
                            profile.name.clone(),
                            format!("{}_in_range", profile.name),
                            expanded_min,
                            expanded_max,
                            ExpectationSeverity::Warning,
                        ));
                    }
                }
            }

            expectations.push(DataExpectation::unique(
                profile.name.clone(),
                format!("{}_unique_check", profile.name),
                ExpectationSeverity::Info,
            ));
        }

        expectations
    }
}
