use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ConnectServerError {
    #[error("Database error: {0}")]
    Database(#[from] mongodb::error::Error),

    #[error("Password hashing error: {0}")]
    PasswordHash(#[from] bcrypt::BcryptError),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Session not found or expired")]
    InvalidSession,

    #[error("Account already logged in")]
    DuplicateLogin,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Authentication required")]
    Unauthorized,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

impl ResponseError for ConnectServerError {
    fn status_code(&self) -> StatusCode {
        match self {
            ConnectServerError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            ConnectServerError::InvalidSession => StatusCode::UNAUTHORIZED,
            ConnectServerError::Unauthorized => StatusCode::UNAUTHORIZED,
            ConnectServerError::DuplicateLogin => StatusCode::CONFLICT,
            ConnectServerError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            ConnectServerError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConnectServerError::PasswordHash(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConnectServerError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConnectServerError::Serialization(_) => StatusCode::BAD_REQUEST,
            ConnectServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let error_response = ErrorResponse {
            success: false,
            error: self.to_string(),
        };

        HttpResponse::build(status).json(error_response)
    }
}

pub type Result<T> = std::result::Result<T, ConnectServerError>;
