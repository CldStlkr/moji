use crate::{
    error::AppError, generate_lobby_id, generate_player_id, get_lobby, types::Result, AppState,
    GameStatus, LobbyState, models::{
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

    app_state.lobbies.with(|lobbies| {
        lobbies.insert(lobby_id.clone(), lobby_state);
    })?;


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
    let is_empty = lobby.players.read(|players| players.is_empty())?;

    if is_empty {
        app_state.lobbies.with(|lobbies| {
            lobbies.remove(&lobby_id);
        })?;

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

// NOTE: Leader Only
#[debug_handler]
pub async fn reset_lobby(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(request): Json<StartGameRequest>, // We can reuse StartGameRequest as it has player_id
) -> Result<Json<serde_json::Value>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;
    lobby.reset_lobby(&request.player_id)?;

    Ok(Json(json!({
        "message": "Lobby reset successfully"
    })))
}

#[debug_handler]
pub async fn get_player_info(
    State(app_state): State<Arc<AppState>>,
    Path((lobby_id, player_id)): Path<(String, PlayerId)>,
) -> Result<Json<PlayerData>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;

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
        lives: player.lives,
        is_eliminated: player.is_eliminated,
        is_turn: player.is_turn,
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
            lobby.generate_random_kanji(true)?
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
    let kanji = lobby.generate_random_kanji(true)?;
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
                    // Login success
                    Ok(Json(json!({
                        "message": "Login successful",
                        "user": user,
                        "token": "TODO_SESSION_TOKEN" // Sessions not yet implemented
                    })))
                } else {
                    Err(AppError::AuthError("Invalid password".to_string()))
                }
            } else {
                // Guest accounts cannot log in with a password.
                Err(AppError::AuthError("Account is a guest account. Cannot login with password.".to_string()))
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
                "token": "TODO_SESSION_TOKEN" // Sessions not yet implemented
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
                "message": "Account created",
                "user": user,
                "token": "TODO_SESSION_TOKEN" // Sessions not yet implemented
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

    let lobby = match get_lobby(&app_state, &lobby_id) {
        Ok(l) => l,
        Err(_) => return, // Lobby closed or not found
    };

    let mut rx = lobby.tx.subscribe();

    // Send initial state directly to THIS client (not broadcast) so they
    // don't miss state that was established before their subscription.
    {
        let players = lobby.get_all_players().unwrap_or_default();
        let init_msg = serde_json::to_string(&shared::ServerMessage::PlayerListUpdate {
            players,
        }).unwrap_or_default();
        let _ = sender.send(Message::Text(init_msg.into())).await;

        let status = lobby.game_status.read(|s| *s).unwrap_or(GameStatus::Lobby);
        let kanji = lobby.get_current_kanji().unwrap_or_default().unwrap_or_default();
        let scores = lobby.get_all_players().unwrap_or_default();
        let game_msg = serde_json::to_string(&shared::ServerMessage::GameState {
            kanji,
            status,
            scores,
        }).unwrap_or_default();
        let _ = sender.send(Message::Text(game_msg.into())).await;
    }

    // Now relay broadcast messages to this client
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sender.send(axum::extract::ws::Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WebSocket receiver lagged behind by {} messages", n);
                    // Continue receiving the latest messages
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    let _db_pool = app_state.db_pool.clone();

    let lobby_ref = lobby.clone();
    let player_id_ref = player_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                 if let Ok(client_msg) = serde_json::from_str::<shared::ClientMessage>(&text) {
                     match client_msg {
                         shared::ClientMessage::Typing { input } => {
                            let _ = lobby_ref.tx.send(serde_json::to_string(&shared::ServerMessage::PlayerTyping {
                                player_id: player_id_ref.clone(),
                                input,
                            }).unwrap_or_default());
                         },
                         shared::ClientMessage::Submit { word, kanji } => {
                             if let Err(e) = lobby_ref.process_guess(&player_id_ref, &word, &kanji) {
                                 tracing::error!("Error processing guess: {:?}", e);
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

    // let _ = lobby.remove_player(&player_id);
}
