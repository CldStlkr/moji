use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameSettings {
    pub difficulty_levels: Vec<String>,
    pub time_limit_seconds: Option<u32>,
    pub max_players: u32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            difficulty_levels: vec![
                String::from("N1"),
                String::from("N2"),
                String::from("N3"),
                String::from("N4"),
                String::from("N5"),
            ],
            time_limit_seconds: None,
            max_players: 4,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GameStatus {
    Lobby,
    Playing,
    Finished,
}

#[derive(Serialize)]
pub struct KanjiPrompt {
    pub kanji: String,
}

#[derive(Deserialize)]
pub struct UserInput {
    pub word: String,
    pub kanji: String,
    pub player_id: String,
}

#[derive(Serialize)]
pub struct LobbyInfo {
    pub lobby_id: String,
    pub leader_id: String,
    pub players: Vec<PlayerData>,
    pub settings: GameSettings,
    pub status: GameStatus,
}

#[derive(Deserialize)]
pub struct JoinLobbyRequest {
    pub player_name: String,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub player_id: String,
    pub settings: GameSettings,
}

#[derive(Deserialize)]
pub struct StartGameRequest {
    pub player_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerData {
    pub id: String,
    pub name: String,
    pub score: u32,
    pub joined_at: String,
}

#[derive(Serialize)]
pub struct CheckWordResponse {
    pub message: String,
    pub score: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kanji: Option<String>,
}

// Still keep the UserScore struct for backward compatibility
#[derive(Default)]
pub struct UserScore {
    pub score: u32,
}

impl UserScore {
    pub fn new() -> Self {
        Self { score: 0 }
    }
}
