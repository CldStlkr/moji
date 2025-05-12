use crate::{
    models::basic::{CheckWordResponse, JoinLobbyRequest, KanjiPrompt, PlayerInfo, UserInput},
    AppState, LobbyState,
};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use rand::{distr::Alphanumeric, Rng};
use serde_json::json;
use std::sync::Arc;

pub async fn create_lobby(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<JoinLobbyRequest>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let mut lobbies = app_state.lobbies.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to acquire lock on lobbies",
        )
    })?;

    // Generate random 6-character alphanumeric lobby ID
    let lobby_id: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    // Generate random player ID
    let player_id: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    let lobby_state = Arc::new(
        LobbyState::create()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create lobby"))?,
    );

    // Add the player who created the lobby
    lobby_state.add_player(player_id.clone(), request.player_name);

    lobbies.insert(lobby_id.clone(), lobby_state);

    Ok(Json(json!({
        "message": "Lobby created successfully!",
        "lobby_id": lobby_id,
        "player_id": player_id
    })))
}

pub async fn join_lobby(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(request): Json<JoinLobbyRequest>,
) -> impl IntoResponse {
    let lobbies = app_state.lobbies.lock().unwrap();

    if let Some(lobby) = lobbies.get(&lobby_id) {
        // Generate a unique player ID
        let player_id: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();

        // Add player to the lobby
        lobby.add_player(player_id.clone(), request.player_name);

        Json(json!({
            "message": "Joined lobby successfully!",
            "lobby_id": lobby_id,
            "player_id": player_id
        }))
    } else {
        Json(json!({
            "error": "Lobby not found"
        }))
    }
}

pub async fn get_player_info(
    State(app_state): State<Arc<AppState>>,
    Path((lobby_id, player_id)): Path<(String, String)>,
) -> Result<Json<PlayerInfo>, Json<serde_json::Value>> {
    let lobbies = app_state.lobbies.lock().unwrap();

    if let Some(lobby) = lobbies.get(&lobby_id) {
        let name = lobby
            .get_player_name(&player_id)
            .unwrap_or_else(|| "Unknown".to_string());
        let score = lobby.get_player_score(&player_id);

        Ok(Json(PlayerInfo { name, score }))
    } else {
        Err(Json(json!({
            "error": "Lobby not found"
        })))
    }
}

pub async fn get_lobby_players(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<serde_json::Value>, Json<serde_json::Value>> {
    let lobbies = app_state.lobbies.lock().unwrap();

    if let Some(lobby) = lobbies.get(&lobby_id) {
        let players = lobby.get_all_players();
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
    } else {
        Err(Json(json!({
            "error": "Lobby not found"
        })))
    }
}

pub async fn get_kanji(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<KanjiPrompt>, Json<serde_json::Value>> {
    let lobbies = app_state.lobbies.lock().unwrap();
    if let Some(lobby) = lobbies.get(&lobby_id) {
        // Try to get the current kanji first
        let kanji = match lobby.get_current_kanji() {
            Some(kanji) => kanji,
            None => {
                // Generate a new kanji if none exists
                lobby.generate_random_kanji()
            }
        };
        Ok(Json(KanjiPrompt { kanji }))
    } else {
        Err(Json(json!({
            "error": "Lobby not found"
        })))
    }
}

pub async fn generate_new_kanji(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<KanjiPrompt>, Json<serde_json::Value>> {
    let lobbies = app_state.lobbies.lock().unwrap();
    if let Some(lobby) = lobbies.get(&lobby_id) {
        // Always generate a new kanji
        let kanji = lobby.generate_random_kanji();
        println!("Generated new Kanji: {} for lobby {}", &kanji, &lobby_id);
        Ok(Json(KanjiPrompt { kanji }))
    } else {
        Err(Json(json!({
            "error": "Lobby not found"
        })))
    }
}

pub async fn check_word(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
    Json(input): Json<UserInput>,
) -> Result<Json<CheckWordResponse>, Json<serde_json::Value>> {
    let lobbies = app_state.lobbies.lock().unwrap();
    if let Some(lobby) = lobbies.get(&lobby_id) {
        let word_list = &lobby.word_list;
        let input_word = input.word.trim();
        let input_kanji = input.kanji.trim();
        let player_id = input.player_id;

        let good_kanji = input_word.contains(input_kanji);
        let good_word = word_list.contains(&input_word.to_string());

        let message = if good_kanji && good_word {
            // Update the specific player's score
            let _ = lobby.increment_player_score(&player_id);
            "Good guess!".to_string()
        } else if good_kanji {
            "Bad Guess: Correct kanji, but not a valid word.".to_string()
        } else if good_word {
            "Bad Guess: Valid word, but does not contain the correct kanji.".to_string()
        } else {
            "Bad guess: Incorrect kanji and not a valid word.".to_string()
        };

        // Get the current score for this player
        let score = lobby.get_player_score(&player_id);

        Ok(Json(CheckWordResponse {
            message,
            score,
            error: None,
        }))
    } else {
        Err(Json(json!({
            "error": "Lobby not found"
        })))
    }
}
