use thiserror::Error;

#[derive(Debug, Error)]
pub enum LabError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Training failed: {0}")]
    TrainingFailed(String),

    #[error("Data loading failed: {0}")]
    DataLoadFailed(String),

    #[error("Model error: {0}")]
    ModelError(String),

    #[error("Engine error: {0}")]
    EngineError(String),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("Session already running: {0}")]
    SessionAlreadyRunning(String),

    #[error("Session not in valid state for this operation: {0}")]
    InvalidSessionState(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Hardware detection error: {0}")]
    HardwareError(String),

    #[error("Remote connection error: {0}")]
    RemoteError(String),

    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, LabError>;
