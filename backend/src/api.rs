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
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
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
    JoinLobbyRequest, PromptResponse, LobbyInfo, LobbyId,
    PlayerData, PlayerId, StartGameRequest, UpdateSettingsRequest, ApiContext
};
use std::sync::Arc;
use async_trait::async_trait;
use leptos::server_fn::error::ServerFnError;

#[async_trait]
impl ApiContext for AppState {
    async fn create_lobby(&self, request: JoinLobbyRequest) -> Result<serde_json::Value, ServerFnError> {
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

    async fn get_lobby_info(&self, lobby_id: LobbyId) -> Result<LobbyInfo, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;
        Ok(lobby.get_lobby_info(&lobby_id))
    }

    async fn update_lobby_settings(&self, lobby_id: LobbyId, request: UpdateSettingsRequest) -> Result<serde_json::Value, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;
        lobby.update_settings(&request.player_id, request.settings)?;
        Ok(json!({ "message": "Settings updated successfully" }))
    }

    async fn start_game(&self, lobby_id: LobbyId, request: StartGameRequest) -> Result<serde_json::Value, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;
        lobby.start_game(&request.player_id)?;
        Ok(json!({ "message": "Game started successfully" }))
    }

    async fn reset_lobby(&self, lobby_id: LobbyId, player_id: PlayerId) -> Result<serde_json::Value, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;

        lobby.reset_lobby(&player_id)?;
        Ok(json!({ "message": "Lobby reset successfully" }))
    }

    async fn get_lobby_players(&self, lobby_id: LobbyId) -> Result<serde_json::Value, ServerFnError> {
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

    async fn join_lobby(&self, lobby_id: LobbyId, request: JoinLobbyRequest) -> Result<serde_json::Value, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;

        let player_id = generate_player_id();
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

    async fn get_prompt(&self, lobby_id: LobbyId) -> Result<PromptResponse, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;

        let prompt = match lobby.get_current_prompt_text() {
            Some(prompt) => prompt,
            None => lobby.generate_random_prompt(true)?
        };
        Ok(PromptResponse { prompt })
    }

    async fn generate_new_prompt(&self, lobby_id: LobbyId) -> Result<PromptResponse, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;

        let prompt = lobby.generate_random_prompt(true)?;
        Ok(PromptResponse { prompt })
    }

    async fn check_username(&self, username: String) -> Result<serde_json::Value, ServerFnError> {
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

    async fn authenticate(&self, request: shared::AuthRequest) -> Result<serde_json::Value, ServerFnError> {
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
                            "user": user,
                            "token": "TODO_SESSION_TOKEN"
                        }))
                    } else {
                        Err(ServerFnError::new("Invalid password"))
                    }
                } else {
                    Err(ServerFnError::new("Account is a guest account. Cannot login with password."))
                }
            } else {
                Err(ServerFnError::new("Password required"))
            }
        } else if request.create_guest {
            let user = User::create(db_pool, &request.username, None, true).await?;
            Ok(json!({
                "message": "Guest account created",
                "user": user,
                "token": "TODO_SESSION_TOKEN"
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
                "user": user,
                "token": "TODO_SESSION_TOKEN"
            }))
        } else {
            Err(ServerFnError::new("Password required to register"))
        }
    }

    async fn get_player_info(&self, lobby_id: LobbyId, player_id: PlayerId) -> Result<PlayerData, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;

        let players = lobby.get_all_players();
        let player = players.into_iter().find(|p| p.id == player_id)
            .ok_or_else(|| ServerFnError::new(format!("Player not found: {}", player_id)))?;

        Ok(player)
    }

    async fn leave_lobby(&self, lobby_id: LobbyId, player_id: PlayerId) -> Result<serde_json::Value, ServerFnError> {
        let lobby = self.get_lobby(&lobby_id)?;

        lobby.remove_player(&player_id);

        let is_empty = lobby.players.read(|players| players.is_empty());

        if is_empty {
            self.lobbies.write(|lobbies| { lobbies.remove(&lobby_id); });

            if let Some(game_id) = lobby.game_session_id {
                if let Some(db_pool) = &self.db_pool {
                    let pool = db_pool.clone();
                    tokio::spawn(async move {
                        let _ = GameSession::end_session(&pool, game_id).await;
                    });
                }
            }
        }

        Ok(json!({ "message": "Left lobby" }))
    }

    async fn logout(&self, username: String) -> Result<serde_json::Value, ServerFnError> {
        let db_pool = self.db_pool.as_ref()
            .ok_or_else(|| ServerFnError::new("Database not configured"))?;

        User::delete_guest_by_username(db_pool, &username).await?;

        Ok(json!({ "message": "Logged out" }))
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
    Path((lobby_id, player_id)): Path<(LobbyId, PlayerId)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, app_state, lobby_id, player_id))
}

async fn handle_socket(socket: WebSocket, app_state: Arc<AppState>, lobby_id: LobbyId, player_id: PlayerId) {
    let (mut sender, mut receiver) = socket.split();

    let lobby = match app_state.get_lobby(&lobby_id) {
        Ok(l) => l,
        Err(_) => return, // Lobby closed or not found
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

    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sender.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WebSocket receiver lagged behind by {} messages", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

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
                         shared::ClientMessage::Submit { input, .. } => {
                             if let Err(e) = lobby_ref.process_guess(&player_id_ref, &input) {
                                 tracing::error!("Error processing guess: {:?}", e);
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
}
