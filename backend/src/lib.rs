pub mod api;
pub mod data;
pub mod db;
pub mod error;
pub mod models;
pub mod types;

use chrono::{DateTime, Utc};
use data::{vectorize_joyo_kanji, vectorize_word_list};
use db::DbPool;
use error::AppError;
use rand::{distr::Alphanumeric, Rng};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};

// Re-export model types that are used in API endpoints
pub use shared::{
    CheckWordResponse, GameSettings, GameStatus, JoinLobbyRequest, KanjiPrompt, PlayerId, UserInput,
};
pub use types::{Result, Shared, SharedState};

// In lib.rs

#[derive(Clone, Debug)]
pub struct PlayerData {
    pub id: PlayerId,
    pub name: String,
    pub score: u32,
    pub joined_at: DateTime<Utc>, // DateTime internally
}

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

#[derive(Clone)]
pub struct LobbyState {
    pub word_list: Vec<String>,
    pub kanji_list: Vec<String>,
    pub players: Shared<Vec<PlayerData>>,
    pub lobby_leader: Shared<PlayerId>,
    pub settings: Shared<GameSettings>,
    pub game_status: Shared<GameStatus>,
    pub current_kanji: Shared<Option<String>>,
}

impl LobbyState {
    pub fn create() -> Result<Self> {
        // Determine the data directory based on environment
        let data_dir = if env::var("PRODUCTION").is_ok() {
            // In production (Docker), data is in /usr/local/data
            "/usr/local/data"
        } else {
            // In development, relative to the backend directory
            "../data"
        };

        let word_list_path = format!("{}/kanji_words.csv", data_dir);
        let kanji_list_path = format!("{}/N5_kanji.csv", data_dir);

        let list_of_words = vectorize_word_list(&word_list_path)
            .map_err(|e| AppError::DataLoadError(e.to_string()))?;

        let list_of_kanji = vectorize_joyo_kanji(&kanji_list_path)
            .map_err(|e| AppError::DataLoadError(e.to_string()))?;

        Ok(Self {
            word_list: list_of_words,
            kanji_list: list_of_kanji,
            players: Shared::new(Vec::new()),
            lobby_leader: Shared::new(PlayerId::default()),
            settings: Shared::new(GameSettings::default()),
            game_status: Shared::new(GameStatus::Lobby),
            current_kanji: Shared::new(None),
        })
    }

    pub fn is_leader(&self, player_id: &PlayerId) -> Result<bool> {
        let leader = self
            .lobby_leader
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        Ok(leader.to_string() == player_id.to_string())
    }

    pub fn update_settings(&self, player_id: &PlayerId, new_settings: GameSettings) -> Result<()> {
        if !self.is_leader(player_id)? {
            return Err(AppError::AuthError(
                "Only lobby leader can change settings".to_string(),
            ));
        }

        let mut settings = self
            .settings
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;

        *settings = new_settings;

        Ok(())
    }

    pub fn get_lobby_info(&self, lobby_id: &str) -> Result<shared::LobbyInfo> {
        let players_guard = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        let settings = self
            .settings
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        let status = self
            .game_status
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        let leader = self
            .lobby_leader
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;

        // Convert internal PlayerData to API PlayerData
        let api_players: Vec<shared::PlayerData> = players_guard
            .iter()
            .map(|p| shared::PlayerData {
                id: PlayerId(p.id.0.clone()),
                name: p.name.clone(),
                score: p.score,
                joined_at: p.joined_at.to_rfc3339(),
            })
            .collect();

        Ok(shared::LobbyInfo {
            lobby_id: lobby_id.to_string(),
            leader_id: leader.clone(),
            players: api_players,
            settings: settings.clone(),
            status: *status,
        })
    }

    pub fn start_game(&self, player_id: &PlayerId) -> Result<()> {
        if !self.is_leader(player_id)? {
            return Err(AppError::AuthError(
                "Only lobby leader can start the game".to_string(),
            ))?;
        }

        let mut status = self
            .game_status
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;

        if *status != GameStatus::Lobby {
            return Err(AppError::InvalidInput(
                "game is not in lobby state".to_string(),
            ))?;
        }

        *status = GameStatus::Playing;

        self.generate_random_kanji()?;

        Ok(())
    }

    // Add player to lobby
    pub fn add_player(&self, player_id: PlayerId, player_name: String) -> Result<bool> {
        let mut players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        let mut leader = self
            .lobby_leader
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;

        let is_leader = players.is_empty();
        if is_leader {
            *leader = player_id.clone();
        }

        players.push(PlayerData {
            id: player_id,
            name: player_name,
            score: 0,
            joined_at: Utc::now(),
        });
        Ok(is_leader)
    }

    // Get player score
    pub fn get_player_score(&self, player_id: &PlayerId) -> Result<u32> {
        let players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;

        players
            .iter()
            .find(|p| &p.id == player_id)
            .map(|p| p.score)
            .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))
    }

    // Get player name
    pub fn get_player_name(&self, player_id: &PlayerId) -> Result<String> {
        let players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        players
            .iter()
            .find(|p| &p.id == player_id)
            .map(|p| p.name.clone())
            .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))
    }

    // Update player score
    pub fn increment_player_score(&self, player_id: &PlayerId) -> Result<u32> {
        let mut players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        let player = players
            .iter_mut()
            .find(|p| &p.id == player_id)
            .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))?;

        player.score += 1;

        Ok(player.score)
    }

    // Get all players and scores (for a leaderboard)
    pub fn get_all_players(&self) -> Result<Vec<PlayerData>> {
        let players = self
            .players
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;
        Ok(players.iter().cloned().collect())
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

pub fn generate_player_id() -> PlayerId {
    PlayerId::from(generate_random_id(10))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lobby_id() {
        let id = generate_lobby_id();
        assert_eq!(id.len(), 6);
        // Check that ID is alphanumeric
        assert!(id.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_increment_player_score() {
        let lobby_state = LobbyState::create().unwrap();
        let player_id = PlayerId(String::from("test_player"));
        lobby_state
            .add_player(player_id.clone(), "Test Player".to_string())
            .unwrap();

        // Initial score should be 0
        assert_eq!(lobby_state.get_player_score(&player_id).unwrap(), 0);

        // After increment, should be 1
        assert_eq!(lobby_state.increment_player_score(&player_id).unwrap(), 1);
        assert_eq!(lobby_state.get_player_score(&player_id).unwrap(), 1);
    }

    #[test]
    fn test_get_current_kanji() {
        let lobby_state = LobbyState::create().unwrap();

        // Initially should be None
        assert_eq!(lobby_state.get_current_kanji().unwrap(), None);

        // Generate a kanji and verify it's set
        let kanji = lobby_state.generate_random_kanji().unwrap();
        assert_eq!(lobby_state.get_current_kanji().unwrap(), Some(kanji));
    }

    #[test]
    fn test_generate_random_kanji() {
        let lobby_state = LobbyState::create().unwrap();

        // Generate a kanji and verify it's from the list
        let kanji = lobby_state.generate_random_kanji().unwrap();
        assert!(lobby_state.kanji_list.contains(&kanji));

        // Generate another and ensure it's set as current
        let kanji2 = lobby_state.generate_random_kanji().unwrap();
        assert_eq!(lobby_state.get_current_kanji().unwrap(), Some(kanji2));
    }

    #[test]
    fn test_get_all_players() {
        let lobby_state = LobbyState::create().unwrap();

        // Initially empty
        assert!(lobby_state.get_all_players().unwrap().is_empty());

        // Add players and verify they're returned
        lobby_state
            .add_player(PlayerId::from("player1"), "Alice".to_string())
            .unwrap();
        lobby_state
            .add_player(PlayerId::from("player2"), "Bob".to_string())
            .unwrap();

        let players = lobby_state.get_all_players().unwrap();
        assert_eq!(players.len(), 2);

        // Option 1: Simple verification - check names exist
        let names: Vec<&String> = players.iter().map(|p| &p.name).collect();
        assert!(names.contains(&&"Alice".to_string()));
        assert!(names.contains(&&"Bob".to_string()));

        // Option 2: More thorough verification - find specific players
        let alice = players.iter().find(|p| p.id.0 == "player1");
        let bob = players.iter().find(|p| p.id.0 == "player2");

        assert!(alice.is_some());
        assert!(bob.is_some());

        // Option 3: Verify specific player details
        assert_eq!(alice.unwrap().name, "Alice");
        assert_eq!(bob.unwrap().name, "Bob");
        assert_eq!(alice.unwrap().score, 0);
        assert_eq!(bob.unwrap().score, 0);

        // Option 4: Verify order is maintained (first player added is first in Vec)
        assert_eq!(players[0].id, PlayerId(String::from("player1")));
        assert_eq!(players[0].name, "Alice");
        assert_eq!(players[1].id, PlayerId(String::from("player2")));
        assert_eq!(players[1].name, "Bob");
    }

    #[test]
    fn test_player_not_found_error() {
        let lobby_state = LobbyState::create().unwrap();

        // Attempt to get score for non-existent player
        let result = lobby_state.get_player_score(&PlayerId(String::from("nonexistent")));
        assert!(result.is_err());

        // Verify error type
        match result {
            Err(AppError::PlayerNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected PlayerNotFound error"),
        }
    }

    #[test]
    fn test_get_lobby_not_found() {
        let app_state = Arc::new(AppState::new());

        let result = get_lobby(&app_state, "nonexistent");
        assert!(result.is_err());

        // Verify error type
        match result {
            Err(AppError::LobbyNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected LobbyNotFound error"),
        }
    }
    #[test]
    fn test_lobby_workflow() {
        // Create app state
        let app_state = Arc::new(AppState::new());

        // Create a lobby and add it to the state
        let lobby_id = generate_lobby_id();
        let lobby_state = Arc::new(LobbyState::create().unwrap());

        {
            let mut lobbies = app_state.lobbies.lock().unwrap();
            lobbies.insert(lobby_id.clone(), lobby_state.clone());
        }

        // Get the lobby and verify it exists
        let retrieved_lobby = get_lobby(&app_state, &lobby_id).unwrap();

        // Add players to lobby
        retrieved_lobby
            .add_player(PlayerId::from("p1"), "Player 1".to_string())
            .unwrap();
        retrieved_lobby
            .add_player(PlayerId::from("p2"), "Player 2".to_string())
            .unwrap();

        // Generate kanji and check word
        let _kanji = retrieved_lobby.generate_random_kanji().unwrap();

        // Verify players and scores
        let players = retrieved_lobby.get_all_players().unwrap();
        assert_eq!(players.len(), 2);
    }

    #[test]
    fn test_lobby_leader_functionality() {
        let lobby_state = LobbyState::create().unwrap();

        // First player becomes leader
        let is_leader1 = lobby_state
            .add_player(PlayerId::from("player1"), "Alice".to_string())
            .unwrap();
        assert!(is_leader1);
        assert!(lobby_state.is_leader(&PlayerId::from("player1")).unwrap());

        // Second player is not leader
        let is_leader2 = lobby_state
            .add_player(PlayerId::from("player2"), "Bob".to_string())
            .unwrap();
        assert!(!is_leader2);
        assert!(!lobby_state.is_leader(&PlayerId::from("player2")).unwrap());
    }

    #[test]
    fn test_update_settings_leader_only() {
        let lobby_state = LobbyState::create().unwrap();

        lobby_state
            .add_player(PlayerId::from("leader"), "Leader".to_string())
            .unwrap();
        lobby_state
            .add_player(PlayerId::from("player"), "Player".to_string())
            .unwrap();

        let new_settings = GameSettings {
            difficulty_levels: vec!["N5".to_string(), "N4".to_string()],
            time_limit_seconds: Some(60),
            max_players: 10,
        };

        // Leader can update settings
        assert!(lobby_state
            .update_settings(&PlayerId::from("leader"), new_settings.clone())
            .is_ok());

        // Non-leader cannot update settings
        assert!(lobby_state
            .update_settings(&PlayerId::from("player"), new_settings)
            .is_err());
    }

    #[test]
    fn test_start_game_leader_only() {
        let lobby_state = LobbyState::create().unwrap();

        lobby_state
            .add_player(PlayerId::from("leader"), "Leader".to_string())
            .unwrap();
        lobby_state
            .add_player(PlayerId::from("player"), "Player".to_string())
            .unwrap();

        // Leader can start game
        assert!(lobby_state.start_game(&PlayerId::from("leader")).is_ok());

        // Game status should change to Playing
        let status = lobby_state.game_status.lock().unwrap();
        assert_eq!(*status, GameStatus::Playing);
    }
}
