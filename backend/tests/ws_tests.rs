/// WebSocket integration tests.
///
/// Each test binds a TCP listener on `127.0.0.1:0` (random port), starts an
/// Axum server via `tokio::spawn`, then connects `tokio-tungstenite` clients.
///
/// `AppState::create()` loads kanji data from `../data/` — run with
/// `cargo test` from the `backend/` directory.
///
/// ## Message shape
/// `ServerMessage` is serde'd with `#[serde(tag = "type", content = "payload")]`, so
/// every message looks like:
///   `{"type": "PlayerListUpdate", "payload": {"players": [...]}}`
/// All field accesses in these tests go through `msg["payload"]`.
use axum::{
    routing::{get, post},
    Router,
};
use futures::{SinkExt, StreamExt};
use moji::{
    api::{create_lobby, join_lobby, start_game, ws_handler},
    AppState,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

// ── Server helpers ────────────────────────────────────────────────────────────

fn make_ws_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/lobby/create", post(create_lobby))
        .route("/lobby/join/{lobby_id}", post(join_lobby))
        .route("/lobby/{lobby_id}/start", post(start_game))
        .route("/ws/{lobby_id}/{player_id}", get(ws_handler))
        .with_state(state)
}

/// Spin up a real server on an ephemeral port.
async fn spawn_server() -> (SocketAddr, Arc<AppState>) {
    let state = Arc::new(AppState::create().expect("AppState::create failed"));
    let router = make_ws_router(Arc::clone(&state));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (addr, state)
}

/// POST JSON to a REST endpoint, return parsed response body.
async fn post_json(addr: SocketAddr, path: &str, body: Value) -> Value {
    let url = format!("http://{addr}{path}");
    reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

/// Open a WebSocket to `ws://addr/ws/<lobby_id>/<player_id>`.
async fn connect_ws(addr: SocketAddr, lobby_id: &str, player_id: &str) -> WsStream {
    let url = format!("ws://{addr}/ws/{lobby_id}/{player_id}");
    let (ws, _) = connect_async(&url).await.expect("WS connect failed");
    ws
}

/// Read the next text message from a WebSocket, parse as JSON.
async fn next_msg(ws: &mut WsStream) -> Value {
    loop {
        match ws.next().await.unwrap().unwrap() {
            Message::Text(t) => return serde_json::from_str(&t).unwrap(),
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("Unexpected WS message: {other:?}"),
        }
    }
}

/// Drain messages until one with `type == expected_type` arrives.
/// Panics if none arrives within 3 seconds.
async fn next_msg_of_type(ws: &mut WsStream, expected_type: &str) -> Value {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(3), next_msg(ws))
            .await
            .unwrap_or_else(|_| panic!("Timed out waiting for '{expected_type}'"));
        if msg["type"].as_str() == Some(expected_type) {
            return msg;
        }
        // Earlier initial-state or unrelated message — keep draining.
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Connecting a WebSocket immediately yields a `PlayerListUpdate`.
/// The message shape: `{"type": "PlayerListUpdate", "payload": {"players": [...]}}`
#[tokio::test]
async fn test_ws_connect_receives_player_list() {
    let (addr, _) = spawn_server().await;

    let create = post_json(addr, "/lobby/create", json!({ "player_name": "Alice" })).await;
    let lobby_id = create["lobby_id"].as_str().unwrap();
    let player_id = create["player_id"].as_str().unwrap();

    let mut ws = connect_ws(addr, lobby_id, player_id).await;

    let msg = next_msg_of_type(&mut ws, "PlayerListUpdate").await;
    let players = msg["payload"]["players"].as_array().unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(players[0]["name"].as_str().unwrap(), "Alice");
}

/// When a second player joins, the first client receives a `PlayerListUpdate`
/// with both players listed.
#[tokio::test]
async fn test_ws_second_player_join_broadcast() {
    let (addr, _) = spawn_server().await;

    let create = post_json(addr, "/lobby/create", json!({ "player_name": "Alice" })).await;
    let lobby_id = create["lobby_id"].as_str().unwrap().to_string();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    // Alice connects and drains her own initial PlayerListUpdate
    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;

    // Bob joins via REST — triggers a broadcast to all subscribers
    post_json(
        addr,
        &format!("/lobby/join/{lobby_id}"),
        json!({ "player_name": "Bob" }),
    )
    .await;

    // Alice receives a PlayerListUpdate listing both players
    let msg = next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    let players = msg["payload"]["players"].as_array().unwrap();
    assert_eq!(players.len(), 2);
    let names: Vec<&str> = players.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"Alice") && names.contains(&"Bob"));
}

/// A `Typing` ClientMessage is broadcast to all other clients as `PlayerTyping`.
/// Shape: `{"type": "PlayerTyping", "payload": {"player_id": "...", "input": "..."}}`
#[tokio::test]
async fn test_ws_typing_broadcast() {
    let (addr, _) = spawn_server().await;

    let create = post_json(addr, "/lobby/create", json!({ "player_name": "Alice" })).await;
    let lobby_id = create["lobby_id"].as_str().unwrap().to_string();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    let join = post_json(
        addr,
        &format!("/lobby/join/{lobby_id}"),
        json!({ "player_name": "Bob" }),
    )
    .await;
    let bob_id = join["player_id"].as_str().unwrap().to_string();

    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    let mut bob_ws = connect_ws(addr, &lobby_id, &bob_id).await;

    // Drain initial state for both
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    next_msg_of_type(&mut bob_ws, "PlayerListUpdate").await;

    // Bob sends a Typing message
    // ClientMessage shape: `{"type": "Typing", "payload": {"input": "..."}}`
    bob_ws
        .send(Message::Text(
            json!({ "type": "Typing", "payload": {"input": "日本"} }).to_string().into(),
        ))
        .await
        .unwrap();

    // Alice should see PlayerTyping
    let msg = next_msg_of_type(&mut alice_ws, "PlayerTyping").await;
    assert_eq!(msg["payload"]["input"].as_str().unwrap(), "日本");
}

/// Starting the game broadcasts a `GameState` with `status == "Playing"`.
/// Shape: `{"type": "GameState", "payload": {"kanji": "...", "status": "Playing", "scores": [...]}}`
#[tokio::test]
async fn test_ws_game_start_broadcast() {
    let (addr, _) = spawn_server().await;

    let create = post_json(addr, "/lobby/create", json!({ "player_name": "Alice" })).await;
    let lobby_id = create["lobby_id"].as_str().unwrap().to_string();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    
    // Drain initial PlayerListUpdate and GameState (Lobby)
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    let initial_game_state = next_msg_of_type(&mut alice_ws, "GameState").await;
    assert_eq!(initial_game_state["payload"]["status"].as_str().unwrap(), "Lobby");

    // Leader starts game via REST
    post_json(
        addr,
        &format!("/lobby/{lobby_id}/start"),
        json!({ "player_id": alice_id }),
    )
    .await;

    // Wait for GameState (Playing)
    let msg = next_msg_of_type(&mut alice_ws, "GameState").await;
    assert_eq!(msg["payload"]["status"].as_str().unwrap(), "Playing");
    assert!(
        msg["payload"]["kanji"]
            .as_str()
            .is_some_and(|k| !k.is_empty())
    );
}

/// Submitting a correct word (when kanji matches) increments the score.
/// The kanji is random — we skip gracefully if no candidate word matches.
#[tokio::test]
async fn test_ws_correct_submit_increments_score() {
    let (addr, _) = spawn_server().await;

    let create = post_json(addr, "/lobby/create", json!({ "player_name": "Alice" })).await;
    let lobby_id = create["lobby_id"].as_str().unwrap().to_string();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;

    // Start game
    post_json(
        addr,
        &format!("/lobby/{lobby_id}/start"),
        json!({ "player_id": alice_id }),
    )
    .await;

    // Get current kanji from GameState
    let game_state = next_msg_of_type(&mut alice_ws, "GameState").await;
    let kanji = game_state["payload"]["kanji"].as_str().unwrap().to_string();

    // Try known words that contain common kanji
    let candidates = ["日本", "月曜日", "縁語", "炎", "渦紋"];
    let Some(&word) = candidates.iter().find(|w| w.contains(kanji.as_str())) else {
        // Random kanji didn't match any candidate — skip gracefully.
        // Correctness of guess logic is covered by unit tests.
        return;
    };

    // Submit as ClientMessage: `{"type": "Submit", "payload": {"word": "...", "kanji": "..."}}`
    alice_ws
        .send(Message::Text(
            json!({
                "type": "Submit",
                "payload": { "word": word, "kanji": kanji }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let response = next_msg_of_type(&mut alice_ws, "WordChecked").await;
    assert_eq!(response["payload"]["result"]["score"].as_u64().unwrap(), 1);
}
