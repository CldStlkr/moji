use serde::{Deserialize, Serialize};

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

#[derive(Deserialize)]
pub struct JoinLobbyRequest {
    pub player_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerInfo {
    pub name: String,
    pub score: u32,
}

#[derive(Serialize)]
pub struct CheckWordResponse {
    pub message: String,
    pub score: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
