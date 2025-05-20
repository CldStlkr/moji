use crate::{
    error::{AppError, Result},
    generate_lobby_id, generate_player_id, get_lobby,
    models::basic::{CheckWordResponse, JoinLobbyRequest, KanjiPrompt, PlayerInfo, UserInput},
    AppState, LobbyState,
};
use axum::{
    debug_handler,
    extract::{Json, Path, State},
};
use serde_json::json;
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
    let player_id: String = generate_player_id();
    let lobby_state =
        Arc::new(LobbyState::create().map_err(|e| AppError::InternalError(e.to_string()))?);

    // Add the player who created the lobby
    lobby_state.add_player(player_id.clone(), request.player_name)?;

    lobbies.insert(lobby_id.clone(), lobby_state);

    Ok(Json(json!({
        "message": "Lobby created successfully!",
        "lobby_id": lobby_id,
        "player_id": player_id
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
    let player_id: String = generate_player_id();

    // Add player to the lobby
    lobby.add_player(player_id.clone(), request.player_name.clone())?;

    Ok(Json(json!({
        "message": "Joined lobby successfully!",
        "lobby_id": lobby_id,
        "player_id": player_id
    })))
}

#[debug_handler]
pub async fn get_player_info(
    State(app_state): State<Arc<AppState>>,
    Path((lobby_id, player_id)): Path<(String, String)>,
) -> Result<Json<PlayerInfo>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;
    let name = lobby.get_player_name(&player_id)?;
    let score = lobby.get_player_score(&player_id)?;

    Ok(Json(PlayerInfo { name, score }))
}

#[debug_handler]
pub async fn get_lobby_players(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let lobby = get_lobby(&app_state, &lobby_id)?;
    let players = lobby.get_all_players()?;
    let player_data: Vec<_> = players
        .into_iter()
        .map(|(id, data)| {
            json!({
                "id": id,
                "name": data.name,
                "score": data.score
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
            // Generate a new kanji if none exists
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
    let good_word = word_list.contains(&input_word.to_string());

    let message = if good_kanji && good_word {
        // Update the specific player's score
        let _ = lobby.increment_player_score(&player_id)?;
        "Good guess!".to_string()
    } else if good_kanji {
        "Bad Guess: Correct kanji, but not a valid word.".to_string()
    } else if good_word {
        "Bad Guess: Valid word, but does not contain the correct kanji.".to_string()
    } else {
        "Bad guess: Incorrect kanji and not a valid word.".to_string()
    };

    // Get the current score for this player
    let score = lobby.get_player_score(&player_id)?;

    Ok(Json(CheckWordResponse {
        message,
        score,
        error: None,
    }))
}
