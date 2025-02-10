#[derive(Debug, thiserror::Error)]
pub enum Sbv2RustError {
    #[error("IOError: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Sbv2CoreError: {0}")]
    Sbv2CoreError(String),

    #[error("TokioJoinError: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error("Model not found")]
    ModelNotFound,
}
