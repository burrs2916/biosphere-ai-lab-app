use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use crate::core::{LabError, Result};
use crate::domain::dataset::aggregate::{ColumnProfile, Dataset, DatasetFilter, DatasetId, DatasetSplit, DatasetStatus, DatasetSummary, SplitStrategy};
use crate::domain::dataset::repository::DatasetRepository;
use crate::types::DataFormat;

pub struct SqliteDatasetRepository {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteDatasetRepository {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_schema(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS datasets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                format TEXT NOT NULL,
                path TEXT NOT NULL,
                digest TEXT NOT NULL,
                rows INTEGER NOT NULL,
                columns INTEGER NOT NULL,
                column_profiles_json TEXT NOT NULL DEFAULT '[]',
                memory_size_mb REAL NOT NULL DEFAULT 0.0,
                tags_json TEXT NOT NULL DEFAULT '[]',
                description TEXT,
                source_type TEXT,
                source_uri TEXT,
                experiment_ids_json TEXT NOT NULL DEFAULT '[]',
                metadata_json TEXT NOT NULL DEFAULT '{}',
                version_history_json TEXT NOT NULL DEFAULT '[]',
                card_json TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );"
        ).map_err(|e| LabError::Custom(format!("Dataset schema init error: {}", e)))?;

        conn.execute_batch("ALTER TABLE datasets ADD COLUMN version_history_json TEXT NOT NULL DEFAULT '[]'").ok();
        conn.execute_batch("ALTER TABLE datasets ADD COLUMN card_json TEXT").ok();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dataset_splits (
                dataset_id TEXT NOT NULL,
                name TEXT NOT NULL,
                strategy TEXT NOT NULL,
                train_ratio REAL NOT NULL,
                val_ratio REAL NOT NULL,
                test_ratio REAL NOT NULL,
                seed INTEGER NOT NULL,
                stratify_column TEXT,
                group_column TEXT,
                train_indices_json TEXT NOT NULL,
                val_indices_json TEXT NOT NULL,
                test_indices_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (dataset_id, name),
                FOREIGN KEY (dataset_id) REFERENCES datasets(id) ON DELETE CASCADE
            );"
        ).map_err(|e| LabError::Custom(format!("Dataset splits schema init error: {}", e)))?;

        Ok(())
    }
}

const SELECT_COLS: &str = "id, name, version, status, format, path, digest, rows, columns, column_profiles_json, memory_size_mb, tags_json, COALESCE(description, ''), COALESCE(source_type, ''), COALESCE(source_uri, ''), experiment_ids_json, metadata_json, version_history_json, card_json, created_at, updated_at";

fn row_to_dataset(
    id_str: String,
    name: String,
    version: String,
    status_str: String,
    format_str: String,
    path: String,
    digest: String,
    rows: usize,
    columns: usize,
    column_profiles_json: String,
    memory_size_mb: f64,
    tags_json: String,
    description: Option<String>,
    source_type: Option<String>,
    source_uri: Option<String>,
    experiment_ids_json: String,
    metadata_json: String,
    version_history_json: String,
    card_json: Option<String>,
    created_at_str: String,
    updated_at_str: String,
) -> Dataset {
    let status = status_str.parse::<DatasetStatus>().unwrap_or(DatasetStatus::Active);
    let format = match format_str.as_str() {
        "csv" => DataFormat::Csv,
        "json" => DataFormat::Json,
        "image" => DataFormat::Image,
        "text" => DataFormat::Text,
        "binary" => DataFormat::Binary,
        "parquet" => DataFormat::Parquet,
        "excel" => DataFormat::Excel,
        "tfrecord" => DataFormat::TfRecord,
        "huggingface" => DataFormat::HuggingFace,
        "database" => DataFormat::Database,
        _ => DataFormat::Csv,
    };
    let column_profiles: Vec<ColumnProfile> = serde_json::from_str(&column_profiles_json).unwrap_or_default();
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let experiment_ids: Vec<String> = serde_json::from_str(&experiment_ids_json).unwrap_or_default();
    let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap_or_default();
    let version_history: Vec<crate::domain::dataset::DatasetVersionRecord> = serde_json::from_str(&version_history_json).unwrap_or_default();
    let card: Option<crate::domain::dataset::aggregate::DatasetCard> = card_json.as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Dataset {
        id: DatasetId::from_str(&id_str),
        name,
        version: crate::domain::dataset::aggregate::DatasetVersion::from_str(&version),
        status,
        format,
        path,
        digest,
        rows,
        columns,
        column_profiles,
        memory_size_mb,
        tags,
        description: description.filter(|d| !d.is_empty()),
        source_type: source_type.filter(|s| !s.is_empty()),
        source_uri: source_uri.filter(|s| !s.is_empty()),
        experiment_ids,
        metadata,
        card,
        version_history,
        created_at,
        updated_at,
    }
}

#[async_trait]
impl DatasetRepository for SqliteDatasetRepository {
    async fn save(&self, dataset: &Dataset) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let column_profiles_json = serde_json::to_string(&dataset.column_profiles)
            .map_err(|e| LabError::Custom(format!("Serialize column_profiles: {}", e)))?;
        let tags_json = serde_json::to_string(&dataset.tags)
            .map_err(|e| LabError::Custom(format!("Serialize tags: {}", e)))?;
        let experiment_ids_json = serde_json::to_string(&dataset.experiment_ids)
            .map_err(|e| LabError::Custom(format!("Serialize experiment_ids: {}", e)))?;
        let metadata_json = serde_json::to_string(&dataset.metadata)
            .map_err(|e| LabError::Custom(format!("Serialize metadata: {}", e)))?;
        let version_history_json = serde_json::to_string(&dataset.version_history)
            .map_err(|e| LabError::Custom(format!("Serialize version_history: {}", e)))?;
        let card_json = dataset.card.as_ref()
            .map(|c| serde_json::to_string(c))
            .transpose()
            .map_err(|e| LabError::Custom(format!("Serialize card: {}", e)))?;

        conn.execute(
            &format!("INSERT OR REPLACE INTO datasets ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)", SELECT_COLS),
            params![
                dataset.id.as_str(),
                dataset.name,
                dataset.version.to_string(),
                dataset.status.to_string(),
                dataset.format.to_string(),
                dataset.path,
                dataset.digest,
                dataset.rows,
                dataset.columns,
                column_profiles_json,
                dataset.memory_size_mb,
                tags_json,
                dataset.description,
                dataset.source_type,
                dataset.source_uri,
                experiment_ids_json,
                metadata_json,
                version_history_json,
                card_json,
                dataset.created_at.to_rfc3339(),
                dataset.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| LabError::Custom(format!("Save dataset error: {}", e)))?;

        Ok(())
    }

    async fn load(&self, id: &DatasetId) -> Result<Option<Dataset>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let result = conn.query_row(
            &format!("SELECT {} FROM datasets WHERE id = ?1", SELECT_COLS),
            [id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, usize>(7)?,
                    row.get::<_, usize>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, f64>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, String>(17)?,
                    row.get::<_, Option<String>>(18)?,
                    row.get::<_, String>(19)?,
                    row.get::<_, String>(20)?,
                ))
            },
        ).optional().map_err(|e| LabError::Custom(format!("Load dataset error: {}", e)))?;

        match result {
            Some((id_str, name, version, status_str, format_str, path, digest, rows, columns, column_profiles_json, memory_size_mb, tags_json, description, source_type, source_uri, experiment_ids_json, metadata_json, version_history_json, card_json, created_at_str, updated_at_str)) => {
                Ok(Some(row_to_dataset(id_str, name, version, status_str, format_str, path, digest, rows, columns, column_profiles_json, memory_size_mb, tags_json, description, source_type, source_uri, experiment_ids_json, metadata_json, version_history_json, card_json, created_at_str, updated_at_str)))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, filter: &DatasetFilter) -> Result<Vec<DatasetSummary>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let mut sql = String::from("SELECT id, name, version, status, format, rows, columns, column_profiles_json, memory_size_mb, tags_json, experiment_ids_json, created_at, updated_at FROM datasets WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(ref status) = filter.status {
            sql.push_str(&format!(" AND status = ?{}", param_idx));
            param_values.push(Box::new(status.to_string()));
            param_idx += 1;
        }

        if let Some(ref format) = filter.format {
            sql.push_str(&format!(" AND format = ?{}", param_idx));
            param_values.push(Box::new(format.to_string()));
            param_idx += 1;
        }

        if let Some(ref name_contains) = filter.name_contains {
            sql.push_str(&format!(" AND name LIKE ?{}", param_idx));
            param_values.push(Box::new(format!("%{}%", name_contains)));
            param_idx += 1;
        }

        sql.push_str(" ORDER BY updated_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT ?{}", param_idx));
            param_values.push(Box::new(limit as i64));
            param_idx += 1;
            if let Some(offset) = filter.offset {
                sql.push_str(&format!(" OFFSET ?{}", param_idx));
                param_values.push(Box::new(offset as i64));
            }
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| LabError::Custom(format!("Prepare dataset list: {}", e)))?;

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, usize>(5)?,
                row.get::<_, usize>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, f64>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
            ))
        }).map_err(|e| LabError::Custom(format!("Query datasets: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let (id_str, name, version, status_str, format_str, rows_count, columns_count, column_profiles_json, memory_size_mb, tags_json, experiment_ids_json, created_at_str, updated_at_str) = row
                .map_err(|e| LabError::Custom(format!("Dataset row error: {}", e)))?;

            let status = status_str.parse::<DatasetStatus>().unwrap_or(DatasetStatus::Active);
            let format = match format_str.as_str() {
                "csv" => DataFormat::Csv,
                "json" => DataFormat::Json,
                "image" => DataFormat::Image,
                "text" => DataFormat::Text,
                "binary" => DataFormat::Binary,
                "parquet" => DataFormat::Parquet,
                "excel" => DataFormat::Excel,
                "tfrecord" => DataFormat::TfRecord,
                "huggingface" => DataFormat::HuggingFace,
                "database" => DataFormat::Database,
                _ => DataFormat::Csv,
            };
            let column_profiles: Vec<ColumnProfile> = serde_json::from_str(&column_profiles_json).unwrap_or_default();
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let experiment_ids: Vec<String> = serde_json::from_str(&experiment_ids_json).unwrap_or_default();
            let has_missing = column_profiles.iter().any(|p| p.null_count > 0);

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(DatasetSummary {
                id: DatasetId::from_str(&id_str),
                name,
                version,
                status,
                format,
                rows: rows_count,
                columns: columns_count,
                has_missing_values: has_missing,
                memory_size_mb,
                tags,
                experiment_count: experiment_ids.len(),
                created_at,
                updated_at,
            });
        }

        Ok(results)
    }

    async fn delete(&self, id: &DatasetId) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        conn.execute("DELETE FROM datasets WHERE id = ?1", [id.as_str()])
            .map_err(|e| LabError::Custom(format!("Delete dataset error: {}", e)))?;
        Ok(())
    }

    async fn exists(&self, id: &DatasetId) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM datasets WHERE id = ?1",
            [id.as_str()],
            |row| row.get(0),
        ).map_err(|e| LabError::Custom(format!("Dataset exists check: {}", e)))?;
        Ok(count > 0)
    }

    async fn find_by_digest(&self, digest: &str) -> Result<Option<Dataset>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let result = conn.query_row(
            &format!("SELECT {} FROM datasets WHERE digest = ?1 AND status != 'deleted' ORDER BY updated_at DESC LIMIT 1", SELECT_COLS),
            [digest],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, usize>(7)?,
                    row.get::<_, usize>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, f64>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, String>(17)?,
                    row.get::<_, Option<String>>(18)?,
                    row.get::<_, String>(19)?,
                    row.get::<_, String>(20)?,
                ))
            },
        ).optional().map_err(|e| LabError::Custom(format!("Find by digest error: {}", e)))?;

        match result {
            Some((id_str, name, version, status_str, format_str, path, digest, rows, columns, column_profiles_json, memory_size_mb, tags_json, description, source_type, source_uri, experiment_ids_json, metadata_json, version_history_json, card_json, created_at_str, updated_at_str)) => {
                Ok(Some(row_to_dataset(id_str, name, version, status_str, format_str, path, digest, rows, columns, column_profiles_json, memory_size_mb, tags_json, description, source_type, source_uri, experiment_ids_json, metadata_json, version_history_json, card_json, created_at_str, updated_at_str)))
            }
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Vec<DatasetSummary>> {
        self.list(&DatasetFilter {
            name_contains: Some(name.to_string()),
            ..DatasetFilter::default()
        }).await
    }

    async fn save_split(&self, dataset_id: &DatasetId, split: &DatasetSplit) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let train_json = serde_json::to_string(&split.train_indices)
            .map_err(|e| LabError::Custom(format!("Serialize train_indices: {}", e)))?;
        let val_json = serde_json::to_string(&split.val_indices)
            .map_err(|e| LabError::Custom(format!("Serialize val_indices: {}", e)))?;
        let test_json = serde_json::to_string(&split.test_indices)
            .map_err(|e| LabError::Custom(format!("Serialize test_indices: {}", e)))?;

        conn.execute(
            "INSERT OR REPLACE INTO dataset_splits (dataset_id, name, strategy, train_ratio, val_ratio, test_ratio, seed, stratify_column, group_column, train_indices_json, val_indices_json, test_indices_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                dataset_id.as_str(),
                split.name,
                split.strategy.to_string(),
                split.train_ratio,
                split.val_ratio,
                split.test_ratio,
                split.seed as i64,
                split.stratify_column,
                split.group_column,
                train_json,
                val_json,
                test_json,
                split.created_at.to_rfc3339(),
            ],
        ).map_err(|e| LabError::Custom(format!("Save split error: {}", e)))?;

        Ok(())
    }

    async fn load_splits(&self, dataset_id: &DatasetId) -> Result<Vec<DatasetSplit>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let mut stmt = conn.prepare(
            "SELECT name, strategy, train_ratio, val_ratio, test_ratio, seed, stratify_column, group_column, train_indices_json, val_indices_json, test_indices_json, created_at FROM dataset_splits WHERE dataset_id = ?1 ORDER BY created_at"
        ).map_err(|e| LabError::Custom(format!("Prepare load splits: {}", e)))?;

        let rows = stmt.query_map([dataset_id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
            ))
        }).map_err(|e| LabError::Custom(format!("Query splits: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let (name, strategy_str, train_ratio, val_ratio, test_ratio, seed, stratify_column, group_column, train_json, val_json, test_json, created_at_str) = row
                .map_err(|e| LabError::Custom(format!("Split row error: {}", e)))?;

            let strategy = strategy_str.parse::<SplitStrategy>().unwrap_or(SplitStrategy::Random);
            let train_indices: Vec<usize> = serde_json::from_str(&train_json).unwrap_or_default();
            let val_indices: Vec<usize> = serde_json::from_str(&val_json).unwrap_or_default();
            let test_indices: Vec<usize> = serde_json::from_str(&test_json).unwrap_or_default();
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(DatasetSplit {
                name,
                strategy,
                train_ratio,
                val_ratio,
                test_ratio,
                seed: seed as u64,
                stratify_column,
                group_column,
                train_indices,
                val_indices,
                test_indices,
                created_at,
            });
        }

        Ok(results)
    }

    async fn load_split(&self, dataset_id: &DatasetId, name: &str) -> Result<Option<DatasetSplit>> {
        let splits = self.load_splits(dataset_id).await?;
        Ok(splits.into_iter().find(|s| s.name == name))
    }

    async fn delete_split(&self, dataset_id: &DatasetId, name: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        conn.execute(
            "DELETE FROM dataset_splits WHERE dataset_id = ?1 AND name = ?2",
            params![dataset_id.as_str(), name],
        ).map_err(|e| LabError::Custom(format!("Delete split error: {}", e)))?;
        Ok(())
    }
}
