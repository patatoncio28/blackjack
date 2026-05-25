use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
pub enum PlatformError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Database error occurred: {0}")]
    DatabaseError(String),

    #[error("Vault operation failed: {0}")]
    VaultError(String),

    #[error("Permission denied: POSIX check failed")]
    PermissionDenied,

    #[error("Invalid or expired session token")]
    InvalidToken,

    #[error("Internal Server Error")]
    InternalError,

    #[error("Resource not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, PlatformError>;
