use chrono::Utc;
use rand::{RngExt, distr::{Distribution, weighted::WeightedIndex}};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use redis::AsyncCommands;
use std::{
    collections::HashSet,
    sync::Arc,
};

pub use shared::{
    CheckWordResponse, GameSettings, GameStatus, JoinLobbyRequest, PlayerId, ApiContext,
    ContentMode, ActivePrompt, LobbyId, LobbyInfo
};
pub use crate::{
    utils::check_prompt,
    types::{Result, PlayerData},
    data::{JlptWordData, KanjiData, DictData},
    error::AppError,
};

// ── LobbyData ────────────────────────────────────────────────────────────────
//
// All mutable game state stored in Redis as JSON at key `lobby:{id}`.
// Plain data — no locks, no Arcs, no channels. Any pod can read and write it.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LobbyData {
    pub players: Vec<PlayerData>,
    pub lobby_leader: PlayerId,
    pub settings: GameSettings,
    pub game_status: GameStatus,
    pub current_prompt: Option<ActivePrompt>,
    pub active_level_indices: Vec<usize>,
    pub turn_order: Vec<PlayerId>,
    pub current_turn_index: usize,
    pub prompt_counter: u64,
    pub skip_votes: HashSet<PlayerId>,
    pub return_lobby_votes: HashSet<PlayerId>,
    pub reuse_prompt: bool,
    pub cleanup_generation: u64,
    pub timer_expires_at: Option<u64>,
    pub game_session_id: Option<uuid::Uuid>,
}

impl Default for LobbyData {
    fn default() -> Self {
        Self {
            players: Vec::new(),
            lobby_leader: PlayerId::default(),
            settings: GameSettings::default(),
            game_status: GameStatus::Lobby,
            current_prompt: None,
            active_level_indices: Vec::new(),
            turn_order: Vec::new(),
            current_turn_index: 0,
            prompt_counter: 0,
            skip_votes: HashSet::new(),
            return_lobby_votes: HashSet::new(),
            reuse_prompt: false,
            cleanup_generation: 0,
            timer_expires_at: None,
            game_session_id: None,
        }
    }
}

impl LobbyData {
    pub fn get_all_players(&self) -> Vec<shared::PlayerData> {
        let current_turn = self.turn_order.get(self.current_turn_index).cloned();
        self.players.iter().map(|p| shared::PlayerData {
            id: p.id.clone(),
            name: p.name.clone(),
            score: p.score,
            joined_at: p.joined_at.to_rfc3339(),
            lives: p.lives,
            is_eliminated: p.is_eliminated,
            is_connected: p.is_connected,
            is_spectator: p.is_spectator,
            is_turn: current_turn.as_ref() == Some(&p.id)
                && self.game_status == GameStatus::Playing
                && self.settings.mode == shared::GameMode::Duel,
        }).collect()
    }

    pub fn get_current_prompt_text(&self) -> Option<String> {
        self.current_prompt.as_ref().map(|p| p.display_text().to_string())
    }

    pub fn to_lobby_info(&self, lobby_id: &LobbyId) -> LobbyInfo {
        LobbyInfo {
            lobby_id: lobby_id.clone(),
            leader_id: self.lobby_leader.clone(),
            players: self.get_all_players(),
            settings: self.settings.clone(),
            status: self.game_status,
        }
    }
}

// ── LobbyHandle ───────────────────────────────────────────────────────────────
//
// Thin handle with no game state. Cheap to clone (all fields are Arc or Clone).
// Used to find and update LobbyData in Redis.

#[derive(Clone)]
pub struct LobbyHandle {
    pub lobby_id: LobbyId,
    pub redis: redis::Client,
    pub kanji_list: Arc<KanjiData>,
    pub word_list: Arc<JlptWordData>,
    pub dict_list: Arc<DictData>,
    pub db_pool: Option<Arc<crate::db::DbPool>>,
    pub tx: broadcast::Sender<String>,
}

impl LobbyHandle {
    pub fn new(
        lobby_id: LobbyId,
        redis: redis::Client,
        kanji_list: Arc<KanjiData>,
        word_list: Arc<JlptWordData>,
        dict_list: Arc<DictData>,
        db_pool: Option<Arc<crate::db::DbPool>>,
    ) -> Self {
        Self { lobby_id, redis, kanji_list, word_list, dict_list, db_pool, tx: broadcast::channel(100).0 }
    }

    fn data_key(&self) -> String { format!("lobby:{}", self.lobby_id.0) }
    fn lock_key(&self) -> String { format!("lock:lobby:{}", self.lobby_id.0) }

    pub async fn get_data(&self) -> Result<LobbyData> {
        let mut conn = self.redis.get_multiplexed_async_connection().await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        let json: Option<String> = conn.get(self.data_key()).await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        match json {
            Some(j) => serde_json::from_str(&j).map_err(|e| AppError::InternalError(e.to_string())),
            None => Err(AppError::LobbyNotFound(self.lobby_id.0.clone())),
        }
    }

    pub async fn save_data(&self, data: &LobbyData) -> Result<()> {
        let mut conn = self.redis.get_multiplexed_async_connection().await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        let json = serde_json::to_string(data)
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        conn.set::<_, _, ()>(self.data_key(), json).await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Atomic read-modify-write with a distributed Redis lock.
    ///
    /// The closure `f` is sync and takes only `&mut LobbyData` — NOT `&LobbyHandle`.
    /// Callers capture anything they need from self (kanji_list, dict_list, etc.)
    /// by Arc::clone BEFORE calling with_lobby. This is intentional: HRTB on
    /// multiple reference parameters (`&LobbyHandle, &mut LobbyData`) prevents
    /// the compiler from proving the returned future is Send, which breaks
    /// `tokio::spawn`. A single `&mut LobbyData` parameter avoids this.
    pub async fn with_lobby<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut LobbyData) -> Result<R> + Send,
        R: Send,
    {
        let mut conn = self.redis.get_multiplexed_async_connection().await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        let lock_key = self.lock_key();

        let mut acquired: bool = redis::cmd("SET")
            .arg(&lock_key).arg("1").arg("NX").arg("PX").arg(2000)
            .query_async(&mut conn).await.unwrap_or(false);

        if !acquired {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            acquired = redis::cmd("SET")
                .arg(&lock_key).arg("1").arg("NX").arg("PX").arg(2000)
                .query_async(&mut conn).await.unwrap_or(false);
        }

        if !acquired {
            return Err(AppError::InternalError("Could not acquire lobby lock".into()));
        }

        let json: Option<String> = conn.get(self.data_key()).await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut data = match json {
            Some(j) => serde_json::from_str::<LobbyData>(&j)
                .map_err(|e| AppError::InternalError(e.to_string()))?,
            None => {
                let _: () = conn.del(&lock_key).await.unwrap_or(());
                return Err(AppError::LobbyNotFound(self.lobby_id.0.clone()));
            }
        };

        let result = f(&mut data);

        if result.is_ok() {
            if let Ok(new_json) = serde_json::to_string(&data) {
                let _: () = conn.set(self.data_key(), &new_json).await.unwrap_or(());
            }
        }

        let _: () = conn.del(&lock_key).await.unwrap_or(());
        result
    }

    pub fn broadcast(&self, msg: shared::ServerMessage) {
        let msg_json = serde_json::to_string(&msg).unwrap_or_default();
        let client = self.redis.clone();
        let channel = format!("lobby:{}", self.lobby_id.0);
        tokio::spawn(async move {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                let _: redis::RedisResult<i64> = redis::cmd("PUBLISH")
                    .arg(&channel).arg(&msg_json).query_async(&mut conn).await;
            }
        });
    }

    // ── Game methods ──────────────────────────────────────────────────────────

    pub async fn get_lobby_info(&self) -> Result<LobbyInfo> {
        let data = self.get_data().await?;
        Ok(data.to_lobby_info(&self.lobby_id))
    }

    pub async fn update_settings(&self, player_id: &PlayerId, new_settings: GameSettings) -> Result<()> {
        let pid = player_id.clone();
        let settings_clone = new_settings.clone();
        self.with_lobby(move |data| {
            if !is_leader(data, &pid) {
                return Err(AppError::AuthError("Only lobby leader can change settings".into()));
            }
            data.settings = new_settings;
            Ok(())
        }).await?;
        self.broadcast(shared::ServerMessage::SettingsUpdate { settings: settings_clone });
        Ok(())
    }

    pub async fn add_player(&self, player_id: PlayerId, player_name: String) -> Result<bool> {
        let (is_leader_result, players) = self.with_lobby(move |data| {
            let is_leader_result = data.players.is_empty();
            if is_leader_result { data.lobby_leader = player_id.clone(); }

            let trimmed = player_name.trim();
            if trimmed.is_empty() {
                return Err(AppError::InvalidInput("Player name cannot be empty".into()));
            }
            let normalized = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");

            data.players.retain(|p| p.id != player_id && p.name != normalized);
            let is_spectator = data.game_status != GameStatus::Lobby;
            data.players.push(PlayerData {
                id: player_id.clone(),
                name: normalized,
                score: 0,
                joined_at: Utc::now(),
                lives: None,
                is_eliminated: false,
                is_connected: true,
                is_spectator,
            });
            Ok((is_leader_result, data.get_all_players()))
        }).await?;
        self.broadcast(shared::ServerMessage::PlayerListUpdate { players });
        Ok(is_leader_result)
    }

    pub async fn remove_player(&self, player_id: &PlayerId) -> Result<bool> {
        let pid = player_id.clone();
        let (removed, players, leader_update) = self.with_lobby(move |data| {
            if let Some(pos) = data.players.iter().position(|p| p.id == pid) {
                data.players.remove(pos);

                if let Some(t_pos) = data.turn_order.iter().position(|id| id == &pid) {
                    data.turn_order.remove(t_pos);
                    if data.current_turn_index >= data.turn_order.len() && !data.turn_order.is_empty() {
                        data.current_turn_index = 0;
                    }
                }

                let leader_update = if data.lobby_leader == pid {
                    if let Some(new_leader) = data.players.first() {
                        let id = new_leader.id.clone();
                        data.lobby_leader = id.clone();
                        tracing::info!("Reassigned lobby leader to {}", id.0);
                        Some(id)
                    } else {
                        tracing::info!("No players left to be leader");
                        data.lobby_leader = PlayerId::default();
                        None
                    }
                } else { None };

                Ok((true, data.get_all_players(), leader_update))
            } else {
                tracing::info!("remove_player: {} not found", pid.0);
                Ok((false, vec![], None))
            }
        }).await?;

        if removed {
            if let Some(new_leader_id) = leader_update {
                self.broadcast(shared::ServerMessage::LeaderUpdate { leader_id: new_leader_id });
            }
            self.broadcast(shared::ServerMessage::PlayerListUpdate { players });
        }
        Ok(removed)
    }

    pub async fn kick_player(&self, requestor_id: &PlayerId, target_id: &PlayerId) -> Result<()> {
        if requestor_id == target_id {
            return Err(AppError::InvalidInput("Cannot kick yourself".into()));
        }
        let req = requestor_id.clone();
        let tgt = target_id.clone();
        let players = self.with_lobby(move |data| {
            if !is_leader(data, &req) {
                return Err(AppError::AuthError("Only the lobby leader can kick players".into()));
            }
            data.players.retain(|p| p.id != tgt);
            Ok(data.get_all_players())
        }).await?;
        self.broadcast(shared::ServerMessage::Kicked { player_id: target_id.clone() });
        self.broadcast(shared::ServerMessage::PlayerListUpdate { players });
        Ok(())
    }

    pub async fn promote_leader(&self, requestor_id: &PlayerId, target_id: &PlayerId) -> Result<()> {
        let req = requestor_id.clone();
        let tgt = target_id.clone();
        let players = self.with_lobby(move |data| {
            if !is_leader(data, &req) {
                return Err(AppError::AuthError("Only the lobby leader can promote a new leader".into()));
            }
            if !data.players.iter().any(|p| p.id == tgt) {
                return Err(AppError::InvalidInput("Target player is not in the lobby".into()));
            }
            data.lobby_leader = tgt.clone();
            Ok(data.get_all_players())
        }).await?;
        self.broadcast(shared::ServerMessage::LeaderUpdate { leader_id: target_id.clone() });
        self.broadcast(shared::ServerMessage::PlayerListUpdate { players });
        Ok(())
    }

    pub async fn set_player_connected(&self, player_id: &PlayerId, is_connected: bool) -> Result<bool> {
        let pid = player_id.clone();
        let (changed, players, bump_gen) = self.with_lobby(move |data| {
            let changed = if let Some(p) = data.players.iter_mut().find(|p| p.id == pid) {
                p.is_connected = is_connected;
                true
            } else { false };
            let bump_gen = changed && is_connected;
            if bump_gen { data.cleanup_generation += 1; }
            Ok((changed, data.get_all_players(), bump_gen))
        }).await?;
        let _ = bump_gen; // already applied inside with_lobby
        if changed {
            self.broadcast(shared::ServerMessage::PlayerListUpdate { players });
        }
        Ok(changed)
    }

    pub async fn all_disconnected(&self) -> Result<bool> {
        let data = self.get_data().await?;
        Ok(!data.players.is_empty() && data.players.iter().all(|p| !p.is_connected))
    }

    pub async fn get_player_name(&self, player_id: &PlayerId) -> Result<String> {
        let data = self.get_data().await?;
        data.players.iter()
            .find(|p| &p.id == player_id)
            .map(|p| p.name.clone())
            .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))
    }

    pub async fn start_game(&self, player_id: &PlayerId) -> Result<()> {
        let pid = player_id.clone();
        let kanji = Arc::clone(&self.kanji_list);
        let words = Arc::clone(&self.word_list);

        let (timer_spawn, prompt, scores, timer_at) = self.with_lobby(move |data| {
            if !is_leader(data, &pid) {
                return Err(AppError::AuthError("Only lobby leader can start the game".into()));
            }
            if data.game_status != GameStatus::Lobby {
                return Err(AppError::InvalidInput("Game is not in lobby state".into()));
            }

            let settings = data.settings.clone();
            let mut indices: Vec<usize> = Vec::new();
            for level in &settings.difficulty_levels {
                let idx = match level.as_str() {
                    "N1" => 0, "N2" => 1, "N3" => 2, "N4" => 3, "N5" => 4, _ => 99
                };
                if idx < kanji.len() && !kanji[idx].is_empty() { indices.push(idx); }
            }
            if indices.is_empty() && kanji.len() > 4 { indices.push(4); }
            data.active_level_indices = indices;

            data.turn_order.clear();
            data.current_turn_index = 0;
            for p in data.players.iter_mut() {
                p.score = 0; p.is_eliminated = false; p.is_spectator = false;
                if settings.mode == shared::GameMode::Duel {
                    p.lives = settings.initial_lives;
                    data.turn_order.push(p.id.clone());
                } else { p.lives = None; }
            }
            if settings.mode == shared::GameMode::Duel {
                use rand::seq::SliceRandom;
                data.turn_order.shuffle(&mut rand::rng());
            }

            let (_, timer_spawn) = generate_prompt_sync(&kanji, &words, data, true)?;
            data.game_status = GameStatus::Playing;

            Ok((timer_spawn, data.get_current_prompt_text().unwrap_or_default(), data.get_all_players(), data.timer_expires_at))
        }).await?;

        self.broadcast(shared::ServerMessage::GameState {
            prompt, status: GameStatus::Playing, scores, timer_expires_at: timer_at,
        });

        if let Some((counter, secs)) = timer_spawn {
            let handle = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(secs as u64)).await;
                let _ = handle.process_timeout(counter).await;
            });
        }
        Ok(())
    }

    pub async fn reset_lobby(&self, player_id: &PlayerId) -> Result<()> {
        let pid = player_id.clone();
        let scores = self.with_lobby(move |data| {
            if !is_leader(data, &pid) {
                return Err(AppError::AuthError("Only lobby leader can reset the lobby".into()));
            }
            data.game_status = GameStatus::Lobby;
            data.skip_votes.clear();
            data.return_lobby_votes.clear();
            Ok(data.get_all_players())
        }).await?;
        self.broadcast(shared::ServerMessage::GameState {
            prompt: "".into(), status: GameStatus::Lobby, scores, timer_expires_at: None,
        });
        Ok(())
    }

    pub async fn generate_random_prompt(&self, do_broadcast: bool, reset_timer: bool) -> Result<String> {
        let kanji = Arc::clone(&self.kanji_list);
        let words = Arc::clone(&self.word_list);
        let (text, timer_spawn, timer_expires_at) = self.with_lobby(move |data| {
            let (text, timer_spawn) = generate_prompt_sync(&kanji, &words, data, reset_timer)?;
            Ok((text, timer_spawn, data.timer_expires_at))
        }).await?;

        if let Some((counter, secs)) = timer_spawn {
            let handle = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(secs as u64)).await;
                let _ = handle.process_timeout(counter).await;
            });
        }
        if do_broadcast {
            self.broadcast(shared::ServerMessage::PromptUpdate { new_prompt: text.clone(), timer_expires_at });
        }
        Ok(text)
    }

    pub async fn process_guess(&self, player_id: &PlayerId, input: &str) -> Result<()> {
        let pid = player_id.clone();
        let trimmed = input.trim().to_string();
        let kanji = Arc::clone(&self.kanji_list);
        let words = Arc::clone(&self.word_list);
        let dict = Arc::clone(&self.dict_list);
        let pool = self.db_pool.clone();

        struct Outcome {
            player_list_msg: shared::ServerMessage,
            word_checked_msg: shared::ServerMessage,
            game_over_msg: Option<shared::ServerMessage>,
            timer_spawn: Option<(u64, u32)>,
            db_words: Option<(Arc<crate::db::DbPool>, String)>,
            is_noop: bool,
        }

        let outcome = self.with_lobby(move |data| {
            if data.players.iter().any(|p| p.id == pid && p.is_spectator) {
                return Err(AppError::InvalidInput("Spectators cannot participate".into()));
            }
            if data.game_status != GameStatus::Playing {
                let dummy = shared::ServerMessage::PlayerListUpdate { players: data.get_all_players() };
                return Ok(Outcome { player_list_msg: dummy.clone(), word_checked_msg: dummy, game_over_msg: None, timer_spawn: None, db_words: None, is_noop: true });
            }
            let settings = data.settings.clone();
            if settings.mode == shared::GameMode::Duel {
                let current = data.turn_order.get(data.current_turn_index).cloned();
                if current.as_ref() != Some(&pid) {
                    let dummy = shared::ServerMessage::PlayerListUpdate { players: data.get_all_players() };
                    return Ok(Outcome { player_list_msg: dummy.clone(), word_checked_msg: dummy, game_over_msg: None, timer_spawn: None, db_words: None, is_noop: true });
                }
            }

            let prompt = data.current_prompt.clone()
                .ok_or_else(|| AppError::InternalError("No active prompt".into()))?;
            let is_correct = check_prompt(&prompt, &trimmed, &dict);

            let mut message = String::new();
            let mut new_prompt_text: Option<String> = None;
            let mut game_over = false;
            let mut error_details = None;
            let mut timer_spawn: Option<(u64, u32)> = None;
            let mut db_words: Option<(Arc<crate::db::DbPool>, String)> = None;

            if is_correct {
                if let Some(p) = data.players.iter_mut().find(|p| p.id == pid) {
                    p.score += 1;
                    if let Some(p_ref) = pool.clone() { db_words = Some((p_ref, p.name.clone())); }
                }
                let new_score = data.players.iter().find(|p| p.id == pid).map(|p| p.score).unwrap_or(0);
                match settings.mode {
                    shared::GameMode::Deathmatch => {
                        if let Some(target) = settings.target_score {
                            if new_score >= target { game_over = true; message = "Winner!".into(); }
                            else {
                                message = "Good guess!".into();
                                let (t, ts) = generate_prompt_sync(&kanji, &words, data, true)?;
                                new_prompt_text = Some(t); timer_spawn = ts;
                            }
                        }
                    }
                    shared::GameMode::Duel => {
                        message = "Good guess!".into();
                        let (t, ts) = generate_prompt_sync(&kanji, &words, data, true)?;
                        new_prompt_text = Some(t); timer_spawn = ts;
                        data.reuse_prompt = false;
                        if !data.turn_order.is_empty() {
                            data.current_turn_index = (data.current_turn_index + 1) % data.turn_order.len();
                        }
                    }
                    shared::GameMode::Zen => {
                        message = "Good guess!".into();
                        let (t, ts) = generate_prompt_sync(&kanji, &words, data, true)?;
                        new_prompt_text = Some(t); timer_spawn = ts;
                    }
                }
            } else {
                error_details = get_error_details_sync(data, &dict);
                match &prompt {
                    ActivePrompt::Kanji { character } => {
                        let has_kanji = trimmed.contains(character.as_str());
                        let valid_word = dict.contains(trimmed.as_str());
                        message = if has_kanji { "Bad Guess: Correct kanji, but not a valid word".into() }
                                  else if valid_word { "Bad Guess: Valid word, but does not contain the correct kanji.".into() }
                                  else { "Bad Guess: Incorrect kanji and not a valid word".into() };
                    }
                    ActivePrompt::Vocab { word, .. } => { message = format!("Incorrect reading for {}", word); }
                }
                if settings.mode == shared::GameMode::Duel {
                    let (elim, duel_msg, new_text, ts) = apply_duel_penalty_sync(&kanji, &words, data, &pid, &mut game_over)?;
                    timer_spawn = ts; new_prompt_text = new_text;
                    if elim { message = format!("{}\n{}", message, duel_msg); }
                }
            }

            let score = data.players.iter().find(|p| p.id == pid).map(|p| p.score).unwrap_or(0);
            let game_over_msg = if game_over {
                data.game_status = GameStatus::Finished;
                Some(shared::ServerMessage::GameState {
                    prompt: data.get_current_prompt_text().unwrap_or_default(),
                    status: GameStatus::Finished, scores: data.get_all_players(), timer_expires_at: None,
                })
            } else { None };

            Ok(Outcome {
                player_list_msg: shared::ServerMessage::PlayerListUpdate { players: data.get_all_players() },
                word_checked_msg: shared::ServerMessage::WordChecked {
                    player_id: pid.clone(),
                    result: shared::CheckWordResponse {
                        message, score,
                        error: if !is_correct { Some("Incorrect".into()) } else { None },
                        error_details, prompt: new_prompt_text, timer_expires_at: data.timer_expires_at,
                    },
                },
                game_over_msg, timer_spawn, db_words, is_noop: false,
            })
        }).await?;

        if outcome.is_noop { return Ok(()); }
        self.broadcast(outcome.player_list_msg);
        self.broadcast(outcome.word_checked_msg);
        if let Some(msg) = outcome.game_over_msg { self.broadcast(msg); }
        if let Some((counter, secs)) = outcome.timer_spawn {
            let handle = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(secs as u64)).await;
                let _ = handle.process_timeout(counter).await;
            });
        }
        if let Some((p, name)) = outcome.db_words {
            tokio::spawn(async move {
                let _ = crate::models::GlobalStats::increment_words(&p).await;
                let _ = crate::models::User::increment_words_guessed(&p, &name, 1).await;
            });
        }
        Ok(())
    }

    pub async fn process_skip(&self, player_id: &PlayerId) -> Result<()> {
        let pid = player_id.clone();
        let kanji = Arc::clone(&self.kanji_list);
        let words = Arc::clone(&self.word_list);
        let dict = Arc::clone(&self.dict_list);

        struct SkipOutcome {
            msgs: Vec<shared::ServerMessage>,
            game_over_msg: Option<shared::ServerMessage>,
            timer_spawn: Option<(u64, u32)>,
        }

        let outcome = self.with_lobby(move |data| {
            if data.players.iter().any(|p| p.id == pid && p.is_spectator) {
                return Err(AppError::InvalidInput("Spectators cannot participate".into()));
            }
            if data.game_status != GameStatus::Playing {
                return Ok(SkipOutcome { msgs: vec![], game_over_msg: None, timer_spawn: None });
            }

            let settings = data.settings.clone();
            let error_details = get_error_details_sync(data, &dict);

            if settings.mode == shared::GameMode::Duel {
                let current = data.turn_order.get(data.current_turn_index).cloned();
                if current.as_ref() != Some(&pid) {
                    return Ok(SkipOutcome { msgs: vec![], game_over_msg: None, timer_spawn: None });
                }
                let mut game_over = false;
                let (elim, duel_msg, new_text, timer_spawn) =
                    apply_duel_penalty_sync(&kanji, &words, data, &pid, &mut game_over)?;
                let message = if elim { format!("Skipped!\n{}", duel_msg) } else { "Skipped!".into() };
                let score = data.players.iter().find(|p| p.id == pid).map(|p| p.score).unwrap_or(0);
                let game_over_msg = if game_over {
                    data.game_status = GameStatus::Finished;
                    Some(shared::ServerMessage::GameState {
                        prompt: data.get_current_prompt_text().unwrap_or_default(),
                        status: GameStatus::Finished, scores: data.get_all_players(), timer_expires_at: None,
                    })
                } else { None };
                Ok(SkipOutcome {
                    msgs: vec![
                        shared::ServerMessage::PlayerListUpdate { players: data.get_all_players() },
                        shared::ServerMessage::WordChecked { player_id: pid.clone(), result: shared::CheckWordResponse {
                            message, score, error: Some("Skipped!".into()), error_details,
                            prompt: new_text, timer_expires_at: data.timer_expires_at,
                        }},
                    ],
                    game_over_msg, timer_spawn,
                })
            } else {
                data.skip_votes.insert(pid.clone());
                let total = data.players.iter().filter(|p| !p.is_eliminated && !p.is_spectator).count();
                let required = (total / 2) + 1;
                let votes = data.skip_votes.len();
                if votes >= required {
                    let (text, timer_spawn) = generate_prompt_sync(&kanji, &words, data, true)?;
                    Ok(SkipOutcome {
                        msgs: vec![shared::ServerMessage::WordChecked { player_id: PlayerId::default(), result: shared::CheckWordResponse {
                            message: "Prompt skipped by vote!".into(), score: 0,
                            error: Some("Skipped!".into()), error_details, prompt: Some(text),
                            timer_expires_at: data.timer_expires_at,
                        }}],
                        game_over_msg: None, timer_spawn,
                    })
                } else {
                    Ok(SkipOutcome { msgs: vec![shared::ServerMessage::SkipVoteUpdate { votes, required }], game_over_msg: None, timer_spawn: None })
                }
            }
        }).await?;

        for msg in outcome.msgs { self.broadcast(msg); }
        if let Some(msg) = outcome.game_over_msg { self.broadcast(msg); }
        if let Some((counter, secs)) = outcome.timer_spawn {
            let handle = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(secs as u64)).await;
                let _ = handle.process_timeout(counter).await;
            });
        }
        Ok(())
    }

    pub async fn process_return_lobby_vote(&self, player_id: &PlayerId) -> Result<()> {
        let pid = player_id.clone();
        let (vote_passed, scores, score_for_msg) = self.with_lobby(move |data| {
            if data.players.iter().any(|p| p.id == pid && p.is_spectator) {
                return Err(AppError::InvalidInput("Spectators cannot participate".into()));
            }
            if data.game_status != GameStatus::Playing {
                return Ok((false, vec![], (0usize, 0usize, 0u32)));
            }
            data.return_lobby_votes.insert(pid.clone());
            let total = data.players.iter().filter(|p| !p.is_eliminated && !p.is_spectator).count();
            let required = (total / 2) + 1;
            let votes = data.return_lobby_votes.len();
            let score = data.players.iter().find(|p| p.id == pid).map(|p| p.score).unwrap_or(0);

            if votes >= required {
                data.return_lobby_votes.clear();
                data.skip_votes.clear();
                data.game_status = GameStatus::Lobby;
                Ok((true, data.get_all_players(), (votes, required, score)))
            } else {
                Ok((false, vec![], (votes, required, score)))
            }
        }).await?;

        let (votes, required, score) = score_for_msg;
        if vote_passed {
            self.broadcast(shared::ServerMessage::GameState {
                prompt: "".into(), status: GameStatus::Lobby, scores, timer_expires_at: None,
            });
        } else {
            self.broadcast(shared::ServerMessage::WordChecked {
                player_id: PlayerId::default(),
                result: shared::CheckWordResponse {
                    message: format!("Return to Lobby vote registered ({}/{})", votes, required),
                    score,
                    error: Some(format!("Return to Lobby vote registered ({}/{})", votes, required)),
                    error_details: None, prompt: None, timer_expires_at: None,
                },
            });
        }
        Ok(())
    }

    // Regular fn (not async) returning a boxed Send future. This breaks the recursive
    // Send-proof cycle: when the spawn inside calls `handle.process_timeout(counter)`,
    // the return type is `Pin<Box<dyn Future + Send>>` — already known to be Send —
    // so the compiler doesn't need to recursively prove it.
    pub fn process_timeout(&self, expected_counter: u64) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'static>> {
        let this = self.clone();
        Box::pin(async move {
            let kanji = Arc::clone(&this.kanji_list);
            let words = Arc::clone(&this.word_list);
            let dict = Arc::clone(&this.dict_list);

            struct TimeoutOutcome {
                msgs: Vec<shared::ServerMessage>,
                game_over_msg: Option<shared::ServerMessage>,
                timer_spawn: Option<(u64, u32)>,
            }

            let outcome = this.with_lobby(move |data| {
                if data.prompt_counter != expected_counter || data.game_status != GameStatus::Playing {
                    return Ok(TimeoutOutcome { msgs: vec![], game_over_msg: None, timer_spawn: None });
                }
                let settings = data.settings.clone();
                let error_details = get_error_details_sync(data, &dict);

                if settings.mode == shared::GameMode::Duel {
                    let player_id = match data.turn_order.get(data.current_turn_index).cloned() {
                        Some(id) => id,
                        None => return Ok(TimeoutOutcome { msgs: vec![], game_over_msg: None, timer_spawn: None }),
                    };
                    let mut game_over = false;
                    let (elim, duel_msg, new_text, timer_spawn) =
                        apply_duel_penalty_sync(&kanji, &words, data, &player_id, &mut game_over)?;
                    let message = if elim { format!("Time's up!\n{}", duel_msg) } else { "Time's up!".into() };
                    let score = data.players.iter().find(|p| p.id == player_id).map(|p| p.score).unwrap_or(0);
                    let game_over_msg = if game_over {
                        data.game_status = GameStatus::Finished;
                        Some(shared::ServerMessage::GameState {
                            prompt: data.get_current_prompt_text().unwrap_or_default(),
                            status: GameStatus::Finished, scores: data.get_all_players(), timer_expires_at: None,
                        })
                    } else { None };
                    Ok(TimeoutOutcome {
                        msgs: vec![
                            shared::ServerMessage::PlayerListUpdate { players: data.get_all_players() },
                            shared::ServerMessage::WordChecked { player_id: player_id.clone(), result: shared::CheckWordResponse {
                                message, score, error: Some("Time's up!".into()), error_details,
                                prompt: new_text, timer_expires_at: data.timer_expires_at,
                            }},
                        ],
                        game_over_msg, timer_spawn,
                    })
                } else {
                    let (text, timer_spawn) = generate_prompt_sync(&kanji, &words, data, true)?;
                    Ok(TimeoutOutcome {
                        msgs: vec![shared::ServerMessage::WordChecked { player_id: PlayerId::default(), result: shared::CheckWordResponse {
                            message: "Time's up! Skipped prompt.".into(), score: 0,
                            error: Some("Time's up!".into()), error_details, prompt: Some(text),
                            timer_expires_at: data.timer_expires_at,
                        }}],
                        game_over_msg: None, timer_spawn,
                    })
                }
            }).await?;

            for msg in outcome.msgs { this.broadcast(msg); }
            if let Some(msg) = outcome.game_over_msg { this.broadcast(msg); }
            if let Some((counter, secs)) = outcome.timer_spawn {
                let handle = this.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(secs as u64)).await;
                    // process_timeout returns Pin<Box<dyn Future + Send>> — no inference cycle.
                    let _ = handle.process_timeout(counter).await;
                });
            }
            Ok(())
        })
    }
}

// ── Free helpers (sync, called inside with_lobby closures) ───────────────────

fn is_leader(data: &LobbyData, player_id: &PlayerId) -> bool {
    data.lobby_leader.to_string() == player_id.to_string()
}

fn generate_prompt_sync(
    kanji_list: &KanjiData,
    word_list: &JlptWordData,
    data: &mut LobbyData,
    reset_timer: bool,
) -> Result<(String, Option<(u64, u32)>)> {
    let mut rng = rand::rng();
    if data.active_level_indices.is_empty() {
        return Err(AppError::InternalError("No active levels configured".into()));
    }
    let level_idx = data.active_level_indices[rng.random_range(0..data.active_level_indices.len())];

    let text = match data.settings.content_mode.clone() {
        ContentMode::Kanji => {
            let list = &kanji_list[level_idx];
            let kanji = if data.settings.weighted {
                let weights: Vec<f64> = list.iter()
                    .map(|k| if k.frequency > 0 { k.frequency as f64 } else { 0.0 }).collect();
                if let Ok(dist) = WeightedIndex::new(&weights) {
                    list[dist.sample(&mut rng)].clone()
                } else { list[rng.random_range(0..list.len())].clone() }
            } else { list[rng.random_range(0..list.len())].clone() };
            let t = kanji.kanji.clone();
            data.current_prompt = Some(ActivePrompt::Kanji { character: t.clone() });
            t
        }
        ContentMode::Vocab => {
            let word_map = &word_list[level_idx];
            let keys: Vec<&String> = word_map.keys().collect();
            let key = keys[rng.random_range(0..keys.len())].clone();
            let readings = word_map[&key].clone();
            data.current_prompt = Some(ActivePrompt::Vocab { word: key.clone(), readings });
            key
        }
    };

    let timer_spawn = if reset_timer {
        data.prompt_counter += 1;
        data.skip_votes.clear();
        if let Some(secs) = data.settings.time_limit_seconds {
            let counter = data.prompt_counter;
            let expires = Utc::now().timestamp_millis() as u64 + (secs as u64 * 1000);
            data.timer_expires_at = Some(expires);
            Some((counter, secs))
        } else { data.timer_expires_at = None; None }
    } else { None };

    Ok((text, timer_spawn))
}

fn apply_duel_penalty_sync(
    kanji_list: &KanjiData,
    word_list: &JlptWordData,
    data: &mut LobbyData,
    player_id: &PlayerId,
    game_over: &mut bool,
) -> Result<(bool, String, Option<String>, Option<(u64, u32)>)> {
    let eliminated = if let Some(p) = data.players.iter_mut().find(|p| p.id == *player_id) {
        if let Some(lives) = p.lives.as_mut() {
            if *lives > 0 { *lives -= 1; }
            if *lives == 0 { p.is_eliminated = true; true } else { false }
        } else { false }
    } else { false };

    let msg = if eliminated { "Eliminated!".to_string() } else { String::new() };
    let mut new_prompt_text = None;
    let mut timer_spawn = None;

    if data.settings.duel_allow_kanji_reuse {
        if data.reuse_prompt {
            let (t, ts) = generate_prompt_sync(kanji_list, word_list, data, true)?;
            new_prompt_text = Some(t); timer_spawn = ts;
            data.reuse_prompt = false;
        } else { data.reuse_prompt = true; }
    } else {
        let (t, ts) = generate_prompt_sync(kanji_list, word_list, data, true)?;
        new_prompt_text = Some(t); timer_spawn = ts;
    }

    if eliminated {
        if let Some(pos) = data.turn_order.iter().position(|id| id == player_id) {
            data.turn_order.remove(pos);
            if data.current_turn_index >= data.turn_order.len() && !data.turn_order.is_empty() {
                data.current_turn_index = 0;
            }
        }
    } else if !data.turn_order.is_empty() {
        data.current_turn_index = (data.current_turn_index + 1) % data.turn_order.len();
    }

    if data.turn_order.len() <= 1 { *game_over = true; }

    Ok((eliminated, msg, new_prompt_text, timer_spawn))
}

fn get_error_details_sync(data: &LobbyData, dict_list: &DictData) -> Option<Vec<String>> {
    match &data.current_prompt {
        Some(ActivePrompt::Vocab { readings, .. }) => Some(readings.clone()),
        Some(ActivePrompt::Kanji { character }) => {
            let mut matches = Vec::new();
            for w in dict_list.iter() {
                if w.contains(character.as_str()) { matches.push(w.clone()); if matches.len() >= 3 { break; } }
            }
            Some(matches)
        }
        None => None,
    }
}
