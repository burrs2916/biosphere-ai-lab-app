pub mod config;
pub mod logging;

pub use logging::{init_logger, log};
pub use config::LogConfig;
