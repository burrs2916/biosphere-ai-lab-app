use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::core::{LabError, Result};
use crate::domain::experiment::aggregate::{Experiment, ExperimentId, ExperimentStatus, ExperimentSummary};
use crate::domain::experiment::metrics::{MetricPoint, MetricSeries, MetricsTimeline};
use crate::domain::experiment::repository::ExperimentRepository;
use crate::domain::experiment::ExperimentFilter;

pub struct SqliteExperimentRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteExperimentRepository {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Self::open(db_path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Self::open(":memory:")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }

    fn open(db_path: &str) -> Result<Connection> {
        if db_path != ":memory:" {
            if let Some(parent) = Path::new(db_path).parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    LabError::Custom(format!("Failed to create db directory: {}", e))
                })?;
            }
        }

        let conn = Connection::open(db_path).map_err(|e| {
            LabError::Custom(format!("Failed to open database: {}", e))
        })?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| LabError::Custom(format!("PRAGMA error: {}", e)))?;

        Self::init_schema(&conn)?;

        Ok(conn)
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS experiments (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'created',
                task_type TEXT NOT NULL,
                config_json TEXT NOT NULL,
                params_json TEXT NOT NULL DEFAULT '{}',
                tags_json TEXT NOT NULL DEFAULT '[]',
                artifacts_json TEXT NOT NULL DEFAULT '[]',
                model_id TEXT,
                dataset_id TEXT,
                dataset_version TEXT,
                error_message TEXT,
                environment_json TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS metric_points (
                experiment_id TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                step INTEGER NOT NULL,
                value REAL NOT NULL,
                timestamp TEXT NOT NULL,
                epoch INTEGER,
                PRIMARY KEY (experiment_id, metric_name, step),
                FOREIGN KEY (experiment_id) REFERENCES experiments(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_metrics_exp ON metric_points(experiment_id);
            CREATE INDEX IF NOT EXISTS idx_metrics_name ON metric_points(experiment_id, metric_name);

            CREATE TABLE IF NOT EXISTS experiment_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                experiment_id TEXT NOT NULL,
                level TEXT NOT NULL DEFAULT 'info',
                message TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (experiment_id) REFERENCES experiments(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_logs_exp ON experiment_logs(experiment_id);
            CREATE INDEX IF NOT EXISTS idx_logs_exp_time ON experiment_logs(experiment_id, timestamp);"
        ).map_err(|e| LabError::Custom(format!("Schema init error: {}", e)))?;

        conn.execute_batch("ALTER TABLE experiments ADD COLUMN environment_json TEXT").ok();
        conn.execute_batch("ALTER TABLE experiments ADD COLUMN final_metrics_json TEXT").ok();
        conn.execute_batch("ALTER TABLE experiments ADD COLUMN description TEXT").ok();
        conn.execute_batch("ALTER TABLE experiments ADD COLUMN experiment_group TEXT").ok();

        Ok(())
    }
}

#[async_trait]
impl ExperimentRepository for SqliteExperimentRepository {
    async fn save(&self, experiment: &Experiment) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let config_json = serde_json::to_string(&experiment.config)
            .map_err(|e| LabError::Custom(format!("Serialize config: {}", e)))?;
        let params_json = serde_json::to_string(&experiment.params)
            .map_err(|e| LabError::Custom(format!("Serialize params: {}", e)))?;
        let tags_json = serde_json::to_string(&experiment.tags)
            .map_err(|e| LabError::Custom(format!("Serialize tags: {}", e)))?;
        let artifacts_json = serde_json::to_string(&experiment.artifacts)
            .map_err(|e| LabError::Custom(format!("Serialize artifacts: {}", e)))?;
        let environment_json = experiment.environment.as_ref()
            .map(|e| serde_json::to_string(e).unwrap_or_default());
        let final_metrics_json = experiment.final_metrics.as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());

        conn.execute(
            "INSERT OR REPLACE INTO experiments (id, name, status, task_type, config_json, params_json, tags_json, artifacts_json, model_id, dataset_id, dataset_version, error_message, environment_json, final_metrics_json, description, experiment_group, created_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                experiment.id.as_str(),
                experiment.name,
                experiment.status.to_string(),
                experiment.task_type.to_string(),
                config_json,
                params_json,
                tags_json,
                artifacts_json,
                experiment.model_id.as_ref().map(|m| m.as_str()),
                experiment.dataset_id,
                experiment.dataset_version,
                experiment.error_message,
                environment_json,
                final_metrics_json,
                experiment.description,
                experiment.group,
                experiment.created_at.to_rfc3339(),
                experiment.updated_at.to_rfc3339(),
                experiment.completed_at.map(|t| t.to_rfc3339()),
            ],
        ).map_err(|e| LabError::Custom(format!("Save experiment error: {}", e)))?;

        Ok(())
    }

    async fn load(&self, id: &ExperimentId) -> Result<Option<Experiment>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let result = conn.query_row(
            "SELECT id, name, status, task_type, config_json, params_json, tags_json, artifacts_json, model_id, dataset_id, dataset_version, error_message, environment_json, final_metrics_json, description, experiment_group, created_at, updated_at, completed_at
             FROM experiments WHERE id = ?1",
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
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, String>(17)?,
                    row.get::<_, Option<String>>(18)?,
                ))
            },
        ).optional().map_err(|e| LabError::Custom(format!("Load experiment error: {}", e)))?;

        let (id_str, name, status_str, task_type_json, config_json, params_json, tags_json, artifacts_json, model_id_str, dataset_id, dataset_version, error_message, environment_json, final_metrics_json, description, experiment_group, created_at_str, updated_at_str, completed_at_str) = match result {
            Some(r) => r,
            None => return Ok(None),
        };

        let status = parse_status(&status_str);
        let task_type: crate::types::TaskType = serde_json::from_str(&task_type_json)
            .or_else(|_| task_type_json.trim_matches('"').parse())
            .unwrap_or(crate::types::TaskType::Custom);
        let config: crate::core::config::TrainingConfig = serde_json::from_str(&config_json)
            .map_err(|e| LabError::Custom(format!("Deserialize config: {}", e)))?;
        let params_map: HashMap<String, serde_json::Value> = serde_json::from_str(&params_json)
            .unwrap_or_default();
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let artifacts: Vec<crate::domain::experiment::ArtifactRef> = serde_json::from_str(&artifacts_json).unwrap_or_default();
        let environment: Option<crate::domain::experiment::aggregate::EnvironmentInfo> = environment_json
            .and_then(|s| serde_json::from_str(&s).ok());
        let final_metrics: Option<serde_json::Value> = final_metrics_json
            .and_then(|s| serde_json::from_str(&s).ok());

        let created_at = parse_datetime(&created_at_str);
        let updated_at = parse_datetime(&updated_at_str);
        let completed_at = completed_at_str.as_deref().and_then(parse_datetime_opt);
        let model_id = model_id_str.as_deref().map(|s| crate::domain::model::aggregate::ModelId::from_str(s));

        let metrics = Self::load_recent_metrics_from_conn(&conn, &id_str, 2000)?;

        Ok(Some(Experiment {
            id: ExperimentId::from_str(&id_str),
            name,
            status,
            task_type,
            config,
            metrics,
            params: params_map,
            tags,
            artifacts,
            model_id,
            dataset_id,
            dataset_version,
            group: experiment_group,
            environment,
            created_at,
            updated_at,
            completed_at,
            error_message,
            final_metrics,
            description,
        }))
    }

    async fn list(&self, filter: &ExperimentFilter) -> Result<Vec<ExperimentSummary>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let mut sql = String::from(
            "SELECT id, name, status, task_type, tags_json, dataset_id, dataset_version, experiment_group, created_at, updated_at FROM experiments WHERE 1=1"
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref status) = filter.status {
            sql.push_str(&format!(" AND status = ?{}", param_values.len() + 1));
            param_values.push(Box::new(status.to_string()));
        }

        if let Some(ref name_contains) = filter.name_contains {
            sql.push_str(&format!(" AND name LIKE ?{}", param_values.len() + 1));
            param_values.push(Box::new(format!("%{}%", name_contains)));
        }

        if let Some(ref task_type) = filter.task_type {
            let task_type_str = task_type.to_string();
            let task_type_quoted = format!("\"{}\"", task_type_str);
            sql.push_str(&format!(" AND (task_type = ?{} OR task_type = ?{})", param_values.len() + 1, param_values.len() + 2));
            param_values.push(Box::new(task_type_str));
            param_values.push(Box::new(task_type_quoted));
        }

        if let Some(ref group) = filter.group {
            sql.push_str(&format!(" AND experiment_group = ?{}", param_values.len() + 1));
            param_values.push(Box::new(group.clone()));
        }

        sql.push_str(" ORDER BY updated_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT ?{}", param_values.len() + 1));
            param_values.push(Box::new(limit as i64));
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| LabError::Custom(format!("Prepare list error: {}", e)))?;

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        }).map_err(|e| LabError::Custom(format!("Query list error: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let (id_str, name, status_str, task_type_json, tags_json, dataset_id, dataset_version, experiment_group, created_at_str, updated_at_str) = row
                .map_err(|e| LabError::Custom(format!("Row error: {}", e)))?;

            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            if !filter.tags.is_empty() {
                if !filter.tags.iter().all(|t| tags.contains(t)) {
                    continue;
                }
            }

            let status = parse_status(&status_str);
            let task_type: crate::types::TaskType = serde_json::from_str(&task_type_json)
                .or_else(|_| task_type_json.trim_matches('"').parse())
                .unwrap_or(crate::types::TaskType::Custom);
            let created_at = parse_datetime(&created_at_str);
            let updated_at = parse_datetime(&updated_at_str);

            let metric_names = Self::get_metric_names_static(&conn, &id_str)?;
            let best_metrics = Self::get_best_metrics_static(&conn, &id_str, &metric_names)?;

            results.push(ExperimentSummary {
                id: ExperimentId::from_str(&id_str),
                name,
                status,
                task_type,
                tags,
                dataset_id,
                dataset_version,
                group: experiment_group,
                created_at,
                updated_at,
                metric_names,
                best_metrics,
            });
        }

        Ok(results)
    }

    async fn delete(&self, id: &ExperimentId) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        conn.execute_batch("BEGIN TRANSACTION")
            .map_err(|e| LabError::Custom(format!("Begin transaction error: {}", e)))?;

        let delete_result: Result<()> = (|| {
            conn.execute("DELETE FROM metric_points WHERE experiment_id = ?1", [id.as_str()])
                .map_err(|e| LabError::Custom(format!("Delete metrics error: {}", e)))?;
            conn.execute("DELETE FROM experiment_logs WHERE experiment_id = ?1", [id.as_str()])
                .map_err(|e| LabError::Custom(format!("Delete logs error: {}", e)))?;
            conn.execute("DELETE FROM experiments WHERE id = ?1", [id.as_str()])
                .map_err(|e| LabError::Custom(format!("Delete experiment error: {}", e)))?;
            Ok(())
        })();

        if delete_result.is_err() {
            let _ = conn.execute_batch("ROLLBACK");
        } else {
            conn.execute_batch("COMMIT")
                .map_err(|e| LabError::Custom(format!("Commit transaction error: {}", e)))?;
        }

        delete_result?;

        let artifact_dir = crate::core::config::get_artifact_dir(id.as_str());
        let _ = std::fs::remove_dir_all(&artifact_dir);

        Ok(())
    }

    async fn query_metrics(
        &self,
        id: &ExperimentId,
        metric_names: &[String],
    ) -> Result<MetricsTimeline> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        let mut timeline = MetricsTimeline::new();

        for name in metric_names {
            let mut stmt = conn.prepare(
                "SELECT step, value, timestamp, epoch FROM metric_points WHERE experiment_id = ?1 AND metric_name = ?2 ORDER BY step"
            ).map_err(|e| LabError::Custom(format!("Prepare metrics query: {}", e)))?;

            let points: Vec<MetricPoint> = stmt.query_map(
                params![id.as_str(), name],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            ).map_err(|e| LabError::Custom(format!("Query metrics error: {}", e)))?
            .filter_map(|r| r.ok())
            .map(|(step, value, ts, epoch)| MetricPoint {
                step: step as u64,
                value,
                timestamp: parse_datetime(&ts),
                epoch: epoch.map(|e| e as usize),
            })
            .collect();

            if !points.is_empty() {
                let mut series = MetricSeries::new(name.clone());
                series.values = points;
                timeline.insert_series(name.clone(), series);
            }
        }

        Ok(timeline)
    }

    async fn exists(&self, id: &ExperimentId) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM experiments WHERE id = ?1",
            [id.as_str()],
            |row| row.get(0),
        ).map_err(|e| LabError::Custom(format!("Exists check error: {}", e)))?;

        Ok(count > 0)
    }

    async fn save_metric_point(
        &self,
        experiment_id: &ExperimentId,
        metric_name: &str,
        step: u64,
        value: f64,
        epoch: Option<usize>,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO metric_points (experiment_id, metric_name, step, value, timestamp, epoch)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                experiment_id.as_str(),
                metric_name,
                step as i64,
                value,
                now,
                epoch.map(|e| e as i64),
            ],
        ).map_err(|e| LabError::Custom(format!("Save metric point error: {}", e)))?;

        Ok(())
    }

    async fn update_status(
        &self,
        experiment_id: &ExperimentId,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let now = chrono::Utc::now().to_rfc3339();
        let status_lower = status.to_lowercase();

        if status_lower == "completed" || status_lower == "failed" || status_lower == "cancelled" {
            conn.execute(
                "UPDATE experiments SET status = ?1, updated_at = ?2, completed_at = ?2, error_message = ?3 WHERE id = ?4",
                params![status_lower, now, error_message, experiment_id.as_str()],
            ).map_err(|e| LabError::Custom(format!("Update status error: {}", e)))?;
        } else {
            conn.execute(
                "UPDATE experiments SET status = ?1, updated_at = ?2, error_message = ?3 WHERE id = ?4",
                params![status_lower, now, error_message, experiment_id.as_str()],
            ).map_err(|e| LabError::Custom(format!("Update status error: {}", e)))?;
        }

        Ok(())
    }

    async fn save_environment(
        &self,
        experiment_id: &ExperimentId,
        environment: &crate::domain::experiment::aggregate::EnvironmentInfo,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let environment_json = serde_json::to_string(environment)
            .map_err(|e| LabError::Custom(format!("Serialize environment: {}", e)))?;

        conn.execute(
            "UPDATE experiments SET environment_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![environment_json, chrono::Utc::now().to_rfc3339(), experiment_id.as_str()],
        ).map_err(|e| LabError::Custom(format!("Save environment error: {}", e)))?;

        Ok(())
    }

    async fn save_log(
        &self,
        experiment_id: &ExperimentId,
        level: &str,
        message: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        conn.execute(
            "INSERT INTO experiment_logs (experiment_id, level, message, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![experiment_id.as_str(), level, message, chrono::Utc::now().to_rfc3339()],
        ).map_err(|e| LabError::Custom(format!("Save log error: {}", e)))?;
        Ok(())
    }

    async fn load_logs(
        &self,
        experiment_id: &ExperimentId,
        limit: usize,
    ) -> Result<Vec<crate::domain::experiment::aggregate::LogEntry>> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT level, message, timestamp FROM experiment_logs WHERE experiment_id = ?1 ORDER BY id DESC LIMIT ?2"
        ).map_err(|e| LabError::Custom(format!("Prepare logs: {}", e)))?;

        let mut logs: Vec<crate::domain::experiment::aggregate::LogEntry> = stmt.query_map(
            params![experiment_id.as_str(), limit as i64],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        ).map_err(|e| LabError::Custom(format!("Query logs error: {}", e)))?
        .filter_map(|r| r.ok())
        .map(|(level, message, ts)| crate::domain::experiment::aggregate::LogEntry {
            level,
            message,
            timestamp: parse_datetime(&ts),
        })
        .collect();

        logs.reverse();
        Ok(logs)
    }
}

impl SqliteExperimentRepository {
    fn load_recent_metrics_from_conn(conn: &Connection, experiment_id: &str, limit: usize) -> Result<MetricsTimeline> {
        let mut timeline = MetricsTimeline::new();

        let metric_names = Self::get_metric_names_static(conn, experiment_id)?;

        for name in &metric_names {
            let mut stmt = conn.prepare(
                "SELECT step, value, timestamp, epoch FROM metric_points WHERE experiment_id = ?1 AND metric_name = ?2 ORDER BY step DESC LIMIT ?3"
            ).map_err(|e| LabError::Custom(format!("Prepare recent metrics: {}", e)))?;

            let mut points: Vec<MetricPoint> = stmt.query_map(
                params![experiment_id, name, limit as i64],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            ).map_err(|e| LabError::Custom(format!("Query recent metrics: {}", e)))?
            .filter_map(|r| r.ok())
            .map(|(step, value, ts, epoch)| MetricPoint {
                step: step as u64,
                value,
                timestamp: parse_datetime(&ts),
                epoch: epoch.map(|e| e as usize),
            })
            .collect();

            points.reverse();

            if !points.is_empty() {
                let mut series = MetricSeries::new(name.clone());
                series.values = points;
                timeline.insert_series(name.clone(), series);
            }
        }

        Ok(timeline)
    }

    #[allow(dead_code)]
    fn load_metrics_from_conn(conn: &Connection, experiment_id: &str) -> Result<MetricsTimeline> {
        let mut timeline = MetricsTimeline::new();

        let metric_names = Self::get_metric_names_static(conn, experiment_id)?;

        for name in &metric_names {
            let mut stmt = conn.prepare(
                "SELECT step, value, timestamp, epoch FROM metric_points WHERE experiment_id = ?1 AND metric_name = ?2 ORDER BY step"
            ).map_err(|e| LabError::Custom(format!("Prepare metrics: {}", e)))?;

            let points: Vec<MetricPoint> = stmt.query_map(
                params![experiment_id, name],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            ).map_err(|e| LabError::Custom(format!("Query metrics: {}", e)))?
            .filter_map(|r| r.ok())
            .map(|(step, value, ts, epoch)| MetricPoint {
                step: step as u64,
                value,
                timestamp: parse_datetime(&ts),
                epoch: epoch.map(|e| e as usize),
            })
            .collect();

            if !points.is_empty() {
                let mut series = MetricSeries::new(name.clone());
                series.values = points;
                timeline.insert_series(name.clone(), series);
            }
        }

        Ok(timeline)
    }

    fn get_metric_names_static(conn: &Connection, experiment_id: &str) -> Result<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT metric_name FROM metric_points WHERE experiment_id = ?1 ORDER BY metric_name"
        ).map_err(|e| LabError::Custom(format!("Prepare metric names: {}", e)))?;

        let names: Vec<String> = stmt.query_map([experiment_id], |row| row.get(0))
            .map_err(|e| LabError::Custom(format!("Query metric names: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(names)
    }

    fn get_best_metrics_static(conn: &Connection, experiment_id: &str, metric_names: &[String]) -> Result<HashMap<String, f64>> {
        let mut best = HashMap::new();

        for name in metric_names {
            let is_loss_metric = name.contains("loss") || name.contains("error") || name.contains("mse") || name.contains("rmse") || name.contains("mae");
            let agg = if is_loss_metric { "MIN" } else { "MAX" };

            let result: Option<f64> = conn.query_row(
                &format!("SELECT {agg}(value) FROM metric_points WHERE experiment_id = ?1 AND metric_name = ?2"),
                params![experiment_id, name],
                |row| row.get(0),
            ).optional().map_err(|e| LabError::Custom(format!("Best metric query: {}", e)))?;

            if let Some(v) = result {
                best.insert(name.clone(), v);
            }
        }

        Ok(best)
    }
}

fn parse_status(s: &str) -> ExperimentStatus {
    match s.to_lowercase().as_str() {
        "running" => ExperimentStatus::Running,
        "completed" => ExperimentStatus::Completed,
        "paused" => ExperimentStatus::Paused,
        "failed" => ExperimentStatus::Failed,
        "cancelled" => ExperimentStatus::Cancelled,
        "archived" => ExperimentStatus::Archived,
        "created" => ExperimentStatus::Created,
        _ => ExperimentStatus::Created,
    }
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_datetime_opt(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}
