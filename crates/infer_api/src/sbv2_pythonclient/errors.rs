#[derive(Debug, thiserror::Error)]
pub enum Sbv2PythonClientError {
    #[error("reqwest Error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("serde_json Error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("ModelInfoParseError: {0}")]
    ModelInfoParseError(String),
}
