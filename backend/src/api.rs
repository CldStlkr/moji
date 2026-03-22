use crate::{
    utils::{generate_lobby_id, generate_player_id},
    models::{
        user::User,
        game::{GameAction, GameSession},
    },
    state::AppState,
    lobby::LobbyState,
};
use axum::{
    extract::{Path, State, WebSocketUpgrade, Query, ws::{Message, WebSocket}},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use argon2::{
    Argon2, password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng
    }
};
use shared::{
    JoinLobbyRequest, PromptResponse, LobbyId,
    PlayerId, StartGameRequest, UpdateSettingsRequest, ApiContext,
    JsonResult, PromptResult, LobbyResult, PlayerResult
};
use async_trait::async_trait;
use leptos::server_fn::error::ServerFnError;
use serde::{Deserialize, Serialize};
use rustrict::{CensorStr, Type};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH}
};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

fn generate_jwt(user_id: &str) -> Result<String, ServerFnError> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize + 60 * 60 * 24; // 24 hours

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "INSECURE_DEFAULT_SECRET".to_string());

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_ref())
    ).map_err(|e| ServerFnError::new(e.to_string()))
}

fn validate_username(username: &str) -> std::result::Result<(), ServerFnError> {
    if username.len() < 3 || username.len() > 20 {
        return Err(ServerFnError::new("Username must be between 3 and 20 characters"));
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ServerFnError::new("Username can only contain letters, numbers, and underscores"));
    }
    if username.is(Type::INAPPROPRIATE) || username.is(Type::EVASIVE) {
        return Err(ServerFnError::new("Username is not appropriate"));
    }

    Ok(())
}

#[async_trait]
impl ApiContext for AppState {
    async fn create_lobby(&self, request: JoinLobbyRequest) -> JsonResult {
        let lobby_id: LobbyId = generate_lobby_id();
        let player_id: PlayerId = generate_player_id();

        let game_session_id = if let Some(db_pool) = &self.db_pool {
            let default_settings = shared::GameSettings::default();
            let session = GameSession::create(db_pool, &lobby_id, 1, default_settings).await?;
            Some(session.id)
        } else {
            None
        };

        let lobby_state = Arc::new(LobbyState::new(
            Arc::clone(&self.kanji_data),
            Arc::clone(&self.word_data),
            Arc::clone(&self.dict_data),
            game_session_id
        ));

        let _ = lobby_state.add_player(player_id.clone(), request.player_name)?;

        self.lobbies.write(|lobbies| { lobbies.insert(lobby_id.clone(), lobby_state); });

        Ok(json!({
            "message": "Lobby created successfully!",
            "lobby_id": lobby_id,
            "player_id": player_id.to_string()
        }))
    }

    async fn get_lobby_info(&self, lobby_id: LobbyId) -> LobbyResult {
        let lobby = self.get_lobby(&lobby_id)?;
        Ok(lobby.get_lobby_info(&lobby_id))
    }

    async fn update_lobby_settings(&self, lobby_id: LobbyId, request: UpdateSettingsRequest) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id)?;
        lobby.update_settings(&request.player_id, request.settings)?;
        Ok(json!({ "message": "Settings updated successfully" }))
    }

    async fn start_game(&self, lobby_id: LobbyId, request: StartGameRequest) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id)?;
        lobby.start_game(&request.player_id)?;
        Ok(json!({ "message": "Game started successfully" }))
    }

    async fn reset_lobby(&self, lobby_id: LobbyId, player_id: PlayerId) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id)?;

        lobby.reset_lobby(&player_id)?;
        Ok(json!({ "message": "Lobby reset successfully" }))
    }

    async fn get_lobby_players(&self, lobby_id: LobbyId) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id)?;

        let players = lobby.get_all_players();

        let player_data: Vec<_> = players.into_iter().map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "score": p.score,
                "joined_at": p.joined_at
            })
        }).collect();

        Ok(json!({ "players": player_data }))
    }

    async fn join_lobby(&self, lobby_id: LobbyId, request: JoinLobbyRequest) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id)?;

        let player_id = request.player_id.unwrap_or_else(generate_player_id);
        let _ = lobby.add_player(player_id.clone(), request.player_name.clone())?;

        if let Some(game_id) = lobby.game_session_id {
            if let Some(db_pool) = &self.db_pool {
                let db = Arc::clone(db_pool);
                let name = request.player_name.clone();
                let pid = player_id.to_string();

                tokio::spawn(async move {
                    let action_data = json!({
                        "player_id": pid,
                        "player_name": name
                    });

                    if let Err(e) = GameAction::create(&db, game_id, None, "player_joined", action_data).await {
                        tracing::error!("Failed to log player join: {:?}", e);
                    }
                });
            }
        }

        Ok(json!({
            "message": "Joined lobby successfully!",
            "lobby_id": lobby_id,
            "player_id": player_id
        }))
    }

    async fn get_prompt(&self, lobby_id: LobbyId) -> PromptResult {
        let lobby = self.get_lobby(&lobby_id)?;

        let prompt = match lobby.get_current_prompt_text() {
            Some(prompt) => prompt,
            None => lobby.generate_random_prompt(true)?
        };
        Ok(PromptResponse { prompt })
    }

    async fn generate_new_prompt(&self, lobby_id: LobbyId) -> PromptResult {
        let lobby = self.get_lobby(&lobby_id)?;

        let prompt = lobby.generate_random_prompt(true)?;
        Ok(PromptResponse { prompt })
    }

    async fn check_username(&self, username: String) -> JsonResult {
        validate_username(&username)?;

        let db_pool = self.db_pool.as_ref()
            .ok_or_else(|| ServerFnError::new("Database not configured"))?;

        let user = User::find_by_username(db_pool, &username).await?;

        if let Some(user) = user {
            Ok(json!({
                "available": false,
                "is_guest": user.is_guest
            }))
        } else {
            Ok(json!({
                "available": true,
                "is_guest": false
            }))
        }
    }

    async fn authenticate(&self, request: shared::AuthRequest) -> JsonResult {
        validate_username(&request.username)?;

        let db_pool = self.db_pool.as_ref()
            .ok_or_else(|| ServerFnError::new("Database not configured"))?;
        let existing_user = User::find_by_username(db_pool, &request.username).await
            ?;

        if let Some(user) = existing_user {
            if let Some(password) = request.password {
                if let Some(hash) = &user.password_hash {
                    let parsed_hash = PasswordHash::new(hash)
                        .map_err(|e| ServerFnError::new(e.to_string()))?;
                    if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok() {
                        Ok(json!({
                            "message": "Login successful",
                            "user": &user,
                            "token": generate_jwt(&user.id.to_string())?
                        }))
                    } else {
                        Err(ServerFnError::new("Invalid password"))
                    }
                } else {
                    // It's a guest account
                    Err(ServerFnError::new("Name currently in use"))
                }
            } else {
                // Name is taken, but no password was provided to check ownership
                Err(ServerFnError::new("Name currently in use"))
            }
        } else if request.create_guest {
            let user = User::create(db_pool, &request.username, None, true).await?;
            Ok(json!({
                "message": "Guest account created",
                "user": &user,
                "token": generate_jwt(&user.id.to_string())?
            }))
        } else if let Some(password) = request.password {
            let salt = SaltString::generate(&mut OsRng);
            let password_hash = Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .to_string();
            let user = User::create(db_pool, &request.username, Some(password_hash), false).await?;
            Ok(json!({
                "message": "Account created",
                "user": &user,
                "token": generate_jwt(&user.id.to_string())?
            }))
        } else {
            Err(ServerFnError::new("Password required to register"))
        }
    }

    async fn get_player_info(&self, lobby_id: LobbyId, player_id: PlayerId) -> PlayerResult {
        let lobby = self.get_lobby(&lobby_id)?;

        let players = lobby.get_all_players();
        let player = players.into_iter().find(|p| p.id == player_id)
            .ok_or_else(|| ServerFnError::new(format!("Player not found: {}", player_id)))?;

        Ok(player)
    }

    async fn leave_lobby(&self, lobby_id: LobbyId, player_id: PlayerId) -> JsonResult {
        let lobby = match self.get_lobby(&lobby_id) {
            Ok(l) => l,
            Err(crate::error::AppError::LobbyNotFound(_)) => {
                // If the lobby is already gone (e.g. WS cleanup finished first), this is fine.
                return Ok(json!({ "message": "Lobby already cleaned up" }));
            }
            Err(e) => return Err(e.into()),
        };

        lobby.remove_player(&player_id);

        let is_empty = lobby.players.read(|players| players.is_empty());
        let actually_removed = if is_empty {
            self.lobbies.write(|lobbies| {
                lobbies.remove(&lobby_id).is_some()
            })
        } else {
            false
        };

        if actually_removed {
            if let Some(game_id) = lobby.game_session_id {
                if let Some(db_pool) = &self.db_pool {
                    let pool = Arc::clone(db_pool);
                    tokio::spawn(async move {
                        let _ = GameSession::end_session(&pool, game_id).await;
                    });
                }
            }
        }

        Ok(json!({ "message": "Left lobby" }))
    }

    async fn logout(&self, username: String) -> JsonResult {
        let db_pool = self.db_pool.as_ref()
            .ok_or_else(|| ServerFnError::new("Database not configured"))?;

        User::delete_guest_by_username(db_pool, &username).await?;

        Ok(json!({ "message": "Logged out" }))
    }

    async fn set_player_connected(&self, lobby_id: LobbyId, player_id: PlayerId, is_connected: bool) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id)?;
        lobby.set_player_connected(&player_id, is_connected);
        Ok(json!({ "message": "Connection status updated" }))
    }

    async fn kick_player(&self, lobby_id: LobbyId, requestor_id: PlayerId, target_player_id: PlayerId) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id)?;
        lobby.kick_player(&requestor_id, &target_player_id)?;
        Ok(json!({ "message": "Player kicked" }))
    }

    async fn promote_leader(&self, lobby_id: LobbyId, requestor_id: PlayerId, target_player_id: PlayerId) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id)?;
        lobby.promote_leader(&requestor_id, &target_player_id)?;
        Ok(json!({ "message": "Leader promoted" }))
    }
}

#[derive(Deserialize)]
pub struct WsParams {
    token: Option<String>,
}

#[axum::debug_handler]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path((lobby_id, player_id)): Path<(LobbyId, PlayerId)>,
    Query(params): Query<WsParams>,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "INSECURE_DEFAULT_SECRET".to_string());
    
    let result = if let Some(t) = params.token {
        jsonwebtoken::decode::<Claims>(
            &t,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default()
        )
    } else {
        Err(jsonwebtoken::errors::ErrorKind::InvalidToken.into())
    };

    if let Err(e) = result {
        let reason = match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => "Token expired",
            jsonwebtoken::errors::ErrorKind::InvalidToken => "Invalid token or missing",
            jsonwebtoken::errors::ErrorKind::InvalidSignature => "Invalid signature",
            _ => "Unauthorized",
        };
        tracing::warn!("Unauthorized WebSocket connection attempt to lobby {}: {}", lobby_id.0, reason);
        return (axum::http::StatusCode::UNAUTHORIZED, reason).into_response();
    }

    ws.on_upgrade(move |socket| async move {
        // Set player as connected when WS starts
        let _ = app_state.set_player_connected(lobby_id.clone(), player_id.clone(), true).await;
        handle_socket(socket, app_state, lobby_id, player_id).await
    })
}

async fn handle_socket(socket: WebSocket, app_state: Arc<AppState>, lobby_id: LobbyId, player_id: PlayerId) {
    let conn_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    tracing::info!("[WS:{}] Connected: player {} in lobby {}", conn_id, player_id.0, lobby_id.0);
    let (mut sender, mut receiver) = socket.split();

    let lobby = match app_state.get_lobby(&lobby_id) {
        Ok(l) => l,
        Err(_) => {
            tracing::warn!("[WS:{}] Connect failed: lobby {} not found for player {}", conn_id, lobby_id.0, player_id.0);
            return;
        }
    };

    let mut rx = lobby.tx.subscribe();

    {
        let players = lobby.get_all_players();
        let init_msg = serde_json::to_string(&shared::ServerMessage::PlayerListUpdate {
            players,
        }).unwrap_or_default();
        let _ = sender.send(Message::Text(init_msg.into())).await;

        let status = lobby.game_status.read(|s| *s);
        let prompt = lobby.get_current_prompt_text().unwrap_or_default();
        let scores = lobby.get_all_players();
        let game_msg = serde_json::to_string(&shared::ServerMessage::GameState {
            prompt,
            status,
            scores,
        }).unwrap_or_default();
        let _ = sender.send(Message::Text(game_msg.into())).await;
    }

    let player_id_for_send = player_id.clone();
    let conn_id_for_send = conn_id.clone();
    let mut send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if sender.send(Message::Ping(Default::default())).await.is_err() {
                        break;
                    }
                }
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            tracing::debug!("[WS:{}] sending to player {}: {}...", conn_id_for_send, player_id_for_send.0, &msg[..msg.len().min(100)]);
                            if sender.send(Message::Text(msg.into())).await.is_err() {
                                tracing::warn!("[WS:{}] send failed for player {}, closing", conn_id_for_send, player_id_for_send.0);
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("[WS:{}] receiver lagged behind by {} messages", conn_id_for_send, n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
            }
        }
    });

    let lobby_ref = lobby.clone();
    let player_id_ref = player_id.clone();
    let mut recv_task = tokio::spawn(async move {
        let mut msg_count = 0;
        let mut last_reset = tokio::time::Instant::now();

        while let Some(Ok(msg)) = receiver.next().await {
            let now = tokio::time::Instant::now();
            if now.duration_since(last_reset).as_secs() >= 1 {
                msg_count = 0;
                last_reset = now;
            }
            
            msg_count += 1;
            if msg_count > 20 { // 20 messages per second limit
                tracing::warn!("WebSocket rate limit exceeded by player {}", player_id_ref);
                break;
            }

            if let Message::Text(text) = msg {
                 if let Ok(client_msg) = serde_json::from_str::<shared::ClientMessage>(&text) {
                     match client_msg {
                         shared::ClientMessage::Typing { input } => {
                            let _ = lobby_ref.tx.send(serde_json::to_string(&shared::ServerMessage::PlayerTyping {
                                player_id: player_id_ref.clone(),
                                input,
                            }).unwrap_or_default());
                         },
                         shared::ClientMessage::Submit { input, .. } => {
                             if let Err(e) = lobby_ref.process_guess(&player_id_ref, &input) {
                                 tracing::error!("Error processing guess: {:?}", e);
                             }
                         },
                         shared::ClientMessage::Skip => {
                             if let Err(e) = lobby_ref.process_skip(&player_id_ref) {
                                 tracing::error!("Error processing skip: {:?}", e);
                             }
                         },
                         shared::ClientMessage::ReturnLobbyVote => {
                             if let Err(e) = lobby_ref.process_return_lobby_vote(&player_id_ref) {
                                 tracing::error!("Error processing return to lobby vote: {:?}", e);
                             }
                         }
                     }
                 }
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    // Mark the player as disconnected instead of removing them immediately
    tracing::info!("[WS:{}] Disconnected: marking player {} in lobby {} as disconnected", conn_id, player_id.0, lobby_id.0);
    let _ = app_state.set_player_connected(lobby_id, player_id, false).await;
}
