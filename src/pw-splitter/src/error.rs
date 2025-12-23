use thiserror::Error;

#[derive(Error, Debug)]
pub enum PwSplitterError {
    #[error("Failed to execute PipeWire command: {0}")]
    CommandFailed(String),

    #[error("Failed to parse PipeWire output: {0}")]
    ParseError(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("No active connection found for source")]
    NoActiveConnection,

    #[error("Failed to spawn loopback: {0}")]
    LoopbackSpawnFailed(String),

    #[error("Failed to create link: {0}")]
    LinkCreationFailed(String),

    #[error("Failed to destroy link: {0}")]
    LinkDestroyFailed(String),

    #[error("State file error: {0}")]
    StateFileError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PwSplitterError>;
