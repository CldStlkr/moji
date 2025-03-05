use crate::{
    models::basic::{KanjiPrompt, UserInput},
    AppState, LobbyState,
};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::json;
use std::sync::Arc;

pub async fn create_lobby(
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let mut lobbies = app_state.lobbies.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to acquire lock on lobbies",
        )
    })?;

    // Generate random 6-character alphanumeric lobby ID
    let lobby_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    let lobby_state = Arc::new(
        LobbyState::create()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create lobby"))?,
    );

    lobbies.insert(lobby_id.clone(), lobby_state);

    Ok(Json(json!({
        "message": "Lobby created successfully!",
        "lobby_id": lobby_id
    })))
}

pub async fn join_lobby(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> impl IntoResponse {
    let lobbies = app_state.lobbies.lock().unwrap();

    if let Some(_) = lobbies.get(&lobby_id) {
        Json(json!({
            "message": "Joined lobby sucessfully!",
            "lobby_id": lobby_id
        }))
    } else {
        Json(json!({
            "error": "json not found..."
        }))
    }
}

pub async fn get_kanji(
    State(app_state): State<Arc<AppState>>,
    Path(lobby_id): Path<String>,
) -> Result<Json<KanjiPrompt>, Json<serde_json::Value>> {
    let lobbies = app_state.lobbies.lock().unwrap();
    if let Some(lobby) = lobbies.get(&lobby_id) {
        let mut rng = rand::thread_rng();
        let random_index = rng.gen_range(0..lobby.kanji_list.len());
        let kanji = &lobby.kanji_list[random_index];
        println!("Serving Kanji: {} for lobby {}", &kanji, &lobby_id);
        Ok(Json(KanjiPrompt {
            kanji: kanji.clone(),
        }))
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
) -> Result<Json<serde_json::Value>, Json<serde_json::Value>> {
    let lobbies = app_state.lobbies.lock().unwrap();

    if let Some(lobby) = lobbies.get(&lobby_id) {
        let word_list = &lobby.word_list;
        let input_word = input.word.trim();
        let input_kanji = input.kanji.trim();

        let good_kanji = input_word.contains(input_kanji);
        let good_word = word_list.contains(&input_word.to_string());

        let mut user_score = lobby.user_score.lock().unwrap();

        let message = if good_kanji && good_word {
            user_score.score += 1;
            "Good guess!".to_string()
        } else if good_kanji {
            "Bad Guess: Correct kanji, but not a valid word.".to_string()
        } else if good_word {
            "Bad Guess: Valid word, but does not contain the correct kanji.".to_string()
        } else {
            "Bad guess: Incorrect kanji and not a valid word.".to_string()
        };

        Ok(Json(json!({
            "message": message,
            "score": user_score.score
        })))
    } else {
        Err(Json(json!({
            "error": "Lobby not found"
        })))
    }
}
