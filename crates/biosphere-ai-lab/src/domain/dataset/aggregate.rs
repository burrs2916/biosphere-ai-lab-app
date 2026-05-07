use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::types::DataFormat;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatasetId(String);

impl DatasetId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for DatasetId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DatasetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatasetVersion(String);

impl DatasetVersion {
    pub fn initial() -> Self {
        Self("v1".to_string())
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn bump(&self) -> Self {
        let num = self.0.trim_start_matches('v')
            .parse::<u32>()
            .unwrap_or(1);
        Self(format!("v{}", num + 1))
    }
}

impl std::fmt::Display for DatasetVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatasetStatus {
    #[serde(alias = "Active")]
    Active,
    #[serde(alias = "Archived")]
    Archived,
    #[serde(alias = "Deleted")]
    Deleted,
}

impl std::fmt::Display for DatasetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Archived => write!(f, "archived"),
            Self::Deleted => write!(f, "deleted"),
        }
    }
}

impl std::str::FromStr for DatasetStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            "deleted" => Ok(Self::Deleted),
            _ => Err(format!("Unknown dataset status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnType {
    Integer,
    Float,
    Boolean,
    String,
    DateTime,
    Categorical,
    Unknown,
}

impl std::fmt::Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer => write!(f, "integer"),
            Self::Float => write!(f, "float"),
            Self::Boolean => write!(f, "boolean"),
            Self::String => write!(f, "string"),
            Self::DateTime => write!(f, "datetime"),
            Self::Categorical => write!(f, "categorical"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for ColumnType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "integer" | "int" => Ok(Self::Integer),
            "float" | "number" | "numeric" => Ok(Self::Float),
            "boolean" | "bool" => Ok(Self::Boolean),
            "string" | "text" => Ok(Self::String),
            "datetime" | "date" => Ok(Self::DateTime),
            "categorical" | "category" => Ok(Self::Categorical),
            _ => Ok(Self::Unknown),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub name: String,
    pub column_type: ColumnType,
    pub null_count: usize,
    pub distinct_count: usize,
    pub total_count: usize,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub mean_value: Option<f64>,
    pub std_value: Option<f64>,
    pub median_value: Option<f64>,
    pub top_values: Vec<(String, usize)>,
}

impl ColumnProfile {
    pub fn null_rate(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        self.null_count as f64 / self.total_count as f64
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self.column_type, ColumnType::Integer | ColumnType::Float)
    }

    pub fn is_categorical(&self) -> bool {
        matches!(self.column_type, ColumnType::Categorical | ColumnType::Boolean)
            || (self.column_type == ColumnType::String && self.distinct_count <= 20 && self.total_count > 20)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub id: DatasetId,
    pub name: String,
    pub version: DatasetVersion,
    pub status: DatasetStatus,
    pub format: DataFormat,
    pub path: String,
    pub digest: String,
    pub rows: usize,
    pub columns: usize,
    pub column_profiles: Vec<ColumnProfile>,
    pub memory_size_mb: f64,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub source_type: Option<String>,
    pub source_uri: Option<String>,
    pub experiment_ids: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub card: Option<DatasetCard>,
    pub version_history: Vec<DatasetVersionRecord>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Dataset {
    pub fn register(
        name: String,
        format: DataFormat,
        path: String,
        digest: String,
        rows: usize,
        columns: usize,
        column_profiles: Vec<ColumnProfile>,
        memory_size_mb: f64,
    ) -> Self {
        let now = Utc::now();
        let initial_version_record = DatasetVersionRecord {
            version: DatasetVersion::initial().to_string(),
            digest: digest.clone(),
            rows,
            columns,
            column_profiles: column_profiles.clone(),
            memory_size_mb,
            created_at: now,
            change_note: None,
        };
        Self {
            id: DatasetId::new(),
            name,
            version: DatasetVersion::initial(),
            status: DatasetStatus::Active,
            format,
            path,
            digest,
            rows,
            columns,
            column_profiles,
            memory_size_mb,
            tags: Vec::new(),
            description: None,
            source_type: None,
            source_uri: None,
            experiment_ids: Vec::new(),
            metadata: HashMap::new(),
            card: None,
            version_history: vec![initial_version_record],
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_version(&mut self, new_digest: String, new_rows: usize, new_columns: usize, new_profiles: Vec<ColumnProfile>, new_size_mb: f64) -> Result<(), String> {
        if new_digest == self.digest {
            return Err("Dataset content unchanged (digest matches current version)".to_string());
        }
        self.version = self.version.bump();
        self.digest = new_digest.clone();
        self.rows = new_rows;
        self.columns = new_columns;
        self.column_profiles = new_profiles.clone();
        self.memory_size_mb = new_size_mb;
        self.version_history.push(DatasetVersionRecord {
            version: self.version.to_string(),
            digest: new_digest,
            rows: new_rows,
            columns: new_columns,
            column_profiles: new_profiles,
            memory_size_mb: new_size_mb,
            created_at: Utc::now(),
            change_note: None,
        });
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn new_version_with_note(&mut self, new_digest: String, new_rows: usize, new_columns: usize, new_profiles: Vec<ColumnProfile>, new_size_mb: f64, note: String) -> Result<(), String> {
        if new_digest == self.digest {
            return Err("Dataset content unchanged (digest matches current version)".to_string());
        }
        self.version = self.version.bump();
        self.digest = new_digest.clone();
        self.rows = new_rows;
        self.columns = new_columns;
        self.column_profiles = new_profiles.clone();
        self.memory_size_mb = new_size_mb;
        self.version_history.push(DatasetVersionRecord {
            version: self.version.to_string(),
            digest: new_digest,
            rows: new_rows,
            columns: new_columns,
            column_profiles: new_profiles,
            memory_size_mb: new_size_mb,
            created_at: Utc::now(),
            change_note: Some(note),
        });
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn link_experiment(&mut self, experiment_id: String) {
        if !self.experiment_ids.contains(&experiment_id) {
            self.experiment_ids.push(experiment_id);
            self.updated_at = Utc::now();
        }
    }

    pub fn unlink_experiment(&mut self, experiment_id: &str) {
        self.experiment_ids.retain(|id| id != experiment_id);
        self.updated_at = Utc::now();
    }

    pub fn archive(&mut self) -> Result<(), String> {
        match self.status {
            DatasetStatus::Active => {
                self.status = DatasetStatus::Archived;
                self.updated_at = Utc::now();
                Ok(())
            }
            DatasetStatus::Archived => Err("Dataset is already archived".to_string()),
            DatasetStatus::Deleted => Err("Cannot archive a deleted dataset".to_string()),
        }
    }

    pub fn restore(&mut self) -> Result<(), String> {
        match self.status {
            DatasetStatus::Archived => {
                self.status = DatasetStatus::Active;
                self.updated_at = Utc::now();
                Ok(())
            }
            DatasetStatus::Active => Err("Dataset is already active".to_string()),
            DatasetStatus::Deleted => Err("Cannot restore a deleted dataset".to_string()),
        }
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
        self.updated_at = Utc::now();
    }

    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
        self.updated_at = Utc::now();
    }

    pub fn set_source(&mut self, source_type: String, source_uri: String) {
        self.source_type = Some(source_type);
        self.source_uri = Some(source_uri);
        self.updated_at = Utc::now();
    }

    pub fn set_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }

    pub fn set_card(&mut self, card: DatasetCard) {
        self.card = Some(card);
        self.updated_at = Utc::now();
    }

    pub fn has_missing_values(&self) -> bool {
        self.column_profiles.iter().any(|p| p.null_count > 0)
    }

    pub fn numeric_columns(&self) -> Vec<&ColumnProfile> {
        self.column_profiles.iter().filter(|p| p.is_numeric()).collect()
    }

    pub fn categorical_columns(&self) -> Vec<&ColumnProfile> {
        self.column_profiles.iter().filter(|p| p.is_categorical()).collect()
    }

    pub fn compute_digest(data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSummary {
    pub id: DatasetId,
    pub name: String,
    pub version: String,
    pub status: DatasetStatus,
    pub format: DataFormat,
    pub rows: usize,
    pub columns: usize,
    pub has_missing_values: bool,
    pub memory_size_mb: f64,
    pub tags: Vec<String>,
    pub experiment_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Dataset> for DatasetSummary {
    fn from(ds: &Dataset) -> Self {
        Self {
            id: ds.id.clone(),
            name: ds.name.clone(),
            version: ds.version.to_string(),
            status: ds.status,
            format: ds.format,
            rows: ds.rows,
            columns: ds.columns,
            has_missing_values: ds.has_missing_values(),
            memory_size_mb: ds.memory_size_mb,
            tags: ds.tags.clone(),
            experiment_count: ds.experiment_ids.len(),
            created_at: ds.created_at,
            updated_at: ds.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetVersionRecord {
    pub version: String,
    pub digest: String,
    pub rows: usize,
    pub columns: usize,
    pub column_profiles: Vec<ColumnProfile>,
    pub memory_size_mb: f64,
    pub created_at: DateTime<Utc>,
    pub change_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetFilter {
    pub status: Option<DatasetStatus>,
    pub format: Option<DataFormat>,
    pub name_contains: Option<String>,
    pub tags: Vec<String>,
    pub has_missing_values: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl Default for DatasetFilter {
    fn default() -> Self {
        Self {
            status: None,
            format: None,
            name_contains: None,
            tags: Vec::new(),
            has_missing_values: None,
            limit: None,
            offset: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitStrategy {
    Random,
    Stratified,
    Temporal,
    Group,
}

impl std::fmt::Display for SplitStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Random => write!(f, "random"),
            Self::Stratified => write!(f, "stratified"),
            Self::Temporal => write!(f, "temporal"),
            Self::Group => write!(f, "group"),
        }
    }
}

impl std::str::FromStr for SplitStrategy {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "random" => Ok(Self::Random),
            "stratified" => Ok(Self::Stratified),
            "temporal" => Ok(Self::Temporal),
            "group" => Ok(Self::Group),
            _ => Err(format!("Unknown split strategy: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSplit {
    pub name: String,
    pub strategy: SplitStrategy,
    pub train_ratio: f64,
    pub val_ratio: f64,
    pub test_ratio: f64,
    pub seed: u64,
    pub stratify_column: Option<String>,
    pub group_column: Option<String>,
    pub train_indices: Vec<usize>,
    pub val_indices: Vec<usize>,
    pub test_indices: Vec<usize>,
    pub created_at: DateTime<Utc>,
}

impl DatasetSplit {
    pub fn new_random(name: String, train_ratio: f64, val_ratio: f64, test_ratio: f64, seed: u64, total_rows: usize) -> Self {
        let (train_idx, val_idx, test_idx) = Self::split_random(total_rows, train_ratio, val_ratio, test_ratio, seed);
        Self {
            name,
            strategy: SplitStrategy::Random,
            train_ratio,
            val_ratio,
            test_ratio,
            seed,
            stratify_column: None,
            group_column: None,
            train_indices: train_idx,
            val_indices: val_idx,
            test_indices: test_idx,
            created_at: Utc::now(),
        }
    }

    pub fn new_stratified(
        name: String,
        train_ratio: f64,
        val_ratio: f64,
        test_ratio: f64,
        seed: u64,
        stratify_column: String,
        column_values: &[String],
    ) -> Self {
        let (train_idx, val_idx, test_idx) = Self::split_stratified(
            column_values, train_ratio, val_ratio, test_ratio, seed,
        );
        Self {
            name,
            strategy: SplitStrategy::Stratified,
            train_ratio,
            val_ratio,
            test_ratio,
            seed,
            stratify_column: Some(stratify_column),
            group_column: None,
            train_indices: train_idx,
            val_indices: val_idx,
            test_indices: test_idx,
            created_at: Utc::now(),
        }
    }

    pub fn new_temporal(
        name: String,
        train_ratio: f64,
        val_ratio: f64,
        test_ratio: f64,
        total_rows: usize,
    ) -> Self {
        let train_end = (total_rows as f64 * train_ratio) as usize;
        let val_end = train_end + (total_rows as f64 * val_ratio) as usize;

        let train_idx: Vec<usize> = (0..train_end).collect();
        let val_idx: Vec<usize> = (train_end..val_end).collect();
        let test_idx: Vec<usize> = (val_end..total_rows).collect();

        Self {
            name,
            strategy: SplitStrategy::Temporal,
            train_ratio,
            val_ratio,
            test_ratio,
            seed: 0,
            stratify_column: None,
            group_column: None,
            train_indices: train_idx,
            val_indices: val_idx,
            test_indices: test_idx,
            created_at: Utc::now(),
        }
    }

    pub fn new_group(
        name: String,
        train_ratio: f64,
        val_ratio: f64,
        test_ratio: f64,
        seed: u64,
        group_column: String,
        group_values: &[String],
    ) -> Self {
        let (train_idx, val_idx, test_idx) = Self::split_by_groups(
            group_values, train_ratio, val_ratio, test_ratio, seed,
        );
        Self {
            name,
            strategy: SplitStrategy::Group,
            train_ratio,
            val_ratio,
            test_ratio,
            seed,
            stratify_column: None,
            group_column: Some(group_column),
            train_indices: train_idx,
            val_indices: val_idx,
            test_indices: test_idx,
            created_at: Utc::now(),
        }
    }

    fn split_random(total: usize, train_ratio: f64, val_ratio: f64, _test_ratio: f64, seed: u64) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
        let mut indices: Vec<usize> = (0..total).collect();
        Self::shuffle_with_seed(&mut indices, seed);

        let train_end = (total as f64 * train_ratio) as usize;
        let val_end = train_end + (total as f64 * val_ratio) as usize;

        let train = indices[..train_end].to_vec();
        let val = indices[train_end..val_end].to_vec();
        let test = indices[val_end..].to_vec();

        (train, val, test)
    }

    fn split_stratified(
        column_values: &[String],
        train_ratio: f64,
        val_ratio: f64,
        _test_ratio: f64,
        seed: u64,
    ) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, v) in column_values.iter().enumerate() {
            groups.entry(v.clone()).or_default().push(i);
        }

        let mut train = Vec::new();
        let mut val = Vec::new();
        let mut test = Vec::new();

        let mut seed_state = seed;
        for (_, mut indices) in groups {
            Self::shuffle_with_seed(&mut indices, seed_state);
            seed_state = seed_state.wrapping_add(1);

            let n = indices.len();
            let train_end = (n as f64 * train_ratio) as usize;
            let val_end = train_end + (n as f64 * val_ratio) as usize;

            train.extend_from_slice(&indices[..train_end]);
            val.extend_from_slice(&indices[train_end..val_end.min(n)]);
            test.extend_from_slice(&indices[val_end.min(n)..]);
        }

        (train, val, test)
    }

    fn split_by_groups(
        group_values: &[String],
        train_ratio: f64,
        val_ratio: f64,
        _test_ratio: f64,
        seed: u64,
    ) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
        let unique_groups: Vec<String> = group_values.iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let mut group_indices: Vec<String> = unique_groups;
        Self::shuffle_with_seed_str(&mut group_indices, seed);

        let n = group_indices.len();
        let train_end = (n as f64 * train_ratio) as usize;
        let val_end = train_end + (n as f64 * val_ratio) as usize;

        let train_groups: std::collections::HashSet<&str> = group_indices[..train_end]
            .iter().map(|s| s.as_str()).collect();
        let val_groups: std::collections::HashSet<&str> = group_indices[train_end..val_end.min(n)]
            .iter().map(|s| s.as_str()).collect();

        let mut train = Vec::new();
        let mut val = Vec::new();
        let mut test = Vec::new();

        for (i, g) in group_values.iter().enumerate() {
            if train_groups.contains(g.as_str()) {
                train.push(i);
            } else if val_groups.contains(g.as_str()) {
                val.push(i);
            } else {
                test.push(i);
            }
        }

        (train, val, test)
    }

    fn shuffle_with_seed(indices: &mut [usize], seed: u64) {
        let mut state = seed;
        for i in (1..indices.len()).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (state >> 33) as usize % (i + 1);
            indices.swap(i, j);
        }
    }

    fn shuffle_with_seed_str(items: &mut [String], seed: u64) {
        let mut state = seed;
        for i in (1..items.len()).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (state >> 33) as usize % (i + 1);
            items.swap(i, j);
        }
    }

    pub fn train_count(&self) -> usize {
        self.train_indices.len()
    }

    pub fn val_count(&self) -> usize {
        self.val_indices.len()
    }

    pub fn test_count(&self) -> usize {
        self.test_indices.len()
    }

    pub fn total_count(&self) -> usize {
        self.train_count() + self.val_count() + self.test_count()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.train_ratio <= 0.0 || self.val_ratio <= 0.0 || self.test_ratio <= 0.0 {
            return Err("Split ratios must be positive".to_string());
        }
        let total = self.train_ratio + self.val_ratio + self.test_ratio;
        if (total - 1.0).abs() > 0.01 {
            return Err(format!("Split ratios must sum to 1.0, got {}", total));
        }
        if self.name.trim().is_empty() {
            return Err("Split name cannot be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectationType {
    NotNull,
    Unique,
    InRange { min: f64, max: f64 },
    InSet { values: Vec<String> },
    TypeMatch { expected_type: String },
    RowCountBetween { min: usize, max: usize },
    NoDuplicateColumns,
    SchemaMatch { expected_columns: Vec<String> },
}

impl std::fmt::Display for ExpectationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotNull => write!(f, "not_null"),
            Self::Unique => write!(f, "unique"),
            Self::InRange { min, max } => write!(f, "in_range[{},{}]", min, max),
            Self::InSet { .. } => write!(f, "in_set"),
            Self::TypeMatch { expected_type } => write!(f, "type_match[{}]", expected_type),
            Self::RowCountBetween { min, max } => write!(f, "row_count_between[{},{}]", min, max),
            Self::NoDuplicateColumns => write!(f, "no_duplicate_columns"),
            Self::SchemaMatch { .. } => write!(f, "schema_match"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExpectation {
    pub id: String,
    pub name: String,
    pub expectation_type: ExpectationType,
    pub column: Option<String>,
    pub severity: ExpectationSeverity,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectationSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for ExpectationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

impl DataExpectation {
    pub fn not_null(column: String, name: String, severity: ExpectationSeverity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            expectation_type: ExpectationType::NotNull,
            column: Some(column),
            severity,
            enabled: true,
        }
    }

    pub fn unique(column: String, name: String, severity: ExpectationSeverity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            expectation_type: ExpectationType::Unique,
            column: Some(column),
            severity,
            enabled: true,
        }
    }

    pub fn in_range(column: String, name: String, min: f64, max: f64, severity: ExpectationSeverity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            expectation_type: ExpectationType::InRange { min, max },
            column: Some(column),
            severity,
            enabled: true,
        }
    }

    pub fn in_set(column: String, name: String, values: Vec<String>, severity: ExpectationSeverity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            expectation_type: ExpectationType::InSet { values },
            column: Some(column),
            severity,
            enabled: true,
        }
    }

    pub fn row_count_between(name: String, min: usize, max: usize, severity: ExpectationSeverity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            expectation_type: ExpectationType::RowCountBetween { min, max },
            column: None,
            severity,
            enabled: true,
        }
    }

    pub fn schema_match(name: String, expected_columns: Vec<String>, severity: ExpectationSeverity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            expectation_type: ExpectationType::SchemaMatch { expected_columns },
            column: None,
            severity,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectationResult {
    pub expectation_id: String,
    pub expectation_name: String,
    pub expectation_type: ExpectationType,
    pub column: Option<String>,
    pub severity: ExpectationSeverity,
    pub passed: bool,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityReport {
    pub dataset_id: String,
    pub dataset_version: String,
    pub checked_at: DateTime<Utc>,
    pub total_expectations: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub info_count: usize,
    pub results: Vec<ExpectationResult>,
    pub overall_passed: bool,
}

impl DataQualityReport {
    pub fn new(dataset_id: String, dataset_version: String, results: Vec<ExpectationResult>) -> Self {
        let total_expectations = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.iter().filter(|r| !r.passed && r.severity == ExpectationSeverity::Error).count();
        let warnings = results.iter().filter(|r| !r.passed && r.severity == ExpectationSeverity::Warning).count();
        let info_count = results.iter().filter(|r| !r.passed && r.severity == ExpectationSeverity::Info).count();

        Self {
            dataset_id,
            dataset_version,
            checked_at: Utc::now(),
            total_expectations,
            passed,
            failed,
            warnings,
            info_count,
            overall_passed: failed == 0,
            results,
        }
    }

    pub fn pass_rate(&self) -> f64 {
        if self.total_expectations == 0 {
            1.0
        } else {
            self.passed as f64 / self.total_expectations as f64
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Quality: {}/{} passed ({:.1}%), {} errors, {} warnings",
            self.passed, self.total_expectations, self.pass_rate() * 100.0, self.failed, self.warnings
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetVersionDiff {
    pub from_version: String,
    pub to_version: String,
    pub rows_added: i64,
    pub rows_removed: i64,
    pub columns_added: Vec<String>,
    pub columns_removed: Vec<String>,
    pub columns_type_changed: Vec<ColumnTypeChange>,
    pub schema_compatible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnTypeChange {
    pub column_name: String,
    pub from_type: String,
    pub to_type: String,
}

impl DatasetVersionDiff {
    pub fn new(
        from_version: String,
        to_version: String,
        from_rows: usize,
        to_rows: usize,
        from_profiles: &[ColumnProfile],
        to_profiles: &[ColumnProfile],
    ) -> Self {
        let rows_added = to_rows as i64 - from_rows as i64;
        let rows_removed = if to_rows < from_rows { (from_rows - to_rows) as i64 } else { 0 };

        let from_cols: HashMap<&str, &ColumnProfile> = from_profiles.iter().map(|p| (p.name.as_str(), p)).collect();
        let to_cols: HashMap<&str, &ColumnProfile> = to_profiles.iter().map(|p| (p.name.as_str(), p)).collect();

        let columns_added: Vec<String> = to_cols.keys()
            .filter(|k| !from_cols.contains_key(*k))
            .map(|k| k.to_string())
            .collect();

        let columns_removed: Vec<String> = from_cols.keys()
            .filter(|k| !to_cols.contains_key(*k))
            .map(|k| k.to_string())
            .collect();

        let columns_type_changed: Vec<ColumnTypeChange> = from_cols.iter()
            .filter_map(|(name, from_p)| {
                to_cols.get(name).and_then(|to_p| {
                    if from_p.column_type != to_p.column_type {
                        Some(ColumnTypeChange {
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

        let schema_compatible = columns_removed.is_empty() && columns_type_changed.is_empty();

        Self {
            from_version,
            to_version,
            rows_added,
            rows_removed,
            columns_added,
            columns_removed,
            columns_type_changed,
            schema_compatible,
        }
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.rows_added != 0 {
            parts.push(format!("rows: {:+}", self.rows_added));
        }
        if !self.columns_added.is_empty() {
            parts.push(format!("+{} cols", self.columns_added.len()));
        }
        if !self.columns_removed.is_empty() {
            parts.push(format!("-{} cols", self.columns_removed.len()));
        }
        if !self.columns_type_changed.is_empty() {
            parts.push(format!("~{} types changed", self.columns_type_changed.len()));
        }
        if parts.is_empty() {
            "No changes".to_string()
        } else {
            format!("v{} → v{}: {}", self.from_version, self.to_version, parts.join(", "))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetCard {
    pub summary: Option<String>,
    pub suitable_tasks: Vec<String>,
    pub unsuitable_tasks: Vec<String>,
    pub data_source: Option<DataSourceInfo>,
    pub ethics: Option<DataEthicsInfo>,
    pub limitations: Vec<String>,
    pub recommended_splits: Vec<RecommendedSplit>,
    pub citation: Option<CitationInfo>,
    pub maintenance: Option<MaintenanceInfo>,
}

impl Default for DatasetCard {
    fn default() -> Self {
        Self {
            summary: None,
            suitable_tasks: Vec::new(),
            unsuitable_tasks: Vec::new(),
            data_source: None,
            ethics: None,
            limitations: Vec::new(),
            recommended_splits: Vec::new(),
            citation: None,
            maintenance: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceInfo {
    pub collector: Option<String>,
    pub collection_method: Option<String>,
    pub time_range: Option<String>,
    pub geographic_coverage: Option<String>,
    pub original_url: Option<String>,
    pub access_instructions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEthicsInfo {
    pub contains_pii: Option<bool>,
    pub pii_fields: Vec<String>,
    pub has_bias: Option<bool>,
    pub bias_description: Option<String>,
    pub license: Option<String>,
    pub license_url: Option<String>,
    pub usage_restrictions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedSplit {
    pub split_name: String,
    pub reason: String,
    pub expected_train_ratio: Option<f64>,
    pub expected_val_ratio: Option<f64>,
    pub expected_test_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationInfo {
    pub bibtex: Option<String>,
    pub doi: Option<String>,
    pub apa: Option<String>,
    pub paper_title: Option<String>,
    pub paper_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceInfo {
    pub owner: Option<String>,
    pub contact: Option<String>,
    pub last_reviewed: Option<DateTime<Utc>>,
    pub update_frequency: Option<String>,
    pub is_deprecated: bool,
    pub deprecation_note: Option<String>,
}
