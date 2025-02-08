use infer_api::{Sbv2PythonError, Sbv2RustError};

#[derive(Debug, thiserror::Error)]
pub enum SonorustError {
    #[error("SerenityError: {0}")]
    SerenityError(#[from] serenity::Error),

    #[error("SqlxError: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("Sbv2PythonError: {0}")]
    Sbv2PythonError(#[from] Sbv2PythonError),

    #[error("Sbv2RustError: {0}")]
    Sbv2RustError(#[from] Sbv2RustError),

    #[error("GuildId is None")]
    GuildIdIsNone,
}
