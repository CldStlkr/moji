pub mod api;
pub mod data;
pub mod db;
pub mod error;
pub mod models;
pub mod types;

use chrono::{DateTime, Utc};
use data::{vectorize_joyo_kanji, load_dictionary, load_jlpt_words, JlptWordData, KanjiData, DictData};
use db::DbPool;
use error::AppError;
use rand::{RngExt, distr::{Alphanumeric, Distribution, weighted::WeightedIndex}};
use shared::{ContentMode, ActivePrompt};
use tokio::sync::broadcast;
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};

pub use shared::{
    CheckWordResponse, GameSettings, GameStatus, JoinLobbyRequest, PlayerId, ApiContext,
};
pub use types::{Result, Shared, SharedState};



#[derive(Clone, Debug)]
pub struct PlayerData {
    pub id: PlayerId,
    pub name: String,
    pub score: u32,
    pub joined_at: DateTime<Utc>,
    pub lives: Option<u32>,
    pub is_eliminated: bool,
}

pub struct AppState {
    pub lobbies: Shared<HashMap<String, SharedState>>,
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


        let list_of_kanji = Arc::new(vectorize_joyo_kanji(&kanji_list_paths)
            .map_err(|e| AppError::DataLoadError(e.to_string()))?);
        let list_of_words = Arc::new(load_jlpt_words(&word_list_paths)
            .map_err(|e| AppError::DataLoadError(e.to_string()))?);
        let dictionary_list = Arc::new(load_dictionary(&dictionary_path)
            .map_err(|e| AppError::DataLoadError(e.to_string()))?);

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

#[derive(Clone)]
pub struct LobbyState {
    pub kanji_list: Arc<KanjiData>,
    pub word_list: Arc<JlptWordData>,
    pub dict_list: Arc<DictData>,
    pub players: Shared<Vec<PlayerData>>,
    pub lobby_leader: Shared<PlayerId>,
    pub settings: Shared<GameSettings>,
    pub game_status: Shared<GameStatus>,
    pub current_prompt: Shared<Option<ActivePrompt>>,
    pub tx: broadcast::Sender<String>,
    pub active_level_indices: Shared<Vec<usize>>,
    pub level_weights: Shared<HashMap<usize, WeightedIndex<f64>>>,
    pub game_session_id: Option<uuid::Uuid>,
    pub turn_order: Shared<Vec<PlayerId>>,
    pub current_turn_index: Shared<usize>,
}

impl LobbyState {
        pub fn new(kanji_list: Arc<KanjiData>, word_list: Arc<JlptWordData>,
        dict_list: Arc<DictData>, game_session_id: Option<uuid::Uuid>) -> Self {
        Self {
            kanji_list,
            word_list,
            dict_list,
            players: Shared::new(Vec::new()),
            lobby_leader: Shared::new(PlayerId::default()),
            settings: Shared::new(GameSettings::default()),
            game_status: Shared::new(GameStatus::Lobby),
            current_prompt: Shared::new(None),
            tx: broadcast::channel(100).0, // .0 = Sender, .1 = Receiver
            active_level_indices: Shared::new(Vec::new()),
            level_weights: Shared::new(HashMap::new()),
            game_session_id,
            turn_order: Shared::new(Vec::new()),
            current_turn_index: Shared::new(0),
        }
    }

    pub fn is_leader(&self, player_id: &PlayerId) -> Result<bool> {
        self.lobby_leader.read(|leader| {
            leader.to_string() == player_id.to_string()
        })
    }

    pub fn update_settings(&self, player_id: &PlayerId, new_settings: GameSettings) -> Result<()> {
        if !self.is_leader(player_id)? {
            return Err(AppError::AuthError(
                "Only lobby leader can change settings".to_string(),
            ));
        }

        self.settings.write(|settings| {
            *settings = new_settings.clone();
        })?;

        let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::SettingsUpdate {
            settings: new_settings
        }).unwrap_or_default());

        Ok(())
    }

    pub fn get_lobby_info(&self, lobby_id: &str) -> Result<shared::LobbyInfo> {
        let status = self.game_status.read(|s| *s)?;
        let settings = self.settings.read(|s| s.clone())?;
        let leader = self.lobby_leader.read(|l| l.clone())?;

        let current_turn = self.turn_order.read(|order| {
             self.current_turn_index.read(|idx| order.get(*idx).cloned())
        })??;

        let api_players = self.players.read(|players| {
             players.iter()
            .map(|p| shared::PlayerData {
                id: p.id.clone(),
                name: p.name.clone(),
                score: p.score,
                joined_at: p.joined_at.to_rfc3339(),
                lives: p.lives,
                is_eliminated: p.is_eliminated,
                is_turn: current_turn.as_ref() == Some(&p.id) && status == GameStatus::Playing && settings.mode == shared::GameMode::Duel,
            })
            .collect::<Vec<_>>()
        })?;

        Ok(shared::LobbyInfo {
            lobby_id: lobby_id.to_string(),
            leader_id: leader,
            players: api_players,
            settings,
            status,
        })
    }

    pub fn start_game(&self, player_id: &PlayerId) -> Result<()> {
        if !self.is_leader(player_id)? {
            return Err(AppError::AuthError(
                "Only lobby leader can start the game".to_string(),
            ))?;
        }

        self.game_status.write(|status| {
            if *status != GameStatus::Lobby {
                return Err(AppError::InvalidInput("game is not in lobby state".to_string()));
            }
            *status = GameStatus::Playing;
            Ok::<(), AppError>(())
        })??;

        let settings = self.settings.read(|s| s.clone())?;

        {
            let levels = &settings.difficulty_levels;
            let weighted = settings.weighted;

            let mut indices: Vec<usize> = Vec::new();
            for level in levels {
                let idx = match level.as_str() {
                    "N1" => 0, "N2" => 1, "N3" => 2, "N4" => 3, "N5" => 4, _ => 99
                };

                if idx < self.kanji_list.len()  && !self.kanji_list[idx].is_empty() {
                    indices.push(idx);
                }
            }

            if indices.is_empty() && self.kanji_list.len() > 4 {
                indices.push(4);
            }

            let mut w_map = HashMap::new();
            if weighted {
                for &idx in &indices {
                    let list = &self.kanji_list[idx];
                    let weights: Vec<f64> = list.iter()
                        .map(|k| if k.frequency > 0 { k.frequency as f64 } else { 0.0 })
                        .collect();

                    if let Ok(dist) = WeightedIndex::new(&weights) {
                        w_map.insert(idx, dist);
                    }
                }
            }

            self.active_level_indices.write(|ai| *ai = indices)?;
            self.level_weights.write(|lw| *lw = w_map)?;

            self.players.write(|players| {
                self.turn_order.write(|turn_order| {
                     self.current_turn_index.write(|idx| {
                        *idx = 0;
                        turn_order.clear();

                        for p in players.iter_mut() {
                            p.score = 0;
                            p.is_eliminated = false;
                            if settings.mode == shared::GameMode::Duel {
                                p.lives = settings.initial_lives;
                                turn_order.push(p.id.clone());
                            } else {
                                p.lives = None;
                            }
                        }

                        if settings.mode == shared::GameMode::Duel {
                            use rand::seq::SliceRandom;
                            let mut rng = rand::rng();
                            turn_order.shuffle(&mut rng);
                        }
                        Ok::<(), AppError>(())
                     })
                })
            })????;
        }

        self.generate_random_prompt(false)?;

        let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::GameState {
            prompt: self.get_current_prompt_text()?.unwrap_or_default(),
            status: GameStatus::Playing,
            scores: self.get_all_players()?,
        }).unwrap_or_default());

        Ok(())
    }

    pub fn add_player(&self, player_id: PlayerId, player_name: String) -> Result<bool> {
        let is_leader_result = self.players.write(|players| {
            let is_leader = players.is_empty();
            if is_leader {
                 self.lobby_leader.write(|leader| *leader = player_id.clone())?;
            }

            let trimmed_name = player_name.trim();
            if trimmed_name.is_empty() {
                return Err(AppError::InvalidInput("Player name cannot be empty".to_string()));
            }

            let normalized_name = trimmed_name.split_whitespace().collect::<Vec<&str>>().join(" ");


            // Allow duplicate names? Original didn't check. 
            // We'll proceed with adding the player.


            players.push(PlayerData {
                id: player_id.clone(),
                name: normalized_name,
                score: 0,
                joined_at: Utc::now(),
                lives: None,
                is_eliminated: false,
            });
            Ok(is_leader)
        })??;

        let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::PlayerListUpdate {
            players: self.get_all_players()?,
        }).unwrap_or_default());

        Ok(is_leader_result)
    }

    pub fn remove_player(&self, player_id: &PlayerId) -> Result<bool> {
        self.players.write(|players| {
            if let Some(pos) = players.iter().position(|p| &p.id == player_id) {
                let p_id = players[pos].id.clone();
                players.remove(pos);

                self.turn_order.write(|turn_order| {
                    if let Some(t_pos) = turn_order.iter().position(|id| id == &p_id) {
                         turn_order.remove(t_pos);
                         self.current_turn_index.write(|idx| {
                             if *idx >= turn_order.len() && !turn_order.is_empty() {
                                 *idx = 0;
                             }
                             Ok::<(), AppError>(())
                         })?
                    } else {
                        Ok::<(), AppError>(())
                    }
                })??;

                self.lobby_leader.write(|leader| {
                    if leader.to_string() == player_id.to_string() {
                        if let Some(new_leader) = players.first() {
                            *leader = new_leader.id.clone();
                            let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::LeaderUpdate {
                                leader_id: new_leader.id.clone()
                            }).unwrap_or_default());
                        } else {
                            *leader = PlayerId::default(); 
                        }
                    }
                    Ok::<(), AppError>(())
                })??;

                let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::PlayerListUpdate {
                    players: players.iter().map(|p| shared::PlayerData {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        score: p.score,
                        joined_at: p.joined_at.to_rfc3339(),
                        lives: p.lives,
                        is_eliminated: p.is_eliminated,
                        is_turn: false,
                    }).collect()
                }).unwrap_or_default());

                Ok(true)
            } else {
                Ok(false)
            }
        })?
    }

    pub fn get_player_score(&self, player_id: &PlayerId) -> Result<u32> {
        self.players.read(|players| {
            players
                .iter()
                .find(|p| &p.id == player_id)
                .map(|p| p.score)
                .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))
        })?
    }

    pub fn get_player_name(&self, player_id: &PlayerId) -> Result<String> {
        self.players.read(|players| {
            players
                .iter()
                .find(|p| &p.id == player_id)
                .map(|p| p.name.clone())
                .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))
        })?
    }

    pub fn increment_player_score(&self, player_id: &PlayerId) -> Result<u32> {
        self.players.write(|players| {
             let player = players
                .iter_mut()
                .find(|p| &p.id == player_id)
                .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))?;

            player.score += 1;
            Ok(player.score)
        })?
    }

    // Get all players and scores (for a leaderboard)
    pub fn get_all_players(&self) -> Result<Vec<shared::PlayerData>> {
        let status = self.game_status.read(|s| *s)?;
        let settings = self.settings.read(|s| s.clone())?;

        // Lock players and potentially turn_order/current_turn_index
        self.players.read(|players| {
             let mut shared_players: Vec<shared::PlayerData> = players.iter().map(|p| shared::PlayerData {
                id: p.id.clone(),
                name: p.name.clone(),
                score: p.score,
                joined_at: p.joined_at.to_rfc3339(),
                lives: p.lives,
                is_eliminated: p.is_eliminated,
                is_turn: false, // Default to false here
            }).collect();

            if settings.mode == shared::GameMode::Duel && status == GameStatus::Playing {
                self.turn_order.read(|order| {
                    self.current_turn_index.read(|idx| {
                         if let Some(current_id) = order.get(*idx) {
                             for p in &mut shared_players {
                                 if p.id.to_string() == current_id.to_string() {
                                     p.is_turn = true;
                                 }
                             }
                         }
                         Ok::<(), AppError>(())
                    })
                })???;
            }
            Ok(shared_players)
        })?
    }

    pub fn get_current_prompt_text(&self) -> Result<Option<String>> {
        self.current_prompt.read(|p| p.as_ref().map(|prompt| prompt.display_text().to_string()))
    }

    /// Generate a new random kanji and store it as current.
    /// If `broadcast` is true, a `PromptUpdate` WS message is sent to all clients.
    /// Pass `false` when the caller will send a more complete message (e.g. `GameState`).
    pub fn generate_random_prompt(&self, broadcast: bool) -> Result<String> {
        let content_mode = self.settings.read(|s| s.content_mode.clone())?;
        let mut rng = rand::rng();
        let indices = self.active_level_indices.read(|i| i.clone())?;

        if indices.is_empty() {
            return Err(AppError::InternalError("No active levels configured".into()));
        }

        let level_idx = indices[rng.random_range(0..indices.len())];

        let display_text = match content_mode {
            ContentMode::Kanji => {
                let weights_map = self.level_weights.read(|w| w.clone())?;
                let kanji_list = &self.kanji_list[level_idx];

                let kanji = if let Some(dist) = weights_map.get(&level_idx) {
                    kanji_list[dist.sample(&mut rng)].clone()
                } else {
                    kanji_list[rng.random_range(0..kanji_list.len())].clone()
                };

                let prompt = ActivePrompt::Kanji { character: kanji.kanji.clone() };
                let text = kanji.kanji;
                self.current_prompt.write(|p| *p = Some(prompt))?;

                text

            },
            ContentMode::Vocab => {
                let word_map = &self.word_list[level_idx];

                // Pick random word from map
                let keys = word_map.keys().collect::<Vec<&String>>();
                let word_key = keys[rng.random_range(0..keys.len())];
                let readings = word_map[word_key].clone();

                let prompt = ActivePrompt::Vocab {
                    word: word_key.clone(),
                    readings,
                };
                let text = word_key.clone();
                self.current_prompt.write(|p| *p = Some(prompt))?;

                text
            }
        };

        if broadcast {
            let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::PromptUpdate {
                new_prompt: display_text.clone(),
            }).unwrap_or_default());
        }

        Ok(display_text)
    }

    pub fn reset_lobby(&self, player_id: &PlayerId) -> Result<()> {
        if !self.is_leader(player_id)? {
            return Err(AppError::AuthError(
                "Only lobby leader can reset the lobby".to_string(),
            ));
        }

        self.game_status.write(|status| *status = GameStatus::Lobby)?;

        let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::GameState {
            prompt: "".to_string(),
            status: GameStatus::Lobby,
            scores: self.get_all_players()?,
        }).unwrap_or_default());

        Ok(())
    }

    pub fn advance_turn(&self) -> Result<PlayerId> {
        self.turn_order.write(|order| {
             self.current_turn_index.write(|idx| {
                 if order.is_empty() {
                     return Err(AppError::InternalError("No players in turn order".to_string()));
                 }
                 *idx = (*idx + 1) % order.len();
                 Ok(order[*idx].clone())
             })
        })??
    }

    pub fn get_current_turn_player(&self) -> Result<Option<PlayerId>> {
        self.turn_order.read(|order| {
             self.current_turn_index.read(|idx| {
                 order.get(*idx).cloned()
             })
        })?
    }

    pub fn process_guess(&self, player_id: &PlayerId, input: &str) -> Result<()> {
        let (settings, status) = {
             let st = self.game_status.read(|s| *s)?;
             let s = self.settings.read(|s| s.clone())?;
             (s, st)
        };

        if status != GameStatus::Playing {
            return Ok(());
        }

        if settings.mode == shared::GameMode::Duel {
            let current_turn = self.get_current_turn_player()?;
            if current_turn.as_ref() != Some(player_id) {
                return Ok(()); // Not your turn
            }
        }


        let trimmed_input = input.trim();
        let prompt = self.current_prompt.read(|p| p.clone())?
            .ok_or(AppError::InternalError("No active prompt".into()))?;


        let is_correct = check_prompt(&prompt, trimmed_input, &self.dict_list);

        let mut message = String::new();
        let mut new_prompt_opt = None;
        let mut game_over = false;

        if is_correct {
            let new_score = self.increment_player_score(player_id)?;

            if settings.mode == shared::GameMode::Deathmatch {
                if let Some(target) = settings.target_score {
                    if new_score >= target {
                        game_over = true;
                        message = "Winner!".to_string();
                    } else {
                        message = "Good guess!".to_string();
                        let _ = self.generate_random_prompt(true);
                        new_prompt_opt = self.get_current_prompt_text().ok().flatten();
                    }
                }
            } else if settings.mode == shared::GameMode::Duel {
                message = "Good guess!".to_string();
                let _ = self.generate_random_prompt(true);
                new_prompt_opt = self.get_current_prompt_text().ok().flatten();
                let _ = self.advance_turn();
            }
        } else {
            match &prompt {
                ActivePrompt::Kanji { character } => {
                    let has_kanji = trimmed_input.contains(character.as_str());
                    let valid_word = self.dict_list.contains(trimmed_input);
                    if has_kanji {
                        message = "Bad Guess: Correct kanji, but not a valid word".to_string();
                    } else if valid_word {
                        message = "Bad Guess: Valid word, but does not contain the correct kanji.".to_string();
                    } else {
                        message = "Bad Guess: Incorrect kanji and not a valid word".to_string();
                    }
                },
                ActivePrompt::Vocab { word, .. } => { message = format!("Incorrect reading for {}", word); }
            }
            if settings.mode == shared::GameMode::Duel {
                let eliminated = self.players.write(|players| {
                     let mut eliminated = false;
                     if let Some(p) = players.iter_mut().find(|p| p.id == *player_id) {
                         if let Some(lives) = p.lives.as_mut() {
                             if *lives > 0 {
                                 *lives -= 1;
                             }
                             if *lives == 0 {
                                 p.is_eliminated = true;
                                 eliminated = true;
                             }
                         }
                     }
                     eliminated
                })?;
                if eliminated {
                     message = "Eliminated!".to_string();
                }

                if !settings.duel_allow_kanji_reuse {
                    let _ = self.generate_random_prompt(true);
                    new_prompt_opt = self.get_current_prompt_text().ok().flatten();
                }

                if eliminated {
                     self.turn_order.write(|order| {
                         if let Some(pos) = order.iter().position(|id| id == player_id) {
                             order.remove(pos);
                             self.current_turn_index.write(|idx| {
                                 if *idx >= order.len() && !order.is_empty() {
                                     *idx = 0;
                                 }
                                 Ok::<(), AppError>(())
                             })?
                         } else {
                             Ok::<(), AppError>(())
                         }
                     })??;
                } else {
                     let _ = self.advance_turn();
                }
                let order_len = self.turn_order.read(|o| o.len())?;
                if order_len <= 1 {
                    game_over = true;
                    if !eliminated {
                         message = "Winner!".to_string();
                    }
                }
            }
        }

        let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::PlayerListUpdate {
            players: self.get_all_players().unwrap_or_default()
        }).unwrap_or_default());

        let score = self.get_player_score(player_id).unwrap_or(0);
        let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::WordChecked {
            player_id: player_id.clone(),
            result: shared::CheckWordResponse {
                message,
                score,
                error: None,
                prompt: new_prompt_opt,
            },
        }).unwrap_or_default());

        if game_over {
            self.game_status.write(|st| *st = GameStatus::Finished)?;
            let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::GameState {
                prompt: self.get_current_prompt_text()?.unwrap_or_default(),
                status: GameStatus::Finished,
                scores: self.get_all_players()?,
            }).unwrap_or_default());
        }

        Ok(())
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
    app_state.lobbies.write(|lobbies| {
        lobbies
            .get(lobby_id)
            .cloned()
            .ok_or_else(|| AppError::LobbyNotFound(lobby_id.to_string()))
    })?
}

fn check_prompt(prompt: &shared::ActivePrompt, input: &str, dictionary: &HashSet<String>) -> bool {
    match prompt {
        ActivePrompt::Kanji { character } => {
            input.contains(character.as_str()) && dictionary.contains(input)
        },
        ActivePrompt::Vocab { readings, .. } => {
            readings.iter().any(|r| r == input)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use data::Kanji;

    fn create_test_lobby() -> LobbyState {
        let test_kanji_list = Arc::new(vec![
            vec![
                Kanji { kanji: "日".to_string(), frequency: 0 },
                Kanji { kanji: "月".to_string(), frequency: 0 },
                Kanji { kanji: "屈".to_string(), frequency: 0 },
                Kanji { kanji: "理".to_string(), frequency: 0 },
                Kanji { kanji: "総".to_string(), frequency: 0 },
                Kanji { kanji: "辱".to_string(), frequency: 0 },
                Kanji { kanji: "酷".to_string(), frequency: 0 },
                Kanji { kanji: "関".to_string(), frequency: 0 },
                Kanji { kanji: "糸".to_string(), frequency: 0 },
                Kanji { kanji: "木".to_string(), frequency: 0 },
            ],
        ]);
        let test_dict_list = Arc::new(HashSet::from([
            "日本".to_string(),
            "弄り回す".to_string(),
            "月曜日".to_string(),
            "比律賓".to_string(),
            "哀歌".to_string(),
            "猥ら".to_string(),
            "育ち".to_string(),
            "縁語".to_string(),
            "炎".to_string(),
            "渦紋".to_string(),
        ]));
        let test_words_list = Arc::new(vec![
            {
                let mut map = HashMap::new();
                map.insert("日本".to_string(), vec!["にほん".to_string(), "にっぽん".to_string()]);
                map.insert("月曜日".to_string(), vec!["げつようび".to_string()]);
                map.insert("木曜日".to_string(), vec!["もくようび".to_string()]);
                map.insert("日記".to_string(), vec!["にっき".to_string()]);
                map.insert("理由".to_string(), vec!["りゆう".to_string()]);
                map
            },
        ]);

        LobbyState::new(test_kanji_list, test_words_list, test_dict_list, None)
    }

    #[test]
    fn test_generate_lobby_id() {
        let id = generate_lobby_id();
        assert_eq!(id.len(), 6);
        // Check that ID is alphanumeric
        assert!(id.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_increment_player_score() {
        let lobby_state = create_test_lobby();
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
    fn test_get_current_prompt_text() {
        let lobby_state = create_test_lobby();

        // Initially should be None
        assert_eq!(lobby_state.get_current_prompt_text().unwrap(), None);

        // Generate a kanji and verify it's set
        // NOTE: New generate_random_prompt requires start_game to populate active_indices or manually setting them
        // Manually set them for the test
        {
            lobby_state.active_level_indices.write(|indices| indices.push(0)).unwrap();
        }

        let kanji = lobby_state.generate_random_prompt(false).unwrap();
        assert_eq!(lobby_state.get_current_prompt_text().unwrap(), Some(kanji));
    }

    #[test]
    fn test_generate_random_prompt() {
        let lobby_state = create_test_lobby();

        // Set active indices
        {
             lobby_state.active_level_indices.write(|indices| indices.push(0)).unwrap();
        }

        // Generate a kanji and verify it's from one of the lists
        let kanji = lobby_state.generate_random_prompt(false).unwrap();
        let kanji_exists = lobby_state.kanji_list
            .iter()
            .flatten()
            .any(|k| k.kanji == kanji);
        assert!(kanji_exists);

        // Generate another and ensure it's set as current
        let kanji2 = lobby_state.generate_random_prompt(false).unwrap();
        assert_eq!(lobby_state.get_current_prompt_text().unwrap(), Some(kanji2));
    }

    #[test]
    fn test_get_all_players() {
        let lobby_state = create_test_lobby();

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
        let lobby_state = create_test_lobby();

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
        let app_state = Arc::new(AppState::create().expect("Failed to create AppState"));

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
        let app_state = Arc::new(AppState::create().expect("Failed to create AppState"));

        // Create a lobby and add it to the state
        let lobby_id = generate_lobby_id();
        let lobby_state = Arc::new(create_test_lobby());

        {
             app_state.lobbies.write(|lobbies| {
                 lobbies.insert(lobby_id.clone(), lobby_state.clone());
             }).unwrap();
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

        // Manually start game or set indices to allow generation
        {
             retrieved_lobby.active_level_indices.write(|indices| indices.push(0)).unwrap();
        }

        // Generate kanji and check word
        let _kanji = retrieved_lobby.generate_random_prompt(false).unwrap();

        // Verify players and scores
        let players = retrieved_lobby.get_all_players().unwrap();
        assert_eq!(players.len(), 2);
    }

    #[test]
    fn test_lobby_leader_functionality() {
        let lobby_state = create_test_lobby();

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
        let lobby_state = create_test_lobby();

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
            weighted: false,
            ..Default::default()
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
        let lobby_state = create_test_lobby();

        lobby_state
            .add_player(PlayerId::from("leader"), "Leader".to_string())
            .unwrap();
        lobby_state
            .add_player(PlayerId::from("player"), "Player".to_string())
            .unwrap();

        // Leader can start game
        assert!(lobby_state.start_game(&PlayerId::from("leader")).is_ok());

        // Game status should change to Playing
        let status = lobby_state.game_status.read(|s| *s).unwrap();
        assert_eq!(status, GameStatus::Playing);
    }

    // ── Player management ───────────────────────────────────────────────────

    #[test]
    fn test_add_player_empty_name_fails() {
        let lobby = create_test_lobby();
        assert!(lobby.add_player(PlayerId::from("p1"), "".to_string()).is_err());
    }

    #[test]
    fn test_add_player_whitespace_name_fails() {
        let lobby = create_test_lobby();
        assert!(lobby.add_player(PlayerId::from("p1"), "   ".to_string()).is_err());
    }

    #[test]
    fn test_remove_player() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("p1"), "Alice".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();

        assert!(lobby.remove_player(&PlayerId::from("p2")).unwrap());
        let players = lobby.get_all_players().unwrap();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "Alice");
    }

    #[test]
    fn test_remove_nonexistent_player_returns_false() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("p1"), "Alice".to_string()).unwrap();
        assert!(!lobby.remove_player(&PlayerId::from("ghost")).unwrap());
    }

    #[test]
    fn test_remove_leader_transfers_leadership() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("leader"), "Leader".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();

        lobby.remove_player(&PlayerId::from("leader")).unwrap();
        assert!(lobby.is_leader(&PlayerId::from("p2")).unwrap());
    }

    // ── Game flow ───────────────────────────────────────────────────────────

    #[test]
    fn test_start_game_non_leader() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("leader"), "Leader".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();
        assert!(lobby.start_game(&PlayerId::from("p2")).is_err());
    }

    #[test]
    fn test_start_game_twice_fails() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("leader"), "Leader".to_string()).unwrap();
        assert!(lobby.start_game(&PlayerId::from("leader")).is_ok());
        assert!(lobby.start_game(&PlayerId::from("leader")).is_err());
    }

    #[test]
    fn test_reset_lobby_returns_to_lobby_status() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("leader"), "Leader".to_string()).unwrap();
        lobby.start_game(&PlayerId::from("leader")).unwrap();

        lobby.reset_lobby(&PlayerId::from("leader")).unwrap();
        assert_eq!(lobby.game_status.read(|s| *s).unwrap(), GameStatus::Lobby);
    }

    #[test]
    fn test_advance_turn_wraps() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("p1"), "Alice".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();
        lobby.turn_order.write(|o| { o.push(PlayerId::from("p1")); o.push(PlayerId::from("p2")); }).unwrap();

        assert_eq!(lobby.advance_turn().unwrap(), PlayerId::from("p2"));
        assert_eq!(lobby.advance_turn().unwrap(), PlayerId::from("p1")); // wraps
    }

    #[test]
    fn test_get_lobby_info_reflects_state() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("p1"), "Alice".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();

        let info = lobby.get_lobby_info("test-lobby").unwrap();
        assert_eq!(info.lobby_id, "test-lobby");
        assert_eq!(info.status, GameStatus::Lobby);
        assert_eq!(info.leader_id, PlayerId::from("p1"));
        assert_eq!(info.players.len(), 2);
    }

    // ── Guess processing ────────────────────────────────────────────────────

    /// Shared setup: start a single-player Deathmatch with target_score=3,
    /// current kanji set to "日", and game status = Playing.
    fn setup_deathmatch_playing() -> (LobbyState, PlayerId) {
        let lobby = create_test_lobby();
        let leader = PlayerId::from("leader");
        lobby.add_player(leader.clone(), "Leader".to_string()).unwrap();
        lobby.settings.write(|s| { s.target_score = Some(3); }).unwrap();
        lobby.active_level_indices.write(|i| i.push(0)).unwrap();
        lobby.current_prompt.write(|k| *k = Some(ActivePrompt::Kanji { character: "日".to_string() })).unwrap();
        lobby.game_status.write(|s| *s = GameStatus::Playing).unwrap();
        (lobby, leader)
    }

    #[test]
    fn test_process_guess_correct_increments_score() {
        let (lobby, leader) = setup_deathmatch_playing();
        lobby.process_guess(&leader, "日本").unwrap();
        assert_eq!(lobby.get_player_score(&leader).unwrap(), 1);
    }

    #[test]
    fn test_process_guess_wrong_word_no_score() {
        let (lobby, leader) = setup_deathmatch_playing();
        // "日xyz" contains "日" but is NOT in the dictionary
        lobby.process_guess(&leader, "日xyz").unwrap();
        assert_eq!(lobby.get_player_score(&leader).unwrap(), 0);
    }

    #[test]
    fn test_process_guess_wrong_kanji_no_score() {
        let (lobby, leader) = setup_deathmatch_playing();
        // "哀歌" is a valid dictionary word but does NOT contain the prompt kanji "日"
        lobby.process_guess(&leader, "哀歌").unwrap();
        assert_eq!(lobby.get_player_score(&leader).unwrap(), 0);
    }

    #[test]
    fn test_process_guess_while_not_playing_is_noop() {
        let lobby = create_test_lobby();
        let leader = PlayerId::from("leader");
        lobby.add_player(leader.clone(), "Leader".to_string()).unwrap();
        // Default status is Lobby — should be silently ignored
        assert!(lobby.process_guess(&leader, "日本").is_ok());
        assert_eq!(lobby.get_player_score(&leader).unwrap(), 0);
    }

    #[test]
    fn test_deathmatch_target_score_ends_game() {
        let (lobby, leader) = setup_deathmatch_playing();
        for _ in 0..3 {
            lobby.current_prompt.write(|k| *k = Some(ActivePrompt::Kanji { character: "日".to_string() })).unwrap();
            let empty = lobby.active_level_indices.read(|i| i.is_empty()).unwrap();
            if empty { lobby.active_level_indices.write(|i| i.push(0)).unwrap(); }
            lobby.process_guess(&leader, "日本").unwrap();
        }
        assert_eq!(lobby.game_status.read(|s| *s).unwrap(), GameStatus::Finished);
    }

    #[test]
    fn test_duel_wrong_turn_is_ignored() {
        let lobby = create_test_lobby();
        let p1 = PlayerId::from("p1");
        let p2 = PlayerId::from("p2");
        lobby.add_player(p1.clone(), "Alice".to_string()).unwrap();
        lobby.add_player(p2.clone(), "Bob".to_string()).unwrap();
        lobby.settings.write(|s| { s.mode = shared::GameMode::Duel; s.initial_lives = Some(3); }).unwrap();
        lobby.game_status.write(|s| *s = GameStatus::Playing).unwrap();
        lobby.turn_order.write(|o| { o.push(p1.clone()); o.push(p2.clone()); }).unwrap();
        lobby.current_prompt.write(|k| *k = Some(ActivePrompt::Kanji { character: "日".to_string() })).unwrap();
        lobby.active_level_indices.write(|i| i.push(0)).unwrap();

        // p2 submits on p1's turn — should be silently ignored
        lobby.process_guess(&p2, "日本").unwrap();
        assert_eq!(lobby.get_player_score(&p2).unwrap(), 0);
    }
}
