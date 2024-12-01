use std::fmt::Display;

#[derive(Debug)]
pub enum SonorustDBError {
    InitDatabase(rusqlite::Error),
    GetGuildData(rusqlite::Error),
    UpdateGuildData(rusqlite::Error),
    GetUserData(rusqlite::Error),
    UpdateUserData(rusqlite::Error),
    Unknown(anyhow::Error),
}

impl Display for SonorustDBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SonorustDBError::*;

        match self {
            InitDatabase(error) => write!(f, "Failed to init database: {}", error),
            GetGuildData(error) => write!(f, "Failed to get guild data: {}", error),
            UpdateGuildData(error) => {
                write!(f, "Failed to update guild data: {}", error)
            }
            GetUserData(error) => write!(f, "Failed to get user data: {}", error),
            UpdateUserData(error) => {
                write!(f, "Failed to update user data: {}", error)
            }
            Unknown(error) => write!(f, "SonorustDB Unknown Error: {}", error),
        }
    }
}

impl std::error::Error for SonorustDBError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SonorustDBError::InitDatabase(error) => Some(error),
            SonorustDBError::GetGuildData(error) => Some(error),
            SonorustDBError::UpdateGuildData(error) => Some(error),
            SonorustDBError::GetUserData(error) => Some(error),
            SonorustDBError::UpdateUserData(error) => Some(error),
            SonorustDBError::Unknown(_) => None,
        }
    }
}
