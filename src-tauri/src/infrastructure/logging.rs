use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::Local;
use crate::infrastructure::config::LogConfig;

pub struct Logger {
    log_path: Option<PathBuf>,
    console_output: bool,
}

impl Logger {
    pub fn new(app_dir: &PathBuf, config: &LogConfig) -> Self {
        let log_path = if config.log_dir.is_absolute() {
            Some(config.log_dir.join(&config.log_file))
        } else {
            Some(app_dir.join(&config.log_dir).join(&config.log_file))
        };

        if let Some(ref path) = log_path {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).ok();
                }
            }

            if config.clear_on_start && path.exists() {
                if let Err(e) = fs::remove_file(path) {
                    eprintln!("[Logger] Failed to clear log file: {}", e);
                }
            }
        }

        Self {
            log_path,
            console_output: config.console_output,
        }
    }

    pub fn log(&self, category: &str, message: &str, data: Option<&str>) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_entry = match data {
            Some(d) => format!("[{}][{}] {} | data: {}\n", timestamp, category, message, d),
            None => format!("[{}][{}] {}\n", timestamp, category, message),
        };

        if let Some(ref path) = self.log_path {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = file.write_all(log_entry.as_bytes());
            }
        }

        if self.console_output {
            print!("{}", log_entry);
        }
    }
}

pub static LOGGER: std::sync::OnceLock<Mutex<Logger>> = std::sync::OnceLock::new();

pub fn init_logger(app_dir: &PathBuf, config: &LogConfig) {
    let logger = Logger::new(app_dir, config);
    let _ = LOGGER.set(Mutex::new(logger));
}

pub fn log(category: &str, message: &str, data: Option<&str>) {
    if let Some(logger_mutex) = LOGGER.get() {
        if let Ok(logger) = logger_mutex.lock() {
            logger.log(category, message, data);
        }
    }
}
