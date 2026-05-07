use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,

    #[serde(default = "default_log_file")]
    pub log_file: String,

    #[serde(default = "default_true")]
    pub clear_on_start: bool,

    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default = "default_true")]
    pub console_output: bool,

    #[serde(default = "default_max_age_minutes")]
    pub max_age_minutes: u64,
}

fn default_log_dir() -> PathBuf {
    PathBuf::from("logs")
}

fn default_log_file() -> String {
    "lab.log".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

fn default_max_age_minutes() -> u64 {
    3
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            log_dir: default_log_dir(),
            log_file: default_log_file(),
            clear_on_start: true,
            level: default_log_level(),
            console_output: true,
            max_age_minutes: default_max_age_minutes(),
        }
    }
}
