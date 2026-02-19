use crate::{
    error::AppError, generate_lobby_id, generate_player_id, get_lobby, types::Result, AppState,
    LobbyState, models::{
        user::User,
        game::{GameAction, GameSession},
    },
};
use axum::{
    debug_handler,
    extract::{Json, Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use serde::Deserialize;
use argon2::{
    Argon2, password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng
    }
};
use shared::{
    JoinLobbyRequest, KanjiPrompt, LobbyInfo, PlayerData, PlayerId,
    StartGameRequest, UpdateSettingsRequest
};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: Option<String>,
    pub create_guest: bool,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub username: String,
}


#[debug_handler]
pub async fn logout(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<serde_json::Value>> {
    let db_pool = app_state.db_pool.as_ref()
    .ok_or(AppError::InternalError("Database not configured".to_string()))?;

    User::delete_guest_by_username(db_pool, &payload.username).await?;

    Ok(Json(json!({ "message": "Logged out" })))
}

#[debug_handler]
pub async fn create_lobby(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<JoinLobbyRequest>,
) -> Result<Json<serde_json::Value>> {
    let lobby_id: String = generate_lobby_id();
    let player_id: PlayerId = generate_player_id();

    let game_session_id = if let Some(db_pool) = &app_state.db_pool {
        let default_settings = shared::GameSettings::default();

        let session = GameSession::create(db_pool, &lobby_id, 1, default_settings).await?;

        Some(session.id)
    } else {
        None // In case we run without DB?
    };

    let lobby_state =
        Arc::new(LobbyState::new(
        Arc::clone(&app_state.kanji_data),
        Arc::clone(&app_state.word_data),
        game_session_id
    ));

    // Add the player who created the lobby (will automatically become leader)
    let _ = lobby_state.add_player(player_id.clone(), request.player_name)?;

    {
        let mut lobbies = app_state
            .lobbies
            .lock()
            .map_err(|e| AppError::LockError(e.to_string()))?;

        lobbies.insert(lobby_id.clone(), lobby_state);
    }


    Ok(Json(json!({
        "message": "Lobby created successfully!",
        "lobby_id": lobby_id,
        "player_id": player_id.to_string()
    })))
}


#[derive(Deserialize)]
pub struct LeaveLobbyRequest {
    pub player_id: PlayerId,
}

#[debug_handler]
pub async fn leave_lobby(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(request): Json<LeaveLobbyRequest>,
) -> Result<Json<serde_json::Value>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;

    lobby.remove_player(&request.player_id)?;

    // Check if empty and remove lobby if so
    let is_empty = {
        let players = lobby.players.lock().map_err(|e| AppError::LockError(e.to_string()))?;
        players.is_empty()
    };

    if is_empty {
        {
            let mut lobbies = app_state.lobbies.lock().map_err(|e| AppError::LockError(e.to_string()))?;
            lobbies.remove(&lobby_id);
        }

        // End game session in DB if exists
        if let Some(game_id) = lobby.game_session_id {
            if let Some(db_pool) = &app_state.db_pool {
                let pool = db_pool.clone();
                tokio::spawn(async move {
                    let _ = GameSession::end_session(&pool, game_id).await;
                });
            }
        }
    }

    Ok(Json(json!({ "message": "Left lobby" })))
}

#[debug_handler]
pub async fn join_lobby(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(request): Json<JoinLobbyRequest>,
) -> Result<Json<serde_json::Value>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;

    let player_id = generate_player_id();
    let _ = lobby.add_player(player_id.clone(), request.player_name.clone())?;

    if let Some(game_id) = lobby.game_session_id {
        if let Some(db_pool) = &app_state.db_pool {
            let db = Arc::clone(db_pool);
            let name = request.player_name.clone();
            let pid = player_id.to_string();

            tokio::spawn(async move {
                let action_data = json!({
                    "player_id": pid,
                    "player_name": name
                });

                if let Err(e) = GameAction::create(
                    &db,
                    game_id,
                    None,
                    "player_joined",
                    action_data
                ).await {
                    tracing::error!("Failed to log player join: {:?}", e);
                }

            });
        }
    }


    Ok(Json(json!({
        "message": "Joined lobby successfully!",
        "lobby_id": lobby_id,
        "player_id": player_id
    })))
}

#[debug_handler]
pub async fn get_lobby_info(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<LobbyInfo>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;
    let lobby_info = lobby.get_lobby_info(&lobby_id)?;
    Ok(Json(lobby_info))
}

// NOTE: Leader Only
#[debug_handler]
pub async fn update_lobby_settings(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(request): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;
    lobby.update_settings(&request.player_id, request.settings)?;

    Ok(Json(json!({
        "message": "Settings updated successfully"
    })))
}

// NOTE: Leader Only
#[debug_handler]
pub async fn start_game(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(request): Json<StartGameRequest>,
) -> Result<Json<serde_json::Value>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;
    lobby.start_game(&request.player_id)?;

    Ok(Json(json!({
        "message": "Game started successfully"
    })))
}

#[debug_handler]
pub async fn get_player_info(
    State(app_state): State<Arc<AppState>>,
    Path((lobby_id, player_id)): Path<(String, PlayerId)>,
) -> Result<Json<PlayerData>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;

    // Get the full player data from the lobby
    let players = lobby.get_all_players()?;
    let player = players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| AppError::PlayerNotFound(player_id.to_string()))?;

    // Convert internal PlayerData to API PlayerData
    Ok(Json(PlayerData {
        id: player.id.clone(),
        name: player.name.clone(),
        score: player.score,
        joined_at: player.joined_at.clone(),
    }))
}

#[debug_handler]
pub async fn get_lobby_players(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;
    let players = lobby.get_all_players()?;

    // Convert internal PlayerData to API PlayerData
    let player_data: Vec<_> = players
        .into_iter()
        .map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "score": p.score,
                "joined_at": p.joined_at
            })
        })
        .collect();

    Ok(Json(json!({
        "players": player_data
    })))
}

#[debug_handler]
pub async fn get_kanji(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<KanjiPrompt>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;

    // Try to get the current kanji first
    let kanji = match lobby.get_current_kanji()? {
        Some(kanji) => kanji,
        None => {
            lobby.generate_random_kanji()?
        }
    };
    Ok(Json(KanjiPrompt { kanji }))
}

#[debug_handler]
pub async fn generate_new_kanji(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<KanjiPrompt>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;

    // Always generate a new kanji
    let kanji = lobby.generate_random_kanji()?;
    Ok(Json(KanjiPrompt { kanji }))
}

pub async fn check_username(
    State(app_state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let db_pool = app_state.db_pool.as_ref()
        .ok_or(AppError::InternalError("Database not configured".to_string()))?;

    let user = User::find_by_username(db_pool, &username).await?;

    if let Some(user) = user {
        Ok(Json(json!({
            "available": false,
            "is_guest": user.is_guest
        })))
    } else {
        Ok(Json(json!({
            "available": true,
            "is_guest": false
        })))
    }
}


#[debug_handler]
pub async fn authenticate(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<serde_json::Value>> {
    let db_pool = app_state.db_pool.as_ref()
        .ok_or(AppError::InternalError("Database not configured".to_string()))?;

    let existing_user = User::find_by_username(db_pool, &payload.username).await?;

    if let Some(user) = existing_user {
        // CASE: User exists via Login
        if let Some(password) = payload.password {
            // Verify password if user has one
            if let Some(hash) = &user.password_hash {
                let parsed_hash = PasswordHash::new(hash)
                    .map_err(|_| AppError::InternalError("Invalid password hash".to_string()))?;

                if Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok() {
                    // Login Cuccess
                    Ok(Json(json!({
                        "message": "Login successful",
                        "user": user,
                        // TODO: Sessions
                        "token": "TODO_SESSION_TOKEN"
                    })))
                } else {
                    Err(AppError::AuthError("Invalid password".to_string()))
                }
            } else {
                // User exits but no password (guest account).
                // TODO: Create password for existing guest conversion flow
                // For now, if they provide a password but user is guest, it's invalid unless we
                // support "claim guest".
                Err(AppError::AuthError("Account is a guest accoutn. Cannot login with password yet".to_string()))
            }
        } else {
            // No password provided for existing user
            Err(AppError::AuthError("Password required".to_string()))
        }
    } else {
        // CASE: new user (Register or Guest)
        if payload.create_guest {
            // Create guest
            let user = User::create(db_pool, &payload.username, None, true).await?;

            Ok(Json(json!({
                "message": "Guest account created",
                "user": user,
                // TODO: Sessions
                "token": "TODO_SESSION_TOKEN"
            })))
        } else if let Some(password) = payload.password {
            // Register real user
            let salt = SaltString::generate(&mut OsRng);
            let password_hash = Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map_err(|e| AppError::InternalError(e.to_string()))?
                .to_string();
            let user = User::create(db_pool, &payload.username, Some(password_hash), false).await?;

            Ok(Json(json!({
                "message": "Account craeted",
                "user": user,
                // TODO: Sessions
                "token":"TODO_SESSION_TOKEN"
            })))
        } else {
            Err(AppError::InvalidInput("Password required to register".to_string()))
        }
    }

}


#[debug_handler]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
    Path((lobby_id, player_id)): Path<(String, PlayerId)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, app_state, lobby_id, player_id))
}

async fn handle_socket(socket: WebSocket, app_state: Arc<AppState>, lobby_id: String, player_id: PlayerId) {
    let (mut sender, mut receiver) = socket.split();

    // Get Lobby and Subscribe
    let lobby = match get_lobby(&app_state, &lobby_id) {
        Ok(l) => l,
        Err(_) => return, // Lobby closed or not found
    };

    let mut rx = lobby.tx.subscribe();

    // Spawn task to forward broadcast messages to this client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Capture DB pool for the receiver task if available
    let db_pool = app_state.db_pool.clone();

    // Listen for client messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(client_msg) = serde_json::from_str::<shared::ClientMessage>(&text) {
                    match client_msg {
                        shared::ClientMessage::Typing { input } => {
                            // Broadcast typing status to everyone else
                            let _ = lobby.tx.send(serde_json::to_string(&shared::ServerMessage::PlayerTyping {
                                player_id: player_id.clone(),
                                input,
                            }).unwrap());
                        }
                        shared::ClientMessage::Submit { word, kanji } => {
                            let word_list = &lobby.word_list;
                            let input_word = word.trim();
                            let input_kanji = kanji.trim();

                            let good_kanji = input_word.contains(input_kanji);
                            let good_word = word_list.contains(input_word);
                            let is_correct = good_kanji && good_word;

                            let (message, new_kanji_opt) = if is_correct {
                                let _ = lobby.increment_player_score(&player_id);
                                let new_k = lobby.generate_random_kanji().ok();

                                let _ = lobby.tx.send(serde_json::to_string(&shared::ServerMessage::PlayerListUpdate {
                                    players: lobby.get_all_players().unwrap_or_default()
                                }).unwrap());

                                ("Good guess!".to_string(), new_k)
                            } else if good_kanji {
                                ("Bad Guess: Correct kanji, but not valid word.".to_string(), None)
                            } else if good_word {
                                ("Bad Guess: Valid word, but does not contain the correct kanji.".to_string(), None)
                            } else {
                                ("Bad Guess: Incorrect kanji and not a valid word.".to_string(), None)
                            };

                            let score = lobby.get_player_score(&player_id).unwrap_or(0);

                            // Broadcast the verification result so clients can show animations/toasts
                            let _ = lobby.tx.send(serde_json::to_string(&shared::ServerMessage::WordChecked {
                                player_id: player_id.clone(),
                                result: shared::CheckWordResponse {
                                    message: message.clone(),
                                    score,
                                    error: None,
                                    kanji: new_kanji_opt,
                                },
                            }).unwrap());

                            // NEW: Persist to DB if session exists
                            if let (Some(game_id), Some(pool)) = (lobby.game_session_id, &db_pool) {
                                let action_data = json!({
                                    "player_id": player_id.to_string(),
                                    "word": input_word,
                                    "kanji": input_kanji,
                                    "correct": is_correct,
                                    "score": score,
                                    "message": message
                                });

                                // Spawn a background task to record the action
                                let pool = Arc::clone(pool);
                                tokio::spawn(async move {
                                    if let Err(e) = GameAction::create(
                                        &pool,
                                        game_id,
                                        None, // User ID is None for guests
                                        "word_submission",
                                        action_data
                                    ).await {
                                        tracing::error!("Failed to log word submission: {:?}", e);
                                    }
                                });
                            }
                        }
                    }
                }
            }
        }
    });

    // If any one of these tasks exit, abort the other
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}
