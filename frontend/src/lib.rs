pub mod api;
pub mod error;
pub mod persistence;

// Re-export
pub use error::{get_user_friendly_message, ClientError};

pub mod components {
    pub mod game;
    pub mod lobby;
    pub mod player_scores;
}
