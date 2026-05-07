use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use crate::core::{LabError, Result};
use crate::domain::model::aggregate::{ModelId, ModelRegistration, ModelStatus};
use crate::domain::model::repository::ModelRepository;

pub struct SqliteModelRepository {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteModelRepository {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_schema(conn: &rusqlite::Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS models (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'none',
                framework TEXT NOT NULL,
                path TEXT,
                signature_json TEXT,
                structured_signature_json TEXT,
                lineage_json TEXT,
                metadata_json TEXT NOT NULL DEFAULT '{}',
                description TEXT,
                tags_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );"
        ).map_err(|e| LabError::Custom(format!("Model schema init error: {}", e)))?;

        conn.execute_batch("ALTER TABLE models ADD COLUMN structured_signature_json TEXT").ok();
        conn.execute_batch("ALTER TABLE models ADD COLUMN lineage_json TEXT").ok();
        conn.execute_batch("ALTER TABLE models ADD COLUMN description TEXT").ok();
        conn.execute_batch("ALTER TABLE models ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]'").ok();
        conn.execute_batch("ALTER TABLE models ADD COLUMN aliases_json TEXT NOT NULL DEFAULT '[]'").ok();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS serving_endpoints (
                model_id TEXT PRIMARY KEY,
                deployed_at TEXT NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                total_latency_ms REAL NOT NULL DEFAULT 0.0,
                status TEXT NOT NULL DEFAULT 'active'
            );"
        ).map_err(|e| LabError::Custom(format!("Serving schema init error: {}", e)))?;

        Ok(())
    }
}

fn row_to_model(
    id_str: String,
    name: String,
    version: String,
    status_str: String,
    framework: String,
    path: Option<String>,
    signature_json: Option<String>,
    structured_signature_json: Option<String>,
    lineage_json: Option<String>,
    metadata_json: String,
    description: Option<String>,
    tags_json: String,
    aliases_json: String,
    created_at_str: String,
    updated_at_str: String,
) -> ModelRegistration {
    let status = match status_str.as_str() {
        "none" => ModelStatus::None,
        "staging" => ModelStatus::Staging,
        "production" => ModelStatus::Production,
        "archived" => ModelStatus::Archived,
        _ => ModelStatus::None,
    };

    let signature = signature_json.and_then(|s| serde_json::from_str(&s).ok());
    let structured_signature = structured_signature_json.and_then(|s| serde_json::from_str(&s).ok());
    let lineage = lineage_json.and_then(|s| serde_json::from_str(&s).ok());
    let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata_json).unwrap_or_default();
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let aliases: Vec<String> = serde_json::from_str(&aliases_json).unwrap_or_default();

    let created_at = DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    ModelRegistration {
        id: ModelId::from_str(&id_str),
        name,
        version,
        status,
        framework,
        path,
        signature,
        structured_signature,
        lineage,
        metadata,
        description,
        tags,
        aliases,
        created_at,
        updated_at,
    }
}

const SELECT_COLS: &str = "id, name, version, status, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at, updated_at";

const INSERT_COLS: &str = "id, name, version, status, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at, updated_at";

#[async_trait]
impl ModelRepository for SqliteModelRepository {
    async fn save(&self, model: &ModelRegistration) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let signature_json = model.signature.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let structured_signature_json = model.structured_signature.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let lineage_json = model.lineage.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let metadata_json = serde_json::to_string(&model.metadata)
            .map_err(|e| LabError::Custom(format!("Serialize metadata: {}", e)))?;
        let tags_json = serde_json::to_string(&model.tags)
            .map_err(|e| LabError::Custom(format!("Serialize tags: {}", e)))?;
        let aliases_json = serde_json::to_string(&model.aliases)
            .map_err(|e| LabError::Custom(format!("Serialize aliases: {}", e)))?;

        conn.execute(
            &format!("INSERT OR REPLACE INTO models ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)", INSERT_COLS),
            params![
                model.id.as_str(),
                model.name,
                model.version,
                model.status.to_string(),
                model.framework,
                model.path,
                signature_json,
                structured_signature_json,
                lineage_json,
                metadata_json,
                model.description,
                tags_json,
                aliases_json,
                model.created_at.to_rfc3339(),
                model.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| LabError::Custom(format!("Save model error: {}", e)))?;

        Ok(())
    }

    async fn load(&self, id: &ModelId) -> Result<Option<ModelRegistration>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let result = conn.query_row(
            &format!("SELECT {} FROM models WHERE id = ?1", SELECT_COLS),
            [id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                ))
            },
        ).optional().map_err(|e| LabError::Custom(format!("Load model error: {}", e)))?;

        match result {
            Some((id_str, name, version, status_str, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at_str, updated_at_str)) => {
                Ok(Some(row_to_model(id_str, name, version, status_str, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at_str, updated_at_str)))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, status: Option<ModelStatus>) -> Result<Vec<ModelRegistration>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let mut results = Vec::new();

        if let Some(ref s) = status {
            let sql = format!("SELECT {} FROM models WHERE status = ?1 ORDER BY updated_at DESC", SELECT_COLS);
            let mut stmt = conn.prepare(&sql)
                .map_err(|e| LabError::Custom(format!("Prepare model list: {}", e)))?;
            let rows = stmt.query_map(params![s.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                ))
            }).map_err(|e| LabError::Custom(format!("Query models: {}", e)))?;

            for row in rows {
                let (id_str, name, version, status_str, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at_str, updated_at_str) = row
                    .map_err(|e| LabError::Custom(format!("Model row error: {}", e)))?;
                results.push(row_to_model(id_str, name, version, status_str, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at_str, updated_at_str));
            }
        } else {
            let sql = format!("SELECT {} FROM models ORDER BY updated_at DESC", SELECT_COLS);
            let mut stmt = conn.prepare(&sql)
                .map_err(|e| LabError::Custom(format!("Prepare model list: {}", e)))?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                ))
            }).map_err(|e| LabError::Custom(format!("Query models: {}", e)))?;

            for row in rows {
                let (id_str, name, version, status_str, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at_str, updated_at_str) = row
                    .map_err(|e| LabError::Custom(format!("Model row error: {}", e)))?;
                results.push(row_to_model(id_str, name, version, status_str, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at_str, updated_at_str));
            }
        }

        Ok(results)
    }

    async fn list_by_name(&self, name: &str) -> Result<Vec<ModelRegistration>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let sql = format!("SELECT {} FROM models WHERE name = ?1 ORDER BY version DESC", SELECT_COLS);

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| LabError::Custom(format!("Prepare model list by name: {}", e)))?;

        let rows = stmt.query_map(params![name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
            ))
        }).map_err(|e| LabError::Custom(format!("Query models by name: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let (id_str, name, version, status_str, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at_str, updated_at_str) = row
                .map_err(|e| LabError::Custom(format!("Model row error: {}", e)))?;
            results.push(row_to_model(id_str, name, version, status_str, framework, path, signature_json, structured_signature_json, lineage_json, metadata_json, description, tags_json, aliases_json, created_at_str, updated_at_str));
        }

        Ok(results)
    }

    async fn delete(&self, id: &ModelId) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        conn.execute("DELETE FROM models WHERE id = ?1", [id.as_str()])
            .map_err(|e| LabError::Custom(format!("Delete model error: {}", e)))?;
        Ok(())
    }

    async fn exists(&self, id: &ModelId) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM models WHERE id = ?1",
            [id.as_str()],
            |row| row.get(0),
        ).map_err(|e| LabError::Custom(format!("Model exists check: {}", e)))?;
        Ok(count > 0)
    }
}
