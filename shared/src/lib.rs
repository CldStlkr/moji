//! Shared domain types for Moji.
//! Compiles to `no_std` + `serde` in WASM, and
//! gets extra DB derives when the `ssr` feature is enabled.

#![cfg_attr(not(feature = "ssr"), no_std)]

extern crate alloc;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use core::ops::Deref;

use serde::{Deserialize, Serialize};

/// A prompt sent from the server to each client at the start of a round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KanjiPrompt {
    pub kanji: String,
}

/// The word a player submits to guess the prompt’s kanji.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInput {
    pub word: String,
    pub kanji: String,
    pub player_id: PlayerId,
}

/// Wrapper-type so we don’t pass raw strings everywhere.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerId(pub String);

impl core::str::FromStr for PlayerId {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PlayerId(s.to_string()))
    }
}

impl core::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}
impl Deref for PlayerId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&str> for PlayerId {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<String> for PlayerId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<PlayerId> for String {
    fn from(id: PlayerId) -> Self {
        id.0
    }
}

/// Sent when a player creates / joins a lobby.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinLobbyRequest {
    pub player_name: String,
}

/// Snapshot of a player inside a lobby.
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerData {
    pub id: PlayerId,
    pub name: String,
    pub score: u32,
    pub joined_at: String, // ISO-8601 for simplicity
}

/// Full lobby state, sent to all clients every poll / push.
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyInfo {
    pub lobby_id: String,
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
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            difficulty_levels: alloc::vec![
                "N1".into(),
                "N2".into(),
                "N3".into(),
                "N4".into(),
                "N5".into(),
            ],
            time_limit_seconds: None,
            max_players: 4,
        }
    }
}

/// Where the lobby / game is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameStatus {
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
    pub kanji: Option<String>,
}
