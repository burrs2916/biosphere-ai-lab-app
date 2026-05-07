use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use crate::core::{LabError, Result};
use crate::domain::settings::AppSettings;

pub struct SqliteSettingsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSettingsRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        {
            let guard = conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
            Self::init_schema(&guard)?;
        }
        Ok(Self { conn })
    }

    pub fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        ).map_err(|e| LabError::Custom(format!("Settings schema init error: {}", e)))?;
        Ok(())
    }

    pub fn load_all(&self) -> Result<AppSettings> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let mut map = HashMap::new();
        let mut stmt = conn.prepare("SELECT key, value FROM app_settings")
            .map_err(|e| LabError::Custom(format!("Prepare settings error: {}", e)))?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| LabError::Custom(format!("Query settings error: {}", e)))?;

        for row in rows {
            let (key, value) = row.map_err(|e| LabError::Custom(format!("Row error: {}", e)))?;
            map.insert(key, value);
        }

        Ok(AppSettings::from_flat_map(&map))
    }

    pub fn save_all(&self, settings: &AppSettings) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;

        let map = settings.to_flat_map();
        for (key, value) in &map {
            conn.execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            ).map_err(|e| LabError::Custom(format!("Save setting error: {}", e)))?;
        }

        Ok(())
    }

    pub fn save_key(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| LabError::Custom(format!("Lock error: {}", e)))?;
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        ).map_err(|e| LabError::Custom(format!("Save setting error: {}", e)))?;
        Ok(())
    }
}
