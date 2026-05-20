use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;
use crate::infrastructure::config::LogConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn" | "warning" => LogLevel::Warn,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }

    pub fn to_file_suffix(self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

fn infer_level_from_message(message: &str) -> LogLevel {
    let msg_lower = message.to_lowercase();
    if msg_lower.starts_with("failed") || msg_lower.starts_with("error") || msg_lower.contains("panic") {
        LogLevel::Error
    } else if msg_lower.starts_with("warning") || msg_lower.starts_with("warn") {
        LogLevel::Warn
    } else if msg_lower.starts_with("debug") {
        LogLevel::Debug
    } else {
        LogLevel::Info
    }
}

pub struct Logger {
    log_dir: PathBuf,
    console_output: bool,
    min_level: LogLevel,
    file_prefix: String,
}

impl Logger {
    pub fn new(app_dir: &std::path::Path, config: &LogConfig) -> Self {
        let log_dir = if config.log_dir.is_absolute() {
            config.log_dir.clone()
        } else {
            app_dir.join(&config.log_dir)
        };

        if !log_dir.exists() {
            fs::create_dir_all(&log_dir).ok();
        }

        if config.clear_on_start {
            Self::clear_old_logs(&log_dir);
        }

        let min_level = LogLevel::from_str(&config.level);

        Self {
            log_dir,
            console_output: config.console_output,
            min_level,
            file_prefix: config.log_file.trim_end_matches(".log").to_string(),
        }
    }

    fn clear_old_logs(log_dir: &PathBuf) {
        if let Ok(entries) = fs::read_dir(log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "log") {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    fn get_log_file_path(&self, level: LogLevel) -> PathBuf {
        let date_str = Local::now().format("%Y-%m-%d").to_string();
        self.log_dir.join(format!("{}-{}-{}.log", self.file_prefix, date_str, level.to_file_suffix()))
    }

    fn write_to_file(&self, level: LogLevel, formatted_entry: &str) {
        let path = self.get_log_file_path(level);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = file.write_all(formatted_entry.as_bytes());
        }

        if level <= LogLevel::Error {
            let all_path = self.get_log_file_path(LogLevel::Error);
            if all_path != path {
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&all_path)
                {
                    let _ = file.write_all(formatted_entry.as_bytes());
                }
            }
        }
    }

    pub fn log_entry(&self, level: LogLevel, category: &str, message: &str, data: Option<&str>) {
        if level > self.min_level {
            return;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = match data {
            Some(d) => format!("[{}] [{}] [{}] {} | data: {}\n", timestamp, level.as_str(), category, message, d),
            None => format!("[{}] [{}] [{}] {}\n", timestamp, level.as_str(), category, message),
        };

        self.write_to_file(level, &log_line);

        if self.console_output {
            match level {
                LogLevel::Error => eprint!("{}", log_line),
                LogLevel::Warn => eprint!("{}", log_line),
                _ => print!("{}", log_line),
            }
        }
    }

    pub fn log(&self, category: &str, message: &str, data: Option<&str>) {
        let level = infer_level_from_message(message);
        self.log_entry(level, category, message, data);
    }
}

pub static LOGGER: std::sync::OnceLock<Mutex<Logger>> = std::sync::OnceLock::new();

pub fn init_logger(app_dir: &std::path::Path, config: &LogConfig) {
    let logger = Logger::new(app_dir, config);
    if LOGGER.set(Mutex::new(logger)).is_err() {
        eprintln!("[src-tauri] Logger already initialized, skipping");
    } else {
        eprintln!("[src-tauri] Logger initialized successfully");
    }
}

pub fn log(category: &str, message: &str, data: Option<&str>) {
    let level = infer_level_from_message(message);
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = match data {
        Some(d) => format!("[{}] [{}] [{}] {} | data: {}", timestamp, level.as_str(), category, message, d),
        None => format!("[{}] [{}] [{}] {}", timestamp, level.as_str(), category, message),
    };

    if let Some(logger_mutex) = LOGGER.get() {
        if let Ok(logger) = logger_mutex.lock() {
            logger.log(category, message, data);
            return;
        }
    }

    eprintln!("{}", log_line);
}

#[allow(dead_code)]
pub fn log_with_level(level: LogLevel, category: &str, message: &str, data: Option<&str>) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let log_line = match data {
        Some(d) => format!("[{}] [{}] [{}] {} | data: {}", timestamp, level.as_str(), category, message, d),
        None => format!("[{}] [{}] [{}] {}", timestamp, level.as_str(), category, message),
    };

    if let Some(logger_mutex) = LOGGER.get() {
        if let Ok(logger) = logger_mutex.lock() {
            logger.log_entry(level, category, message, data);
            return;
        }
    }

    eprintln!("{}", log_line);
}
