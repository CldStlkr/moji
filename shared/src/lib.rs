pub mod api_fns;
pub use api_fns::*;

use serde::{Deserialize, Serialize};


macro_rules! new_type_id {
    ($name:ident) => {
        #[derive(Debug, Default, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl core::str::FromStr for $name {
            type Err = core::convert::Infallible;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok($name(s.to_string()))
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl core::ops::Deref for $name {
            type Target = String;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.into())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }

    };
}


new_type_id!(PlayerId);
new_type_id!(LobbyId);

///Messages sent from Client -> Server
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    /// User types something in input box
    Typing { input: String },
    /// User submits a guess
    Submit { input: String, prompt: String },
    // Return to lobby vote
    ReturnLobbyVote,
    /// User votes to skip or skips their turn
    Skip,
}



///Messages sent from Server -> Client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
    /// Sent upon connection and significant state changes
    GameState {
        prompt: String,
        status: GameStatus,
        scores: Vec<PlayerData>,
    },

    /// Broadcast when another player is typing
    PlayerTyping {
        player_id: PlayerId,
        input: String,
    },

    /// Broadcast result of a submission
    WordChecked {
        player_id: PlayerId,
        result: CheckWordResponse,
    },

    /// Broadcast immediately when a correct word is found
    PromptUpdate { new_prompt: String },
    PlayerListUpdate { players: Vec<PlayerData> },
    SettingsUpdate { settings: GameSettings },
    LeaderUpdate { leader_id: PlayerId },
    SkipVoteUpdate { votes: usize, required: usize },
    Kicked { player_id: PlayerId },
}

/// A prompt sent from the server to each client at the start of a round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptResponse {
    pub prompt: String,
}

/// Sent when a player creates / joins a lobby.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinLobbyRequest {
    pub player_name: String,
    pub player_id: Option<PlayerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: Option<String>,
    pub create_guest: bool,
}

/// Snapshot of a player inside a lobby.
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerData {
    pub id: PlayerId,
    pub name: String,
    pub score: u32,
    pub joined_at: String, // ISO-8601 for simplicity
    pub lives: Option<u32>,
    pub is_eliminated: bool,
    pub is_turn: bool,
    pub is_connected: bool,
    #[serde(default)]
    pub is_spectator: bool,
}

/// Full lobby state, sent to all clients every poll / push.
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyInfo {
    pub lobby_id: LobbyId,
    pub leader_id: PlayerId,
    pub players: Vec<PlayerData>,
    pub settings: GameSettings,
    pub status: GameStatus,
}

/// Per-game tunables chosen by the leader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSettings {
    pub difficulty_levels: Vec<String>,
    pub time_limit_seconds: Option<u32>,
    pub max_players: u32,
    pub weighted: bool,
    pub mode: GameMode,
    pub content_mode: ContentMode,
    pub target_score: Option<u32>,
    pub initial_lives: Option<u32>,
    pub duel_allow_kanji_reuse: bool,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    #[default]
    Deathmatch,
    Duel,
    Zen,
}


impl Default for GameSettings {
    fn default() -> Self {
        Self {
            difficulty_levels: vec![ // Replaced alloc::vec!
                "N1".into(),
                "N2".into(),
                "N3".into(),
                "N4".into(),
                "N5".into(),
            ],
            time_limit_seconds: None,
            max_players: 4,
            weighted: false,
            mode: GameMode::Deathmatch,
            content_mode: ContentMode::Kanji,
            target_score: Some(5), // Default target score for Deathmatch
            initial_lives: Some(3), // Default lives for Duel
            duel_allow_kanji_reuse: false,
        }
    }
}

/// Where the lobby / game is in its lifecycle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameStatus {
    #[default]
    Lobby,
    Playing,
    Finished,
}

/// Leader tweaks settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSettingsRequest {
    pub player_id: PlayerId,
    pub settings: GameSettings,
}

/// Leader presses *Start*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartGameRequest {
    pub player_id: PlayerId,
}

/// Returned to a client after the server scores its word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckWordResponse {
    pub message: String,
    pub score: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}


#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentMode {
    #[default]
    Kanji,
    Vocab,
}

/// The current game's prompt - varies by content mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivePrompt {
    /// Player must sumbit a word containing this kanji character
    Kanji { character: String },
    /// Player must submit the correct hiragana reading of this word
    Vocab { word: String, readings: Vec<String> },
}

impl ActivePrompt {
    pub fn display_text(&self) -> &str {
        match self {
            Self::Kanji { character } => character,
            Self::Vocab { word, .. } => word,
        }
    }
}
