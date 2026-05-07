use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use chrono::Local;
use crate::infrastructure::config::LogConfig;

pub struct Logger {
    log_path: Option<PathBuf>,
    console_output: bool,
    #[allow(dead_code)]
    level: String,
    max_age_minutes: u64,
    last_reset: Instant,
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

            if config.clear_on_start {
                Self::clear_log_file(path);
            } else {
                Self::check_and_reset_if_stale(path, config.max_age_minutes);
            }
        }

        Self {
            log_path,
            console_output: config.console_output,
            level: config.level.clone(),
            max_age_minutes: config.max_age_minutes,
            last_reset: Instant::now(),
        }
    }

    fn clear_log_file(path: &PathBuf) {
        if path.exists() {
            if let Err(e) = fs::remove_file(path) {
                eprintln!("[Logger] Failed to clear log file: {}", e);
            }
        }
    }

    fn check_and_reset_if_stale(path: &PathBuf, max_age_minutes: u64) {
        if !path.exists() {
            return;
        }

        let metadata = match fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return,
        };

        let modified = match metadata.modified() {
            Ok(t) => t,
            Err(_) => return,
        };

        let modified_duration = match modified.duration_since(UNIX_EPOCH) {
            Ok(d) => d,
            Err(_) => return,
        };

        let now_duration = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d,
            Err(_) => return,
        };

        let age_minutes = (now_duration.as_secs() - modified_duration.as_secs()) / 60;

        if age_minutes >= max_age_minutes {
            Self::clear_log_file(path);
        }
    }

    fn should_rotate(&self) -> bool {
        if self.max_age_minutes == 0 {
            return false;
        }
        self.last_reset.elapsed().as_secs() >= self.max_age_minutes * 60
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

    pub fn rotate_if_needed(&mut self) {
        if self.should_rotate() {
            if let Some(ref path) = self.log_path {
                Self::clear_log_file(path);
                self.log("LOGGER", &format!("Log rotated (max age: {}min)", self.max_age_minutes), None);
            }
            self.last_reset = Instant::now();
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
        if let Ok(mut logger) = logger_mutex.lock() {
            logger.rotate_if_needed();
            logger.log(category, message, data);
        }
    }
}
