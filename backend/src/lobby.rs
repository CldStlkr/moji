use chrono::Utc;
use rand::{RngExt, distr::{Distribution, weighted::WeightedIndex}};
use tokio::sync::broadcast;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub use shared::{
    CheckWordResponse, GameSettings, GameStatus, JoinLobbyRequest, PlayerId, ApiContext,
    ContentMode, ActivePrompt, LobbyId, LobbyInfo
};
pub use crate::{
    utils::check_prompt,
    types::{Result, Shared, PlayerData},
    data::{JlptWordData, KanjiData, DictData},
    error::AppError,
};




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
    pub prompt_counter: Shared<u64>,
    pub skip_votes: Shared<HashSet<PlayerId>>,
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
            prompt_counter: Shared::new(0),
            skip_votes: Shared::new(HashSet::new()),
        }
    }

    pub fn broadcast(&self, msg: shared::ServerMessage) {
        let msg_json = serde_json::to_string(&msg).unwrap_or_default();
        if self.tx.receiver_count() > 0 {
            let _ = self.tx.send(msg_json);
        }
    }

    pub fn is_leader(&self, player_id: &PlayerId) -> bool {
        self.lobby_leader.read(|leader| {
            leader.to_string() == player_id.to_string()
        })
    }

    pub fn update_settings(&self, player_id: &PlayerId, new_settings: GameSettings) -> Result<()> {
        if !self.is_leader(player_id) {
            return Err(AppError::AuthError(
                "Only lobby leader can change settings".to_string(),
            ));
        }

        self.settings.write(|settings| {
            *settings = new_settings.clone();
        });

        self.broadcast(shared::ServerMessage::SettingsUpdate {
            settings: new_settings
        });

        Ok(())
    }

    pub fn get_lobby_info(&self, lobby_id: &LobbyId) -> LobbyInfo {
        let status = self.game_status.read(|s| *s);
        let settings = self.settings.read(|s| s.clone());
        let leader = self.lobby_leader.read(|l| l.clone());

        let current_turn = self.turn_order.read(|order| {
             self.current_turn_index.read(|idx| order.get(*idx).cloned())
        });

        let api_players = self.players.read(|players| {
             players.iter()
            .map(|p| shared::PlayerData {
                id: p.id.clone(),
                name: p.name.clone(),
                score: p.score,
                joined_at: p.joined_at.to_rfc3339(),
                lives: p.lives,
                is_eliminated: p.is_eliminated,
                is_connected: p.is_connected,
                is_turn: current_turn.as_ref() == Some(&p.id) && status == GameStatus::Playing && settings.mode == shared::GameMode::Duel,
            })
            .collect::<Vec<_>>()
        });

        shared::LobbyInfo {
            lobby_id: lobby_id.clone(),
            leader_id: leader,
            players: api_players,
            settings,
            status,
        }
    }

    pub fn start_game(&self, player_id: &PlayerId) -> Result<()> {
        if !self.is_leader(player_id) {
            return Err(AppError::AuthError(
                "Only lobby leader can start the game".to_string(),
            ))?;
        }

        let status = self.game_status.read(|status| *status);
        if status != GameStatus::Lobby {
            return Err(AppError::InvalidInput("game is not in lobby state".to_string()))?;
        }

        let settings = self.settings.read(|s| s.clone());

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

            self.active_level_indices.write(|ai| *ai = indices);
            self.level_weights.write(|lw| *lw = w_map);

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
                     })
                })
            });
        }

        self.generate_random_prompt(false)?;

        self.game_status.write(|status| *status = GameStatus::Playing);

        self.broadcast(shared::ServerMessage::GameState {
            prompt: self.get_current_prompt_text().unwrap_or_default(),
            status: GameStatus::Playing,
            scores: self.get_all_players(),
        });

        Ok(())
    }

    pub fn add_player(&self, player_id: PlayerId, player_name: String) -> Result<bool> {
        let is_leader_result = self.players.write(|players| {
            let is_leader = players.is_empty();
            if is_leader {
                 self.lobby_leader.write(|leader| *leader = player_id.clone());
            }

            let trimmed_name = player_name.trim();
            if trimmed_name.is_empty() {
                return Err(AppError::InvalidInput("Player name cannot be empty".to_string()));
            }

            let normalized_name = trimmed_name.split_whitespace().collect::<Vec<&str>>().join(" ");


            // Remove any lingering duplicates (same ID or same name)
            players.retain(|p| p.id != player_id && p.name != normalized_name);

            players.push(PlayerData {
                id: player_id.clone(),
                name: normalized_name,
                score: 0,
                joined_at: Utc::now(),
                lives: None,
                is_eliminated: false,
                is_connected: true,
            });
            Ok(is_leader)
        })?;

        self.broadcast(shared::ServerMessage::PlayerListUpdate {
            players: self.get_all_players(),
        });

        Ok(is_leader_result)
    }

    pub fn remove_player(&self, player_id: &PlayerId) -> bool {
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
                         })
                    } 
                });

                self.lobby_leader.write(|leader| {
                    if leader.to_string() == player_id.to_string() {
                        if let Some(new_leader) = players.first() {
                            tracing::info!("Reassigned lobby leader to {}", new_leader.id.0);
                            *leader = new_leader.id.clone();
                            self.broadcast(shared::ServerMessage::LeaderUpdate {
                                leader_id: new_leader.id.clone()
                            });
                        } else {
                            tracing::info!("No players left to be leader");
                            *leader = PlayerId::default(); 
                        }
                    }
                });

                let pl_update = shared::ServerMessage::PlayerListUpdate {
                    players: players.iter().map(|p| shared::PlayerData {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        score: p.score,
                        joined_at: p.joined_at.to_rfc3339(),
                        lives: p.lives,
                        is_eliminated: p.is_eliminated,
                        is_connected: p.is_connected,
                        is_turn: false,
                    }).collect()
                };
                
                self.broadcast(pl_update);

                true
            } else {
                tracing::info!("remove_player: player {} already removed or not found (likely already cleaned up)", player_id.0);
                false
            }
        })
    }

    pub fn set_player_connected(&self, player_id: &PlayerId, is_connected: bool) -> bool {
        let changed = self.players.write(|players| {
            if let Some(p) = players.iter_mut().find(|p| &p.id == player_id) {
                p.is_connected = is_connected;
                true
            } else {
                false
            }
        });

        if changed {
            let pl_update = shared::ServerMessage::PlayerListUpdate {
                players: self.get_all_players()
            };
            let _ = self.tx.send(serde_json::to_string(&pl_update).unwrap_or_default());
        }
        
        changed
    }

    pub fn get_player_score(&self, player_id: &PlayerId) -> Result<u32> {
        self.players.read(|players| {
            players
                .iter()
                .find(|p| &p.id == player_id)
                .map(|p| p.score)
                .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))
        })
    }

    pub fn get_player_name(&self, player_id: &PlayerId) -> Result<String> {
        self.players.read(|players| {
            players
                .iter()
                .find(|p| &p.id == player_id)
                .map(|p| p.name.clone())
                .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))
        })
    }

    pub fn increment_player_score(&self, player_id: &PlayerId) -> Result<u32> {
        self.players.write(|players| {
             let player = players
                .iter_mut()
                .find(|p| &p.id == player_id)
                .ok_or_else(|| AppError::PlayerNotFound(player_id.0.clone()))?;

            player.score += 1;
            Ok(player.score)
        })
    }

    // Get all players and scores (for a leaderboard)
    pub fn get_all_players(&self) -> Vec<shared::PlayerData> {
        let status = self.game_status.read(|s| *s);
        let settings = self.settings.read(|s| s.clone());

        // Lock players and potentially turn_order/current_turn_index
        self.players.read(|players| {
             let mut shared_players: Vec<shared::PlayerData> = players.iter().map(|p| shared::PlayerData {
                id: p.id.clone(),
                name: p.name.clone(),
                score: p.score,
                joined_at: p.joined_at.to_rfc3339(),
                lives: p.lives,
                is_eliminated: p.is_eliminated,
                is_connected: p.is_connected,
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
                    })
                });
            }
            shared_players
        })
    }

    pub fn get_current_prompt_text(&self) -> Option<String> {
        self.current_prompt.read(|p| p.as_ref().map(|prompt| prompt.display_text().to_string()))
    }

    /// Generate a new random kanji and store it as current.
    /// If `broadcast` is true, a `PromptUpdate` WS message is sent to all clients.
    /// Pass `false` when the caller will send a more complete message (e.g. `GameState`).
    pub fn generate_random_prompt(&self, broadcast: bool) -> Result<String> {
        let content_mode = self.settings.read(|s| s.content_mode.clone());
        let mut rng = rand::rng();
        let indices = self.active_level_indices.read(|i| i.clone());

        if indices.is_empty() {
            return Err(AppError::InternalError("No active levels configured".into()));
        }

        let level_idx = indices[rng.random_range(0..indices.len())];

        let display_text = match content_mode {
            ContentMode::Kanji => {
                let weights_map = self.level_weights.read(|w| w.clone());
                let kanji_list = &self.kanji_list[level_idx];

                let kanji = if let Some(dist) = weights_map.get(&level_idx) {
                    kanji_list[dist.sample(&mut rng)].clone()
                } else {
                    kanji_list[rng.random_range(0..kanji_list.len())].clone()
                };

                let prompt = ActivePrompt::Kanji { character: kanji.kanji.clone() };
                let text = kanji.kanji;
                self.current_prompt.write(|p| *p = Some(prompt));

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
                self.current_prompt.write(|p| *p = Some(prompt));

                text
            }
        };

        if broadcast {
            self.broadcast(shared::ServerMessage::PromptUpdate {
                new_prompt: display_text.clone(),
            });
        }

        self.prompt_counter.write(|c| *c += 1);
        self.skip_votes.write(|v| v.clear());

        let time_limit = self.settings.read(|s| s.time_limit_seconds);
        if let Some(secs) = time_limit {
            let counter = self.prompt_counter.read(|c| *c);
            let lobby = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(secs as u64)).await;
                lobby.process_timeout(counter);
            });
        }

        Ok(display_text)
    }

    pub fn reset_lobby(&self, player_id: &PlayerId) -> Result<()> {
        if !self.is_leader(player_id) {
            return Err(AppError::AuthError(
                "Only lobby leader can reset the lobby".to_string(),
            ));
        }

        self.game_status.write(|status| *status = GameStatus::Lobby);
 
        self.broadcast(shared::ServerMessage::GameState {
            prompt: "".to_string(),
            status: GameStatus::Lobby,
            scores: self.get_all_players(),
        });

        Ok(())
    }

    pub fn advance_turn(&self) -> Result<PlayerId> {
        self.turn_order.write(|order| {
             self.current_turn_index.write(|idx| {
                 if order.is_empty() { return Err(AppError::InternalError("No players in turn order".to_string())); }
                 *idx = (*idx + 1) % order.len();
                 Ok(order[*idx].clone())
             })
        })
    }

    pub fn get_current_turn_player(&self) -> Option<PlayerId> {
        self.turn_order.read(|order| {
             self.current_turn_index.read(|idx| {
                 order.get(*idx).cloned()
             })
        })
    }

    pub fn process_guess(&self, player_id: &PlayerId, input: &str) -> Result<()> {
        let (settings, status) = {
             let st = self.game_status.read(|s| *s);
             let s = self.settings.read(|s| s.clone());
             (s, st)
        };

        if status != GameStatus::Playing {
            return Ok(());
        }

        if settings.mode == shared::GameMode::Duel {
            let current_turn = self.get_current_turn_player();
            if current_turn.as_ref() != Some(player_id) {
                return Ok(()); // Not your turn
            }
        }


        let trimmed_input = input.trim();
        let prompt = self.current_prompt.read(|p| p.clone())
            .ok_or(AppError::InternalError("No active prompt".into()))?;


        let is_correct = check_prompt(&prompt, trimmed_input, &self.dict_list);

        let mut message = String::new();
        let mut new_prompt_opt = None;
        let mut game_over = false;
        let mut error_details = None;

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
                        new_prompt_opt = self.get_current_prompt_text();
                    }
                }
            } else if settings.mode == shared::GameMode::Duel {
                message = "Good guess!".to_string();
                let _ = self.generate_random_prompt(true);
                new_prompt_opt = self.get_current_prompt_text();
                let _ = self.advance_turn();
            } else if settings.mode == shared::GameMode::Zen {
                message = "Good guess!".to_string();
                let _ = self.generate_random_prompt(true);
                new_prompt_opt = self.get_current_prompt_text();
            }
        } else {
            error_details = self.get_error_details();
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
                let (eliminated, duel_message) = self.apply_duel_penalty(player_id, &mut new_prompt_opt, &mut game_over);
                if eliminated {
                    message = format!("{}\n{}", message, duel_message);
                }
            }
        }

        self.broadcast(shared::ServerMessage::PlayerListUpdate {
            players: self.get_all_players()
        });

        let score = self.get_player_score(player_id).unwrap_or(0);
        self.broadcast(shared::ServerMessage::WordChecked {
            player_id: player_id.clone(),
            result: shared::CheckWordResponse {
                message,
                score,
                error: if !is_correct { Some("Incorrect".into()) } else { None },
                error_details,
                prompt: new_prompt_opt,
            },
        });

        if game_over {
            self.game_status.write(|st| *st = GameStatus::Finished);
            self.broadcast(shared::ServerMessage::GameState {
                prompt: self.get_current_prompt_text().unwrap_or_default(),
                status: GameStatus::Finished,
                scores: self.get_all_players(),
            });
        }

        Ok(())
    }

    fn get_error_details(&self) -> Option<Vec<String>> {
        let prompt = self.current_prompt.read(|p| p.clone())?;
        match prompt {
            ActivePrompt::Vocab { readings, .. } => Some(readings),
            ActivePrompt::Kanji { character } => {
                let mut matches = Vec::new();
                for w in self.dict_list.iter() {
                    if w.contains(&character) {
                        matches.push(w.clone());
                        if matches.len() >= 3 { break; }
                    }
                }
                Some(matches)
            }
        }
    }

    fn apply_duel_penalty(&self, player_id: &PlayerId, new_prompt_opt: &mut Option<String>, game_over: &mut bool) -> (bool, String) {
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
        });
        
        let mut msg = String::new();
        if eliminated {
             msg = "Eliminated!".to_string();
        }

        let settings = self.settings.read(|s| s.clone());
        if !settings.duel_allow_kanji_reuse {
            let _ = self.generate_random_prompt(true);
            *new_prompt_opt = self.get_current_prompt_text();
        }

        if eliminated {
             self.turn_order.write(|order| {
                 if let Some(pos) = order.iter().position(|id| id == player_id) {
                     order.remove(pos);
                     self.current_turn_index.write(|idx| {
                         if *idx >= order.len() && !order.is_empty() {
                             *idx = 0;
                         }
                     })
                 }
            });
        } else {
             let _ = self.advance_turn();
        }
        let order_len = self.turn_order.read(|o| o.len());
        if order_len <= 1 {
            *game_over = true;
            if !eliminated {
                 msg = "Winner!".to_string();
            }
        }
        (eliminated, msg)
    }

    pub fn process_timeout(&self, expected_counter: u64) {
        let current_counter = self.prompt_counter.read(|c| *c);
        if current_counter != expected_counter {
            return; // Prompt has already advanced
        }
        
        // Ensure game is still active
        let status = self.game_status.read(|s| *s);
        if status != GameStatus::Playing {
            return;
        }

        let settings = self.settings.read(|s| s.clone());
        let error_details = self.get_error_details();
        
        if settings.mode == shared::GameMode::Duel {
            if let Some(player_id) = self.get_current_turn_player() {
                let mut new_prompt_opt = None;
                let mut game_over = false;
                
                let (eliminated, duel_msg) = self.apply_duel_penalty(&player_id, &mut new_prompt_opt, &mut game_over);
                let message = if eliminated { format!("Time's up!\n{}", duel_msg) } else { "Time's up!".to_string() };
 
                self.broadcast(shared::ServerMessage::PlayerListUpdate {
                    players: self.get_all_players()
                });
            
                let score = self.get_player_score(&player_id).unwrap_or(0);
                self.broadcast(shared::ServerMessage::WordChecked {
                    player_id,
                    result: shared::CheckWordResponse {
                        message,
                        score,
                        error: Some("Time's up!".into()),
                        error_details,
                        prompt: new_prompt_opt.clone(),
                    },
                });
            
                if game_over {
                    self.game_status.write(|st| *st = GameStatus::Finished);
                    self.broadcast(shared::ServerMessage::GameState {
                        prompt: self.get_current_prompt_text().unwrap_or_default(),
                        status: GameStatus::Finished,
                        scores: self.get_all_players(),
                    });
                }
            }
        } else {
            // Deathmatch: Skip the prompt
            let _ = self.generate_random_prompt(true);
            self.broadcast(shared::ServerMessage::WordChecked {
                player_id: PlayerId::default(), // Nobody in particular
                result: shared::CheckWordResponse {
                    message: "Time's up! Skipped prompt.".to_string(),
                    score: 0,
                    error: Some("Time's up!".into()),
                    error_details,
                    prompt: self.get_current_prompt_text(),
                },
            });
        }
    }

    pub fn process_skip(&self, player_id: &PlayerId) -> Result<()> {
        let status = self.game_status.read(|s| *s);
        if status != GameStatus::Playing {
            return Ok(());
        }

        let settings = self.settings.read(|s| s.clone());
        let error_details = self.get_error_details();

        if settings.mode == shared::GameMode::Duel {
            let current_turn = self.get_current_turn_player();
            if current_turn.as_ref() != Some(player_id) {
                return Ok(()); // Handled only if it's your turn
            }

            let mut new_prompt_opt = None;
            let mut game_over = false;
            
            let (eliminated, duel_msg) = self.apply_duel_penalty(player_id, &mut new_prompt_opt, &mut game_over);
            let message = if eliminated { format!("Skipped!\n{}", duel_msg) } else { "Skipped!".to_string() };
 
            self.broadcast(shared::ServerMessage::PlayerListUpdate {
                players: self.get_all_players()
            });
        
            let score = self.get_player_score(player_id).unwrap_or(0);
            self.broadcast(shared::ServerMessage::WordChecked {
                player_id: player_id.clone(),
                result: shared::CheckWordResponse {
                    message,
                    score,
                    error: Some("Skipped!".into()),
                    error_details,
                    prompt: new_prompt_opt,
                },
            });
        
            if game_over {
                self.game_status.write(|st| *st = GameStatus::Finished);
                self.broadcast(shared::ServerMessage::GameState {
                    prompt: self.get_current_prompt_text().unwrap_or_default(),
                    status: GameStatus::Finished,
                    scores: self.get_all_players(),
                });
            }

        } else {
            // Deathmatch skipping requires majority vote
            let mut skip_passed = false;
            let (votes, required) = self.skip_votes.write(|votes| {
                votes.insert(player_id.clone());
                let total_players = self.get_all_players().iter().filter(|p| !p.is_eliminated).count();
                // Majority > 50%
                let required = (total_players / 2) + 1;
                if votes.len() >= required {
                    skip_passed = true;
                }
                (votes.len(), required)
            });

            if skip_passed {
                let _ = self.generate_random_prompt(true);
                self.broadcast(shared::ServerMessage::WordChecked {
                    player_id: PlayerId::default(),
                    result: shared::CheckWordResponse {
                        message: "Prompt skipped by vote!".to_string(),
                        score: 0,
                        error: Some("Skipped!".into()),
                        error_details,
                        prompt: self.get_current_prompt_text(),
                    },
                });
            } else {
                self.broadcast(shared::ServerMessage::SkipVoteUpdate {
                    votes,
                    required,
                });
            }
        }

        Ok(())
    }
}
