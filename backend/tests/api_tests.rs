/// HTTP integration tests for the moji backend REST API.
///
/// These tests spin up an in-process Axum router via `axum_test::TestServer`
/// (no real TCP socket) and exercise every REST endpoint.
///
/// `AppState::create()` loads kanji data from `../data/` relative to the
/// backend build dir — run tests from the `backend/` directory (default with
/// `cargo test`).
use axum::{
    routing::{get, post},
    Router,
};
use axum_test::TestServer;
use moji::{
    api::{
        create_lobby, generate_new_kanji, get_kanji, get_lobby_info, get_lobby_players,
        get_player_info, join_lobby, leave_lobby, reset_lobby, start_game, update_lobby_settings,
    },
    AppState,
};
use serde_json::{json, Value};
use std::sync::Arc;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_router() -> Router {
    let state = Arc::new(AppState::create().expect("Failed to create AppState"));
    Router::new()
        .route("/lobby/create", post(create_lobby))
        .route("/lobby/join/{lobby_id}", post(join_lobby))
        .route("/player/{lobby_id}/{player_id}", get(get_player_info))
        .route("/lobby/players/{lobby_id}", get(get_lobby_players))
        .route("/lobby/{lobby_id}/leave", post(leave_lobby))
        .route("/kanji/{lobby_id}", get(get_kanji))
        .route("/new_kanji/{lobby_id}", post(generate_new_kanji))
        .route("/lobby/{lobby_id}/info", get(get_lobby_info))
        .route("/lobby/{lobby_id}/settings", post(update_lobby_settings))
        .route("/lobby/{lobby_id}/start", post(start_game))
        .route("/lobby/{lobby_id}/reset", post(reset_lobby))
        .with_state(state)
}

/// Create a lobby and return `(server, lobby_id, player_id)`.
async fn create_lobby_helper(server: &TestServer) -> (String, String) {
    let res = server
        .post("/lobby/create")
        .json(&json!({ "player_name": "Alice" }))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    let lobby_id = body["lobby_id"].as_str().unwrap().to_string();
    let player_id = body["player_id"].as_str().unwrap().to_string();
    (lobby_id, player_id)
}

// ── Lobby creation ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_lobby_returns_ids() {
    let server = TestServer::new(make_router()).unwrap();
    let res = server
        .post("/lobby/create")
        .json(&json!({ "player_name": "Alice" }))
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    assert!(body["lobby_id"].as_str().is_some_and(|id| id.len() == 6));
    assert!(body["player_id"].as_str().is_some_and(|id| !id.is_empty()));
}

// ── Joining ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_join_lobby_gives_unique_player_id() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, creator_id) = create_lobby_helper(&server).await;

    let res = server
        .post(&format!("/lobby/join/{lobby_id}"))
        .json(&json!({ "player_name": "Bob" }))
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    let joiner_id = body["player_id"].as_str().unwrap();
    assert_ne!(joiner_id, creator_id.as_str());
}

#[tokio::test]
async fn test_join_nonexistent_lobby_is_404() {
    let server = TestServer::new(make_router()).unwrap();
    let res = server
        .post("/lobby/join/NOPE00")
        .json(&json!({ "player_name": "Nobody" }))
        .await;
    res.assert_status_not_found();
}

// ── Lobby info & players ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_lobby_info_contains_creator() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, _) = create_lobby_helper(&server).await;

    let res = server.get(&format!("/lobby/{lobby_id}/info")).await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["lobby_id"].as_str().unwrap(), lobby_id);
    assert_eq!(body["players"].as_array().unwrap().len(), 1);
    assert_eq!(body["players"][0]["name"].as_str().unwrap(), "Alice");
}

#[tokio::test]
async fn test_get_lobby_players_returns_list() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, _) = create_lobby_helper(&server).await;

    server
        .post(&format!("/lobby/join/{lobby_id}"))
        .json(&json!({ "player_name": "Bob" }))
        .await;

    let res = server.get(&format!("/lobby/players/{lobby_id}")).await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["players"].as_array().unwrap().len(), 2);
}

// ── Player info ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_player_info_found() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, player_id) = create_lobby_helper(&server).await;

    let res = server
        .get(&format!("/player/{lobby_id}/{player_id}"))
        .await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["name"].as_str().unwrap(), "Alice");
    assert_eq!(body["score"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn test_get_player_info_not_found_is_404() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, _) = create_lobby_helper(&server).await;

    let res = server
        .get(&format!("/player/{lobby_id}/nonexistent_player"))
        .await;
    res.assert_status_not_found();
}

// ── Leaving ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_leave_lobby_removes_player() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, _) = create_lobby_helper(&server).await;

    let join_res = server
        .post(&format!("/lobby/join/{lobby_id}"))
        .json(&json!({ "player_name": "Bob" }))
        .await;
    let joiner_id = join_res.json::<Value>()["player_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Leave
    let leave_res = server
        .post(&format!("/lobby/{lobby_id}/leave"))
        .json(&json!({ "player_id": joiner_id }))
        .await;
    leave_res.assert_status_ok();

    // Confirm player is gone
    let info: Value = server
        .get(&format!("/lobby/{lobby_id}/info"))
        .await
        .json();
    assert_eq!(info["players"].as_array().unwrap().len(), 1);
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_settings_leader_succeeds() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, player_id) = create_lobby_helper(&server).await;

    let res = server
        .post(&format!("/lobby/{lobby_id}/settings"))
        .json(&json!({
            "player_id": player_id,
            "settings": {
                "difficulty_levels": ["N5", "N4"],
                "max_players": 6,
                "weighted": false,
                "mode": "Deathmatch",
                "target_score": 5,
                "time_limit_seconds": null,
                "initial_lives": null,
                "duel_allow_kanji_reuse": false
            }
        }))
        .await;
    res.assert_status_ok();
}

#[tokio::test]
async fn test_update_settings_non_leader_fails() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, _) = create_lobby_helper(&server).await;

    let join_res: Value = server
        .post(&format!("/lobby/join/{lobby_id}"))
        .json(&json!({ "player_name": "Bob" }))
        .await
        .json();
    let bob_id = join_res["player_id"].as_str().unwrap().to_string();

    let res = server
        .post(&format!("/lobby/{lobby_id}/settings"))
        .json(&json!({
            "player_id": bob_id,
            "settings": {
                "difficulty_levels": ["N5"],
                "max_players": 4,
                "weighted": false,
                "mode": "Deathmatch",
                "target_score": 5,
                "time_limit_seconds": null,
                "initial_lives": null,
                "duel_allow_kanji_reuse": false
            }
        }))
        .await;
    // Non-leader should get a 4xx
    assert!(res.status_code().is_client_error());
}

// ── Starting the game ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_start_game_leader_succeeds() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, player_id) = create_lobby_helper(&server).await;

    let res = server
        .post(&format!("/lobby/{lobby_id}/start"))
        .json(&json!({ "player_id": player_id }))
        .await;
    res.assert_status_ok();

    let info: Value = server.get(&format!("/lobby/{lobby_id}/info")).await.json();
    assert_eq!(info["status"].as_str().unwrap(), "Playing");
}

#[tokio::test]
async fn test_start_game_non_leader_fails() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, _) = create_lobby_helper(&server).await;

    let join_body: Value = server
        .post(&format!("/lobby/join/{lobby_id}"))
        .json(&json!({ "player_name": "Bob" }))
        .await
        .json();
    let bob_id = join_body["player_id"].as_str().unwrap();

    let res = server
        .post(&format!("/lobby/{lobby_id}/start"))
        .json(&json!({ "player_id": bob_id }))
        .await;
    assert!(res.status_code().is_client_error());
}

// ── Reset ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_reset_lobby_returns_to_lobby_status() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, player_id) = create_lobby_helper(&server).await;

    server
        .post(&format!("/lobby/{lobby_id}/start"))
        .json(&json!({ "player_id": player_id }))
        .await;

    server
        .post(&format!("/lobby/{lobby_id}/reset"))
        .json(&json!({ "player_id": player_id }))
        .await
        .assert_status_ok();

    let info: Value = server.get(&format!("/lobby/{lobby_id}/info")).await.json();
    assert_eq!(info["status"].as_str().unwrap(), "Lobby");
}

// ── Kanji ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_kanji_after_game_start() {
    let server = TestServer::new(make_router()).unwrap();
    let (lobby_id, player_id) = create_lobby_helper(&server).await;

    server
        .post(&format!("/lobby/{lobby_id}/start"))
        .json(&json!({ "player_id": player_id }))
        .await;

    let res = server.get(&format!("/kanji/{lobby_id}")).await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert!(
        body["kanji"].as_str().is_some_and(|k| !k.is_empty()),
        "Expected a non-empty kanji string"
    );
}
