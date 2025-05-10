pub mod api;
pub mod data;
pub mod db;
pub mod models;

use data::{vectorize_joyo_kanji, vectorize_word_list};
use db::DbPool;
use models::basic::UserScore;
use rand::Rng;
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};

// Re-export model types that are used in API endpoints
pub use models::basic::{KanjiPrompt, UserInput};

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

    pub async fn new_with_db(db_pool: Arc<DbPool>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            lobbies: Arc::new(Mutex::new(HashMap::new())),
            db_pool: Some(db_pool),
        })
    }
}

#[derive(Debug)]
pub enum LobbyCreationError {
    FailedToVectorizeKanjiListError,
    FailedToVectorizeWordListError,
}

#[derive(Clone)]
pub struct LobbyState {
    pub word_list: Vec<String>,
    pub kanji_list: Vec<String>,
    pub user_score: Arc<Mutex<UserScore>>,
    pub current_kanji: Arc<Mutex<Option<String>>>,
}

impl LobbyState {
    pub fn create() -> Result<Self, LobbyCreationError> {
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
            .map_err(|_| LobbyCreationError::FailedToVectorizeWordListError);
        let list_of_kanji = vectorize_joyo_kanji(&kanji_list_path)
            .map_err(|_| LobbyCreationError::FailedToVectorizeKanjiListError);

        Ok(Self {
            word_list: list_of_words.unwrap(),
            kanji_list: list_of_kanji.unwrap(),
            user_score: Arc::new(Mutex::new(UserScore::new())),
            current_kanji: Arc::new(Mutex::new(None)),
        })
    }

    pub fn generate_random_kanji(&self) -> String {
        let mut rng = rand::rng();
        let random_index = rng.random_range(0..self.kanji_list.len());
        let new_kanji = self.kanji_list[random_index].clone();

        // Update the current kanji in the lobby state
        let mut current_kanji = self.current_kanji.lock().unwrap();
        *current_kanji = Some(new_kanji.clone());

        new_kanji
    }

    pub fn get_current_kanji(&self) -> Option<String> {
        let kanji = self.current_kanji.lock().unwrap();
        kanji.clone()
    }
}

pub type SharedState = Arc<LobbyState>;
