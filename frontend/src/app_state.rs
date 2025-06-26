use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum AppState {
    #[default]
    NotInLobby, // Inital create/join screen
    InLobby, // Pre-game lobby
    InGame,  // Actively playing the game
}

impl AppState {
    pub fn _is_in_session(&self) -> bool {
        matches!(self, AppState::InLobby | AppState::InGame)
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "in_lobby" => AppState::InLobby,
            "in_game" => AppState::InGame,
            "not_in_lobby" => AppState::NotInLobby,
            _ => AppState::NotInLobby,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            AppState::NotInLobby => "not_in_lobby",
            AppState::InLobby => "in_lobby",
            AppState::InGame => "in_game",
        }
    }
}
