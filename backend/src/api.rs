use crate::{
    error::AppError, generate_lobby_id, generate_player_id, get_lobby, types::Result, AppState,
    LobbyState,
};
use axum::{
    debug_handler,
    extract::{Json, Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde_json::json;
use shared::{
    CheckWordResponse, JoinLobbyRequest, KanjiPrompt, LobbyInfo, PlayerData, PlayerId,
    StartGameRequest, UpdateSettingsRequest, UserInput,
};
use std::sync::Arc;

#[debug_handler]
pub async fn create_lobby(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<JoinLobbyRequest>,
) -> Result<Json<serde_json::Value>> {
    let mut lobbies = app_state
        .lobbies
        .lock()
        .map_err(|e| AppError::LockError(e.to_string()))?;

    // Generate random 6-character alphanumeric lobby ID
    let lobby_id: String = generate_lobby_id();
    // Generate random player ID
    let player_id: PlayerId = generate_player_id();
    let lobby_state =
        Arc::new(LobbyState::new(Arc::clone(&app_state.kanji_data), Arc::clone(&app_state.word_data)));

    // Add the player who created the lobby (will automatically become leader)
    let _ = lobby_state.add_player(player_id.clone(), request.player_name)?;

    lobbies.insert(lobby_id.clone(), lobby_state);

    Ok(Json(json!({
        "message": "Lobby created successfully!",
        "lobby_id": lobby_id,
        "player_id": player_id.to_string()
    })))
}

#[debug_handler]
pub async fn join_lobby(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(request): Json<JoinLobbyRequest>,
) -> Result<Json<serde_json::Value>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;

    // Generate a unique player ID
    let player_id = generate_player_id();

    // Add player to the lobby
    let _ = lobby.add_player(player_id.clone(), request.player_name.clone())?;

    Ok(Json(json!({
        "message": "Joined lobby successfully!",
        "lobby_id": lobby_id,
        "player_id": player_id
    })))
}

// Get complete lobby information
#[debug_handler]
pub async fn get_lobby_info(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<LobbyInfo>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;
    let lobby_info = lobby.get_lobby_info(&lobby_id)?;
    Ok(Json(lobby_info))
}

// Update lobby settings (leader only)
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

// Start game (leader only)
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

#[debug_handler]
pub async fn check_word(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(input): Json<UserInput>,
) -> Result<Json<CheckWordResponse>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;

    let word_list = &lobby.word_list;
    let input_word = input.word.trim();
    let input_kanji = input.kanji.trim();
    let player_id = input.player_id;

    let good_kanji = input_word.contains(input_kanji);
    let good_word = word_list.contains(input_word);

    let (message, new_kanji) = if good_kanji && good_word {
        // Update the specific player's score
        let _ = lobby.increment_player_score(&player_id)?;
        let new_kanji = lobby.generate_weighted_random_kanji()?;
        ("Good guess!".to_string(), Some(new_kanji))
    } else if good_kanji {
        (
            "Bad Guess: Correct kanji, but not a valid word.".to_string(),
            None,
        )
    } else if good_word {
        (
            "Bad Guess: Valid word, but does not contain the correct kanji.".to_string(),
            None,
        )
    } else {
        (
            "Bad guess: Incorrect kanji and not a valid word.".to_string(),
            None,
        )
    };

    // Get the current score for this player
    let score = lobby.get_player_score(&player_id)?;

    Ok(Json(CheckWordResponse {
        message,
        score,
        error: None,
        kanji: new_kanji,
    }))
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

                            let (message, new_kanji_opt) = if good_kanji && good_word {
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
                                     message,
                                     score,
                                     error: None,
                                     kanji: new_kanji_opt,
                                 },
                             }).unwrap());
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
