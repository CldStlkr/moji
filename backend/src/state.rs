use shared::LobbyId;
use std::{
    collections::HashMap,
    env,
    sync::Arc,
};
use crate::{
    data::{vectorize_joyo_kanji, load_dictionary, load_jlpt_words, JlptWordData, KanjiData, DictData},
    db::DbPool,
    error::AppError,
    types::{Result, Shared, SharedState},
};
pub use shared::{
    CheckWordResponse, GameSettings, GameStatus, JoinLobbyRequest, PlayerId, ApiContext,
};


pub struct AppState {
    pub lobbies: Shared<HashMap<LobbyId, SharedState>>,
    pub db_pool: Option<Arc<DbPool>>,
    pub kanji_data: Arc<KanjiData>,
    pub word_data: Arc<JlptWordData>,
    pub dict_data: Arc<DictData>,
}

impl AppState {

    fn load_data() -> Result<(Arc<KanjiData>, Arc<JlptWordData>, Arc<DictData>)> {
        let is_production = matches!(
            env::var("PRODUCTION").as_deref(),
            Ok("1") | Ok("true") | Ok("yes")
        );

        let data_dir = if is_production {
            "/usr/local/data"
        } else {
            // In development, relative to the backend directory
            "../data"
        };

        let kanji_list_paths: Vec<String> = vec![
            format!("{}/N1_kanji.csv", data_dir),
            format!("{}/N2_kanji.csv", data_dir),
            format!("{}/N3_kanji.csv", data_dir),
            format!("{}/N4_kanji.csv", data_dir),
            format!("{}/N5_kanji.csv", data_dir),
        ];
        let word_list_paths: Vec<String> = vec![
            format!("{}/N1_words.csv", data_dir),
            format!("{}/N2_words.csv", data_dir),
            format!("{}/N3_words.csv", data_dir),
            format!("{}/N4_words.csv", data_dir),
            format!("{}/N5_words.csv", data_dir),
        ];
        let dictionary_path = format!("{}/kanji_words.csv", data_dir);


        let list_of_kanji = Arc::new(vectorize_joyo_kanji(&kanji_list_paths)?);
        let list_of_words = Arc::new(load_jlpt_words(&word_list_paths)?);
        let dictionary_list = Arc::new(load_dictionary(&dictionary_path)?);

        Ok((list_of_kanji, list_of_words, dictionary_list))
    }

    pub fn create() -> Result<Self> {
        let (kanji_data, word_data, dict_data) = Self::load_data()?;
        Ok(Self {
            lobbies: Shared::new(HashMap::new()),
            db_pool: None,
            kanji_data,
            word_data,
            dict_data
        })
    }


    pub fn get_lobby(&self, lobby_id: &LobbyId) -> Result<SharedState> {
        self.lobbies.read(|lobbies| {
            lobbies.get(lobby_id).cloned()
                .ok_or_else(|| AppError::LobbyNotFound(lobby_id.to_string()))
        })
    }

    pub async fn new_with_db(db_pool: Arc<DbPool>) -> Result<Self> {

        let (kanji_data, word_data, dict_data) = Self::load_data()?;
        Ok(Self {
            lobbies: Shared::new(HashMap::new()),
            db_pool: Some(db_pool),
            kanji_data,
            word_data,
            dict_data
        })
    }
}
