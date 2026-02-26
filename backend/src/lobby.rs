use chrono::Utc;
use crate::data::{JlptWordData, KanjiData, DictData};
use crate::error::AppError;
use crate::{PlayerData, check_prompt};
use rand::{RngExt, distr::{Distribution, weighted::WeightedIndex}};
use shared::{ContentMode, ActivePrompt, LobbyId};
use tokio::sync::broadcast;
use std::{
    collections::HashMap,
    sync::Arc,
};

pub use shared::{
    CheckWordResponse, GameSettings, GameStatus, JoinLobbyRequest, PlayerId, ApiContext,
};
pub use crate::types::{Result, Shared, SharedState};




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

        let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::SettingsUpdate {
            settings: new_settings
        }).unwrap_or_default());

        Ok(())
    }

    pub fn get_lobby_info(&self, lobby_id: &LobbyId) -> Result<shared::LobbyInfo> {
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
                is_turn: current_turn.as_ref() == Some(&p.id) && status == GameStatus::Playing && settings.mode == shared::GameMode::Duel,
            })
            .collect::<Vec<_>>()
        });

        Ok(shared::LobbyInfo {
            lobby_id: lobby_id.clone(),
            leader_id: leader,
            players: api_players,
            settings,
            status,
        })
    }

    pub fn start_game(&self, player_id: &PlayerId) -> Result<()> {
        if !self.is_leader(player_id) {
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
        })?;

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

        let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::GameState {
            prompt: self.get_current_prompt_text().unwrap_or_default(),
            status: GameStatus::Playing,
            scores: self.get_all_players()?,
        }).unwrap_or_default());

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
        })?;

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
                         })
                    } 
                });

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
                });

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
        })
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
    pub fn get_all_players(&self) -> Result<Vec<shared::PlayerData>> {
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
            Ok(shared_players)
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
            let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::PromptUpdate {
                new_prompt: display_text.clone(),
            }).unwrap_or_default());
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
                });
                if eliminated {
                     message = "Eliminated!".to_string();
                }

                if !settings.duel_allow_kanji_reuse {
                    let _ = self.generate_random_prompt(true);
                    new_prompt_opt = self.get_current_prompt_text();
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
            self.game_status.write(|st| *st = GameStatus::Finished);
            let _ = self.tx.send(serde_json::to_string(&shared::ServerMessage::GameState {
                prompt: self.get_current_prompt_text().unwrap_or_default(),
                status: GameStatus::Finished,
                scores: self.get_all_players()?,
            }).unwrap_or_default());
        }

        Ok(())
    }
}
