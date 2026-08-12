use shared::LobbyId;
use std::{
    env,
    sync::Arc,
};
use redis::AsyncCommands;
use crate::{
    data::{vectorize_joyo_kanji, load_dictionary, load_jlpt_words, JlptWordData, KanjiData, DictData},
    db::DbPool,
    error::AppError,
    types::Result,
    lobby::{LobbyHandle, LobbyData},
};
pub use shared::{
    CheckWordResponse, GameSettings, GameStatus, JoinLobbyRequest, PlayerId, ApiContext,
};


pub struct AppState {
    pub db_pool: tokio::sync::RwLock<Option<Arc<DbPool>>>,
    pub kanji_data: Arc<KanjiData>,
    pub word_data: Arc<JlptWordData>,
    pub dict_data: Arc<DictData>,
    pub redis: Option<redis::Client>,
}

impl AppState {

    fn load_data() -> Result<(Arc<KanjiData>, Arc<JlptWordData>, Arc<DictData>)> {
        let is_production = matches!(
            env::var("PRODUCTION").as_deref(),
            Ok("1") | Ok("true") | Ok("yes")
        );

        let data_dir = if is_production { "/usr/local/data" } else { "../data" };

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
            db_pool: tokio::sync::RwLock::new(None),
            kanji_data,
            word_data,
            dict_data,
            redis: None,
        })
    }

    pub fn set_redis(&mut self, client: redis::Client) { self.redis = Some(client); }

    /// Build a LobbyHandle for an existing lobby, verifying it exists in Redis.
    pub async fn get_lobby(&self, lobby_id: &LobbyId) -> Result<LobbyHandle> {
        let redis = self.redis.as_ref()
            .ok_or_else(|| AppError::InternalError("Redis not configured".into()))?;

        let mut conn = redis.get_multiplexed_async_connection().await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        let exists: bool = conn.exists(format!("lobby:{}", lobby_id.0)).await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        if !exists {
            return Err(AppError::LobbyNotFound(lobby_id.to_string()));
        }

        let db_pool = self.db_pool.read().await.clone();
        Ok(LobbyHandle::new(
            lobby_id.clone(),
            redis.clone(),
            Arc::clone(&self.kanji_data),
            Arc::clone(&self.word_data),
            Arc::clone(&self.dict_data),
            db_pool,
        ))
    }

    /// Build a LobbyHandle for a NEW lobby and write its initial data to Redis.
    pub async fn create_lobby_handle(
        &self,
        lobby_id: LobbyId,
        initial_data: LobbyData,
    ) -> Result<LobbyHandle> {
        let redis = self.redis.as_ref()
            .ok_or_else(|| AppError::InternalError("Redis not configured".into()))?;
        let db_pool = self.db_pool.read().await.clone();
        let handle = LobbyHandle::new(
            lobby_id,
            redis.clone(),
            Arc::clone(&self.kanji_data),
            Arc::clone(&self.word_data),
            Arc::clone(&self.dict_data),
            db_pool,
        );
        handle.save_data(&initial_data).await?;
        Ok(handle)
    }

    /// Scan Redis for all lobby keys and return summaries of public ones.
    pub async fn get_public_lobbies(&self) -> Result<Vec<shared::LobbySummary>> {
        let redis = self.redis.as_ref()
            .ok_or_else(|| AppError::InternalError("Redis not configured".into()))?;
        let mut conn = redis.get_multiplexed_async_connection().await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        // SCAN is the Redis-safe alternative to KEYS for production use.
        // It iterates the keyspace in chunks without blocking the server.
        let mut cursor = 0u64;
        let mut lobby_keys: Vec<String> = Vec::new();
        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor).arg("MATCH").arg("lobby:*").arg("COUNT").arg(100)
                .query_async(&mut conn).await
                .map_err(|e| AppError::InternalError(e.to_string()))?;
            // Filter out lock keys (lock:lobby:*) — they don't start with "lobby:"
            // so this is just defensive filtering
            lobby_keys.extend(keys.into_iter().filter(|k| !k.starts_with("lock:")));
            cursor = new_cursor;
            if cursor == 0 { break; }
        }

        let mut summaries = Vec::new();
        for key in lobby_keys {
            let json: Option<String> = conn.get(&key).await.ok().flatten();
            if let Some(j) = json {
                if let Ok(data) = serde_json::from_str::<LobbyData>(&j) {
                    if data.settings.is_public {
                        let lobby_id = LobbyId(key.trim_start_matches("lobby:").to_string());
                        let leader_name = data.players.iter()
                            .find(|p| p.id == data.lobby_leader)
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        summaries.push(shared::LobbySummary {
                            id: lobby_id,
                            leader_name,
                            player_count: data.players.len(),
                            max_players: data.settings.max_players,
                            mode: data.settings.mode,
                        });
                    }
                }
            }
        }

        Ok(summaries)
    }

    pub async fn set_db(&self, pool: Arc<DbPool>) {
        *self.db_pool.write().await = Some(pool);
    }
}
