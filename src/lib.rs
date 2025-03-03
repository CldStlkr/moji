pub mod api;
pub mod data;
pub mod models;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use data::{vectorize_joyo_kanji, vectorize_word_list};
use models::UserScore;

pub struct AppState {
    pub lobbies: Arc<Mutex<HashMap<String, SharedState>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            lobbies: Arc::new(Mutex::new(HashMap::new())),
        }
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
    pub user_score: Arc<Mutex<models::UserScore>>,
    pub current_kanji: String,
}

impl LobbyState {
    pub fn create() -> Result<Self, LobbyCreationError> {
        let list_of_words = vectorize_word_list("./data/kanji_words.csv")
            .map_err(|_| LobbyCreationError::FailedToVectorizeWordListError);
        let list_of_kanji = vectorize_joyo_kanji("./data/joyo_kanji.csv")
            .map_err(|_| LobbyCreationError::FailedToVectorizeKanjiListError);

        Ok(Self {
            word_list: list_of_words.unwrap(),
            kanji_list: list_of_kanji.unwrap(),
            user_score: Arc::new(Mutex::new(UserScore::new())),
            current_kanji: String::new(),
        })
    }
}

pub type SharedState = Arc<LobbyState>;
