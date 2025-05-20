pub mod api;
pub mod data;
pub mod db;
pub mod error;
pub mod models;
use data::{vectorize_joyo_kanji, vectorize_word_list};
use db::DbPool;
use error::{AppError, Result};
use rand::{distr::Alphanumeric, Rng};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};

// Re-export model types that are used in API endpoints
pub use models::basic::{CheckWordResponse, JoinLobbyRequest, KanjiPrompt, PlayerInfo, UserInput};

pub struct AppState {
    pub lobbies: Arc<Mutex<HashMap<String, SharedState>>>,
    pub db_pool: Option<Arc<DbPool>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            lobbies: Arc::new(Mutex::new(HashMap::new())),
            db_pool: None,
        }
    }

    pub async fn new_with_db(db_pool: Arc<DbPool>) -> Result<Self> {
        Ok(Self {
            lobbies: Arc::new(Mutex::new(HashMap::new())),
            db_pool: Some(db_pool),
        })
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct PlayerData {
    pub name: String,
    pub score: u32,
}

#[derive(Clone)]
pub struct LobbyState {
    pub word_list: Vec<String>,
    pub kanji_list: Vec<String>,
    pub players: Arc<Mutex<HashMap<String, PlayerData>>>,
    pub current_kanji: Arc<Mutex<Option<String>>>,
}

impl LobbyState {
    pub fn create() -> Result<Self> {
        // Determine the data directory based on environment
        let data_dir = if env::var("PRODUCTION").is_ok() {
            // In production (Docker), data is in /usr/local/data
            "/usr/local/data"
        } else {
            // In development, relative to the backend directory
            "./data"
        };

        let word_list_path = format!("{}/kanji_words.csv", data_dir);
        let kanji_list_path = format!("{}/joyo_kanji.csv", data_dir);

        let list_of_words = vectorize_word_list(&word_list_path)
            .map_err(|e| AppError::DataLoadError(e.to_string()))?;

        let list_of_kanji = vectorize_joyo_kanji(&kanji_list_path)
            .map_err(|e| AppError::DataLoadError(e.to_string()))?;

        Ok(Self {
            word_list: list_of_words,
            kanji_list: list_of_kanji,
            players: Arc::new(Mutex::new(HashMap::new())),
            current_kanji: Arc::new(Mutex::new(None)),
        })
    }

    // Add player to lobby
    pub fn add_player(&self, player_id: String, player_name: String) -> Result<()> {
        let mut players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        players.insert(
            player_id,
            PlayerData {
                name: player_name,
                score: 0,
            },
        );
        Ok(())
    }

    // Update player score
    pub fn increment_player_score(&self, player_id: &str) -> Result<u32> {
        let mut players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        let player = players
            .get_mut(player_id)
            .ok_or_else(|| AppError::PlayerNotFound(player_id.to_string()))?;

        player.score += 1;

        Ok(player.score)
    }

    // Get player score
    pub fn get_player_score(&self, player_id: &str) -> Result<u32> {
        let players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        players
            .get(player_id)
            .map(|data| data.score)
            .ok_or_else(|| AppError::PlayerNotFound(player_id.to_string()))
    }

    // Get player name
    pub fn get_player_name(&self, player_id: &str) -> Result<String> {
        let players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        players
            .get(player_id)
            .map(|data| data.name.clone())
            .ok_or_else(|| AppError::PlayerNotFound(player_id.to_string()))
    }

    // Get all players and scores (for a leaderboard)
    pub fn get_all_players(&self) -> Result<Vec<(String, PlayerData)>> {
        let players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        Ok(players
            .iter()
            .map(|(id, data)| (id.clone(), data.clone()))
            .collect())
    }

    pub fn get_current_kanji(&self) -> Result<Option<String>> {
        let kanji = self
            .current_kanji
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        Ok(kanji.clone())
    }

    pub fn generate_random_kanji(&self) -> Result<String> {
        let mut rng = rand::rng();
        let random_index = rng.random_range(0..self.kanji_list.len());
        let new_kanji = self.kanji_list[random_index].clone();

        // Update the current kanji in the lobby state
        let mut current_kanji = self
            .current_kanji
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        *current_kanji = Some(new_kanji.clone());
        Ok(new_kanji)
    }
}

pub fn generate_random_id(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn generate_player_id() -> String {
    generate_random_id(10)
}

pub fn generate_lobby_id() -> String {
    generate_random_id(6)
}

pub fn get_lobby(app_state: &Arc<AppState>, lobby_id: &str) -> Result<SharedState> {
    let lobbies = app_state
        .lobbies
        .lock()
        .map_err(|e| AppError::LockError(e.to_string()))?;
    lobbies
        .get(lobby_id)
        .cloned()
        .ok_or_else(|| AppError::LobbyNotFound(lobby_id.to_string()))
}

pub type SharedState = Arc<LobbyState>;
