use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ClientError {
    #[error("Failed to connect to server: {0}")]
    Connection(String),

    #[error("Error response from server: {0}")]
    Server(String),

    #[error("Failed to load game data: {0}")]
    Data(String),

    #[error("Login error: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),
}

// Helper function to convert network errors to our ClientError
pub fn handle_network_error(error: gloo_net::Error) -> ClientError {
    match error {
        gloo_net::Error::JsError(js_err) => {
            // This could be a network error or failed fetch
            ClientError::Network(js_err.to_string())
        }
        gloo_net::Error::SerdeError(serde_err) => {
            // Error deserializing the response
            ClientError::Data(format!("Failed to parse server response: {}", serde_err))
        }
        gloo_net::Error::GlooError(gloo_err) => {
            // General Gloo error
            ClientError::Connection(gloo_err.to_string())
        }
    }
}

// Helper function to extract error from a server JSON response
pub fn extract_server_error(response: &serde_json::Value) -> Option<String> {
    response
        .get("error")
        .and_then(|e| e.as_str())
        .map(|s| s.to_string())
}
