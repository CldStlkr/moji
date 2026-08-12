use crate::{
    utils::{generate_lobby_id, generate_player_id},
    models::{
        user::User,
        game::{GameAction, GameSession},
        GlobalStats,
    },
    error::AppError,
    state::AppState,
    lobby::LobbyData,
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
        let pool_guard = self.db_pool.read().await;

        // Record game session in DB if available
        let game_session_id = if let Some(db_pool) = pool_guard.as_ref() {
            let session = GameSession::create(db_pool, &lobby_id, 1, shared::GameSettings::default()).await?;
            Some(session.id)
        } else {
            None
        };
        drop(pool_guard);

        // Write initial (empty) lobby state to Redis, then add the creator
        let mut initial_data = LobbyData::default();
        initial_data.game_session_id = game_session_id;

        let lobby = self.create_lobby_handle(lobby_id.clone(), initial_data).await?;
        lobby.add_player(player_id.clone(), request.player_name).await?;

        Ok(json!({
            "message": "Lobby created successfully!",
            "lobby_id": lobby_id,
            "player_id": player_id.to_string()
        }))
    }

    async fn get_lobby_info(&self, lobby_id: LobbyId) -> LobbyResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        lobby.get_lobby_info().await.map_err(Into::into)
    }

    async fn update_lobby_settings(&self, lobby_id: LobbyId, request: UpdateSettingsRequest) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        lobby.update_settings(&request.player_id, request.settings).await?;
        Ok(json!({ "message": "Settings updated successfully" }))
    }

    async fn start_game(&self, lobby_id: LobbyId, request: StartGameRequest) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        lobby.start_game(&request.player_id).await?;

        let pool_guard = self.db_pool.read().await;
        if let Some(pool) = pool_guard.as_ref() {
            let _ = GlobalStats::increment_games(pool).await;
        }

        Ok(json!({ "message": "Game started successfully" }))
    }

    async fn reset_lobby(&self, lobby_id: LobbyId, player_id: PlayerId) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        lobby.reset_lobby(&player_id).await?;
        Ok(json!({ "message": "Lobby reset successfully" }))
    }

    async fn get_lobby_players(&self, lobby_id: LobbyId) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        let data = lobby.get_data().await?;
        let player_data: Vec<_> = data.get_all_players().into_iter().map(|p| {
            json!({ "id": p.id, "name": p.name, "score": p.score, "joined_at": p.joined_at })
        }).collect();
        Ok(json!({ "players": player_data }))
    }

    async fn join_lobby(&self, lobby_id: LobbyId, request: JoinLobbyRequest) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id).await?;

        if request.joining_from_public_list {
            let data = lobby.get_data().await?;
            if !data.settings.is_public {
                return Err(AppError::InvalidInput("This lobby is now private".into()).into());
            }
        }

        let player_id = request.player_id.unwrap_or_else(generate_player_id);
        lobby.add_player(player_id.clone(), request.player_name.clone()).await?;

        // Log join action to DB if we have a session
        let data = lobby.get_data().await?;
        if let Some(game_id) = data.game_session_id {
            let pool_guard = self.db_pool.read().await;
            if let Some(db_pool) = pool_guard.as_ref() {
                let db = Arc::clone(db_pool);
                let name = request.player_name.clone();
                let pid = player_id.to_string();
                tokio::spawn(async move {
                    let action_data = json!({ "player_id": pid, "player_name": name });
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
        let lobby = self.get_lobby(&lobby_id).await?;
        let data = lobby.get_data().await?;
        let prompt = match data.get_current_prompt_text() {
            Some(p) => p,
            None => lobby.generate_random_prompt(true, true).await?,
        };
        Ok(PromptResponse { prompt })
    }

    async fn generate_new_prompt(&self, lobby_id: LobbyId) -> PromptResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        let prompt = lobby.generate_random_prompt(true, true).await?;
        Ok(PromptResponse { prompt })
    }

    async fn check_username(&self, username: String) -> JsonResult {
        validate_username(&username)?;
        let pool_guard = self.db_pool.read().await;
        let db_pool = pool_guard.as_ref()
            .ok_or_else(|| ServerFnError::new("Database not configured"))?;
        let user = User::find_by_username(db_pool, &username).await?;
        if let Some(user) = user {
            Ok(json!({ "available": false, "is_guest": user.is_guest }))
        } else {
            Ok(json!({ "available": true, "is_guest": false }))
        }
    }

    async fn authenticate(&self, request: shared::AuthRequest) -> JsonResult {
        validate_username(&request.username)?;
        let pool_guard = self.db_pool.read().await;
        let db_pool = pool_guard.as_ref()
            .ok_or_else(|| ServerFnError::new("Database not configured"))?;
        let existing_user = User::find_by_username(db_pool, &request.username).await?;

        if let Some(user) = existing_user {
            if let Some(password) = request.password {
                if let Some(hash) = &user.password_hash {
                    let parsed_hash = PasswordHash::new(hash)
                        .map_err(|e| ServerFnError::new(e.to_string()))?;
                    if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok() {
                        Ok(json!({ "message": "Login successful", "user": &user, "token": generate_jwt(&user.id.to_string())? }))
                    } else {
                        Err(ServerFnError::new("Invalid password"))
                    }
                } else {
                    Err(ServerFnError::new("Name currently in use"))
                }
            } else {
                Err(ServerFnError::new("Name currently in use"))
            }
        } else if request.create_guest {
            let user = User::create(db_pool, &request.username, None, true).await?;
            Ok(json!({ "message": "Guest account created", "user": &user, "token": generate_jwt(&user.id.to_string())? }))
        } else if let Some(password) = request.password {
            let salt = SaltString::generate(&mut OsRng);
            let password_hash = Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|e| ServerFnError::new(e.to_string()))?.to_string();
            let user = User::create(db_pool, &request.username, Some(password_hash), false).await?;
            Ok(json!({ "message": "Account created", "user": &user, "token": generate_jwt(&user.id.to_string())? }))
        } else {
            Err(ServerFnError::new("Password required to register"))
        }
    }

    async fn get_player_info(&self, lobby_id: LobbyId, player_id: PlayerId) -> PlayerResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        let data = lobby.get_data().await?;
        data.get_all_players().into_iter().find(|p| p.id == player_id)
            .ok_or_else(|| ServerFnError::new(format!("Player not found: {}", player_id)))
    }

    async fn leave_lobby(&self, lobby_id: LobbyId, player_id: PlayerId) -> JsonResult {
        let lobby = match self.get_lobby(&lobby_id).await {
            Ok(l) => l,
            Err(AppError::LobbyNotFound(_)) => return Ok(json!({ "message": "Lobby already cleaned up" })),
            Err(e) => return Err(e.into()),
        };

        lobby.remove_player(&player_id).await?;

        // If lobby is now empty, end the game session and delete from Redis
        let data = lobby.get_data().await.unwrap_or_default();
        if data.players.is_empty() {
            use redis::AsyncCommands;
            if let Ok(mut conn) = lobby.redis.get_multiplexed_async_connection().await {
                let _: () = conn.del(format!("lobby:{}", lobby_id.0)).await.unwrap_or(());
            }
            if let Some(game_id) = data.game_session_id {
                let pool_guard = self.db_pool.read().await;
                if let Some(db_pool) = pool_guard.as_ref() {
                    let pool = Arc::clone(db_pool);
                    tokio::spawn(async move { let _ = GameSession::end_session(&pool, game_id).await; });
                }
            }
        }

        Ok(json!({ "message": "Left lobby" }))
    }

    async fn logout(&self, username: String) -> JsonResult {
        let pool_guard = self.db_pool.read().await;
        let db_pool = pool_guard.as_ref()
            .ok_or_else(|| ServerFnError::new("Database not configured"))?;
        User::delete_guest_by_username(db_pool, &username).await?;
        Ok(json!({ "message": "Logged out" }))
    }

    async fn set_player_connected(&self, lobby_id: LobbyId, player_id: PlayerId, is_connected: bool) -> JsonResult {
        let lobby = match self.get_lobby(&lobby_id).await {
            Ok(l) => l,
            Err(AppError::LobbyNotFound(_)) => return Ok(json!({ "message": "Lobby not found" })),
            Err(e) => return Err(e.into()),
        };

        lobby.set_player_connected(&player_id, is_connected).await?;

        if !is_connected {
            let all_gone = lobby.all_disconnected().await.unwrap_or(false);
            if all_gone {
                // Read cleanup_generation before spawning
                let generation = lobby.get_data().await.map(|d| d.cleanup_generation).unwrap_or(0);
                let lobby_clone = lobby.clone();
                let lid = lobby_id.clone();
                let db_pool = self.db_pool.read().await.clone();

                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

                    // Cancel if a player reconnected (cleanup_generation changed)
                    let current_gen = lobby_clone.get_data().await
                        .map(|d| d.cleanup_generation).unwrap_or(0);
                    if current_gen != generation {
                        tracing::info!("Lobby {} cleanup cancelled: player reconnected", lid.0);
                        return;
                    }

                    if !lobby_clone.all_disconnected().await.unwrap_or(false) {
                        return;
                    }

                    tracing::info!("Lobby {} cleaned up after 60s idle", lid.0);
                    let data = lobby_clone.get_data().await.unwrap_or_default();

                    use redis::AsyncCommands;
                    if let Ok(mut conn) = lobby_clone.redis.get_multiplexed_async_connection().await {
                        let _: () = conn.del(format!("lobby:{}", lid.0)).await.unwrap_or(());
                    }

                    if let Some(game_id) = data.game_session_id {
                        if let Some(pool) = db_pool.as_ref() {
                            let _ = crate::models::game::GameSession::end_session(pool, game_id).await;
                        }
                    }
                });
            }
        }

        Ok(json!({ "message": "Connection status updated" }))
    }

    async fn kick_player(&self, lobby_id: LobbyId, requestor_id: PlayerId, target_player_id: PlayerId) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        lobby.kick_player(&requestor_id, &target_player_id).await?;

        let data = lobby.get_data().await?;
        if data.players.is_empty() {
            use redis::AsyncCommands;
            if let Ok(mut conn) = lobby.redis.get_multiplexed_async_connection().await {
                let _: () = conn.del(format!("lobby:{}", lobby_id.0)).await.unwrap_or(());
            }
        }

        Ok(json!({ "message": "Player kicked" }))
    }

    async fn promote_leader(&self, lobby_id: LobbyId, requestor_id: PlayerId, target_player_id: PlayerId) -> JsonResult {
        let lobby = self.get_lobby(&lobby_id).await?;
        lobby.promote_leader(&requestor_id, &target_player_id).await?;
        Ok(json!({ "message": "Leader promoted" }))
    }

    async fn get_public_lobbies(&self) -> Result<Vec<shared::LobbySummary>, leptos::server_fn::error::ServerFnError> {
        self.get_public_lobbies().await.map_err(Into::into)
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

    let claims = match result {
        Ok(token_data) => token_data.claims,
        Err(e) => {
            let reason = match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => "Token expired",
                jsonwebtoken::errors::ErrorKind::InvalidToken => "Invalid token or missing",
                jsonwebtoken::errors::ErrorKind::InvalidSignature => "Invalid signature",
                _ => "Unauthorized",
            };
            tracing::warn!("Unauthorized WebSocket connection attempt to lobby {}: {}", lobby_id.0, reason);
            return (axum::http::StatusCode::UNAUTHORIZED, reason).into_response();
        }
    };

    let user_db_uuid = uuid::Uuid::parse_str(&claims.sub).ok();

    ws.on_upgrade(move |socket| async move {
        {
            let pool_guard = app_state.db_pool.read().await;
            if let (Some(uid), Some(pool)) = (user_db_uuid, pool_guard.as_ref()) {
                let _ = User::update_last_seen_by_id(pool, uid).await;
            }
        }

        let _ = app_state.set_player_connected(lobby_id.clone(), player_id.clone(), true).await;
        handle_socket(socket, app_state, lobby_id, player_id, user_db_uuid).await
    })
}
async fn handle_socket(socket: WebSocket, app_state: Arc<AppState>, lobby_id: LobbyId, player_id: PlayerId, user_db_uuid: Option<uuid::Uuid>) {
    let conn_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    tracing::info!("[WS:{}] Connected: player {} in lobby {}", conn_id, player_id.0, lobby_id.0);
    let (mut sender, mut receiver) = socket.split();

    let lobby = match app_state.get_lobby(&lobby_id).await {
        Ok(l) => l,
        Err(_) => {
            tracing::warn!("[WS:{}] Connect failed: lobby {} not found for player {}", conn_id, lobby_id.0, player_id.0);
            return;
        }
    };

    // Subscribe to local fallback channel (used when Redis is not configured)
    let local_rx = lobby.tx.subscribe();

    {
        // One Redis read to get current state, sent immediately to the connecting client.
        let data = lobby.get_data().await.unwrap_or_default();
        let players = data.get_all_players();

        let init_msg = serde_json::to_string(&shared::ServerMessage::PlayerListUpdate {
            players: players.clone(),
        }).unwrap_or_default();
        let _ = sender.send(Message::Text(init_msg.into())).await;

        let game_msg = serde_json::to_string(&shared::ServerMessage::GameState {
            prompt: data.get_current_prompt_text().unwrap_or_default(),
            status: data.game_status,
            scores: players,
            timer_expires_at: data.timer_expires_at,
        }).unwrap_or_default();
        let _ = sender.send(Message::Text(game_msg.into())).await;
    }

    let redis_client = app_state.redis.clone();
    let channel = format!("lobby:{}", lobby_id.0);
    let player_id_for_send = player_id.clone();
    let conn_id_for_send = conn_id.clone();
    let mut send_task = tokio::spawn(async move {
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));

        if let Some(client) = redis_client {
            // Redis path: subscribe to the lobby's pub/sub channel.
            // Any pod that publishes to this channel will be received here,
            // enabling cross-pod fan-out.
            let mut pubsub = match client.get_async_pubsub().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("[WS:{}] Redis pubsub connection failed: {:?}", conn_id_for_send, e);
                    return;
                }
            };
            if let Err(e) = pubsub.subscribe(&channel).await {
                tracing::error!("[WS:{}] Redis subscribe failed for {}: {:?}", conn_id_for_send, channel, e);
                return;
            }
            let mut stream = pubsub.on_message();
            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
                        if sender.send(Message::Ping(Default::default())).await.is_err() {
                            break;
                        }
                    }
                    msg = stream.next() => {
                        match msg {
                            Some(m) => {
                                if let Ok(payload) = m.get_payload::<String>() {
                                    tracing::debug!("[WS:{}] sending to player {}: {}...", conn_id_for_send, player_id_for_send.0, &payload[..payload.len().min(100)]);
                                    if sender.send(Message::Text(payload.into())).await.is_err() {
                                        tracing::warn!("[WS:{}] send failed for player {}, closing", conn_id_for_send, player_id_for_send.0);
                                        break;
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        } else {
            // Fallback path: no Redis configured (tests, local dev without Redis).
            // Uses the in-process broadcast channel — single-pod only.
            let mut rx = local_rx;
            loop {
                tokio::select! {
                    _ = ping_interval.tick() => {
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
        }
    });

    let lobby_ref = lobby.clone();
    let player_id_ref = player_id.clone();
    let app_state_for_recv = app_state.clone();
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
                            lobby_ref.broadcast(shared::ServerMessage::PlayerTyping {
                                player_id: player_id_ref.clone(),
                                input,
                            });
                         },
                         shared::ClientMessage::Submit { input, .. } => {
                             let pool_guard = app_state_for_recv.db_pool.read().await;
                             if let (Some(uid), Some(pool)) = (user_db_uuid, pool_guard.as_ref()) {
                                 let pool_clone = pool.clone();
                                 tokio::spawn(async move {
                                     let _ = User::update_last_seen_by_id(&pool_clone, uid).await;
                                 });
                             }
                             if let Err(e) = lobby_ref.process_guess(&player_id_ref, &input).await {
                                  tracing::error!("Error processing guess: {:?}", e);
                             }
                         },
                         shared::ClientMessage::Skip => {
                             if let Err(e) = lobby_ref.process_skip(&player_id_ref).await {
                                 tracing::error!("Error processing skip: {:?}", e);
                             }
                         },
                         shared::ClientMessage::ReturnLobbyVote => {
                             if let Err(e) = lobby_ref.process_return_lobby_vote(&player_id_ref).await {
                                 tracing::error!("Error processing return to lobby vote: {:?}", e);
                             }
                         },
                         shared::ClientMessage::Chat { message } => {
                             let name = lobby_ref.get_player_name(&player_id_ref).await.unwrap_or_else(|_| "Unknown".to_string());
                             // Apply profanity filter
                             let clean_message = message.censor();
                             lobby_ref.broadcast(shared::ServerMessage::ChatMessage(shared::ChatMessage {
                                 player_id: player_id_ref.clone(),
                                 player_name: name,
                                 message: clean_message,
                             }));
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

    tracing::info!("[WS:{}] Disconnected: marking player {} in lobby {} as disconnected", conn_id, player_id.0, lobby_id.0);
    let _ = app_state.set_player_connected(lobby_id, player_id, false).await;
}

pub async fn get_global_stats(
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let pool_guard = app_state.db_pool.read().await;
    if let Some(pool) = pool_guard.as_ref() {
        let stats = match GlobalStats::get(pool).await {
            Ok(s) => s,
            Err(_) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to load stats").into_response(),
        };
        let online_count = User::get_online_count(pool).await.unwrap_or_default();

        let response = serde_json::json!({
            "total_unique_visitors": stats.total_unique_visitors,
            "total_games_played": stats.total_games_played,
            "total_words_guessed": stats.total_words_guessed,
            "current_online_players": online_count,
        });

        axum::Json(response).into_response()
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, "Database unavailable").into_response()
    }
}
