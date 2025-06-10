use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::PoisonError;
use thiserror::Error;

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
    DataLoadError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl Clone for AppError {
    fn clone(&self) -> Self {
        match self {
            AppError::Database(e) => AppError::InternalError(format!("Database error: {}", e)),
            AppError::LobbyNotFound(id) => AppError::LobbyNotFound(id.clone()),
            AppError::PlayerNotFound(player_id) => AppError::PlayerNotFound(player_id.clone()),
            AppError::LockError(msg) => AppError::LockError(msg.clone()),
            AppError::InvalidInput(msg) => AppError::InvalidInput(msg.clone()),
            AppError::DataLoadError(msg) => AppError::DataLoadError(msg.clone()),
            AppError::AuthError(msg) => AppError::AuthError(msg.clone()),
            AppError::InternalError(msg) => AppError::InternalError(msg.clone()),
        }
    }
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
        tracing::error!("API error: {:?}", self);

        // Return user-friendly error to client
        let body = Json(json!({
            "error": self.to_string()
        }));

        (status, body).into_response()
    }
}

///// Handler to convert anyhow errors into responses
//pub async fn handle_anyhow_error(err: anyhow::Error) -> impl IntoResponse {
//    if let Some(app_err) = err.downcast_ref::<AppError>() {
//        // If it's one of our custom errors, we can use its IntoResponse implementation
//        return app_err.clone().into_response();
//    }
//
//    // Otherwise, it's an unexpected error
//    tracing::error!("Unexpected error: {:?}", err);
//    (
//        StatusCode::INTERNAL_SERVER_ERROR,
//        Json(json!({ "error": "An unexpected error occurred" })),
//    )
//        .into_response()
//}
