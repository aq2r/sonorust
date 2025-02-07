#[derive(Debug, thiserror::Error)]
pub enum SonorustError {
    #[error("SerenityError: {0}")]
    SerenityError(#[from] serenity::Error),

    #[error("SqlxError: {0}")]
    SqlxError(#[from] sqlx::Error),
}
