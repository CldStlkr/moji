use serde::{Deserialize, Serialize};
use thiserror::Error;
use web_sys::console;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ClientError {
    #[error("Failed to connect to server: {0}")]
    Connection(String),

    #[error("Server error: {status_code} - {message}")]
    Server { status_code: u16, message: String },

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Failed to load game data: {0}")]
    Data(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Input validation error: {0}")]
    Validation(String),

    #[error("Network error: {0}")]
    Network(String),
}

// Helper function to convert network errors to our ClientError
impl From<gloo_net::Error> for ClientError {
    fn from(error: gloo_net::Error) -> Self {
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
}

pub async fn parse_error_response(response: gloo_net::http::Response) -> ClientError {
    let status = response.status();

    // Try to extract structured error message
    match response.json::<serde_json::Value>().await {
        Ok(json) => {
            let message = json
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown server error")
                .to_string();

            match status {
                404 => ClientError::NotFound(message),
                400 => ClientError::Validation(message),
                401 | 403 => ClientError::Auth(message),
                500..=599 => ClientError::Server {
                    status_code: status,
                    message,
                },
                _ => ClientError::Server {
                    status_code: status,
                    message,
                },
            }
        }
        Err(_) => ClientError::Server {
            status_code: status,
            message: "Could not parse error response".to_string(),
        },
    }
}

// // Helper function to extract error from a server JSON response
// pub fn extract_server_error(response: &serde_json::Value) -> Option<String> {
//     response
//         .get("error")
//         .and_then(|e| e.as_str())
//         .map(|s| s.to_string())
// }

// Get a user-friendly error message for displaying to users
pub fn get_user_friendly_message(error: &ClientError) -> String {
    match error {
        ClientError::Network(_) => {
            "Unable to connect to the server. Please check your internet connection.".to_string()
        }
        ClientError::Connection(_) => {
            "Connection to the server failed. Please try again later.".to_string()
        }
        ClientError::Server {
            status_code: 500..=599,
            ..
        } => "The server encountered an error. Please try again later.".to_string(),
        ClientError::NotFound(msg) => format!("Not found: {}", msg),
        ClientError::Auth(_) => "You are not authorized to perform this action.".to_string(),
        ClientError::Validation(msg) => format!("Invalid input: {}", msg),
        ClientError::Data(msg) => format!("Data error: {}", msg),
        ClientError::Server { message, .. } => format!("Server error: {}", message),
    }
}

// Debug log an error (for developers)
pub fn log_error(message: &str, error: &ClientError) {
    console::log_1(&format!("{}: {:?}", message, error).into());
}
