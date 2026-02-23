use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::PoisonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataLoadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV parse error: {0}")]
    Csv(#[from] csv::Error),
    #[error("Empty data file")]
    EmptyFile(std::path::PathBuf),
}

/// Application-specific error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Lobby with ID {0} not found")]
    LobbyNotFound(String),

    #[error("Player with ID {0} not found in lobby")]
    PlayerNotFound(String),

    #[error("Failed to access shared state: {0}")]
    LockError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Failed to load game data: {0}")]
    DataLoadError(#[from] DataLoadError),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}


// Convert any LockError into our AppError
impl<T> From<PoisonError<T>> for AppError {
    fn from(err: PoisonError<T>) -> Self {
        AppError::LockError(err.to_string())
    }
}

// Make our errors compatible with axum's response system
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::LobbyNotFound(_) | AppError::PlayerNotFound(_) => StatusCode::NOT_FOUND,
            AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            AppError::AuthError(_) => StatusCode::UNAUTHORIZED,
            AppError::Database(_)
            | AppError::LockError(_)
            | AppError::DataLoadError(_)
            | AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        // Log detailed error internally
        match &self {
            AppError::LobbyNotFound(_) | AppError::PlayerNotFound(_) => {
                tracing::warn!("Client requested missing resource: {:?}", self);
            }
            AppError::InvalidInput(_) | AppError::AuthError(_) => {
                tracing::warn!("Client error: {:?}", self);
            }
            _ => {
                tracing::error!("Internal API error: {:?}", self);
            }
        }

        // Return user-friendly error to client
        let body = Json(json!({
            "error": self.to_string()
        }));

        (status, body).into_response()
    }
}
