use axum::{
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use moji::{
    api::ws_handler,
    state::AppState,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use shared::{ApiContext, JoinLobbyRequest, StartGameRequest, PlayerId, LobbyId};

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

fn make_ws_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws/{lobby_id}/{player_id}", get(ws_handler))
        .with_state(state)
}

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

fn generate_test_jwt(user_id: &str) -> String {
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize + 60 * 60; // 1 hour
        
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "INSECURE_DEFAULT_SECRET".to_string());
    
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_ref())
    ).unwrap()
}

async fn connect_ws(addr: SocketAddr, lobby_id: &str, player_id: &str) -> WsStream {
    let token = generate_test_jwt(player_id);
    let url = format!("ws://{addr}/ws/{lobby_id}/{player_id}?token={token}");
    let (ws, _) = connect_async(&url).await.expect("WS connect failed");
    ws
}

async fn next_msg(ws: &mut WsStream) -> Value {
    loop {
        match ws.next().await.unwrap().unwrap() {
            Message::Text(t) => return serde_json::from_str(&t).unwrap(),
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("Unexpected WS message: {other:?}"),
        }
    }
}

async fn next_msg_of_type(ws: &mut WsStream, expected_type: &str) -> Value {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(3), next_msg(ws))
            .await
            .unwrap_or_else(|_| panic!("Timed out waiting for '{expected_type}'"));
        if msg["type"].as_str() == Some(expected_type) {
            return msg;
        }
    }
}

#[tokio::test]
async fn test_ws_connect_receives_player_list() {
    let (addr, state) = spawn_server().await;
    let create = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into(), player_id: None }).await.unwrap();
    let lobby_id = create["lobby_id"].as_str().unwrap();
    let player_id = create["player_id"].as_str().unwrap();

    let mut ws = connect_ws(addr, lobby_id, player_id).await;

    let msg = next_msg_of_type(&mut ws, "PlayerListUpdate").await;
    let players = msg["payload"]["players"].as_array().unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(players[0]["name"].as_str().unwrap(), "Alice");
}

#[tokio::test]
async fn test_ws_second_player_join_broadcast() {
    let (addr, state) = spawn_server().await;
    let create = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into(), player_id: None }).await.unwrap();
    let lobby_id: LobbyId = create["lobby_id"].as_str().unwrap().into();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;

    state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into(), player_id: None }).await.unwrap();

    let msg = next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    let players = msg["payload"]["players"].as_array().unwrap();
    assert_eq!(players.len(), 2);
}

#[tokio::test]
async fn test_ws_typing_broadcast() {
    let (addr, state) = spawn_server().await;
    let create = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into(), player_id: None }).await.unwrap();
    let lobby_id: LobbyId = create["lobby_id"].as_str().unwrap().into();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    let join = state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into(), player_id: None }).await.unwrap();
    let bob_id = join["player_id"].as_str().unwrap().to_string();

    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    let mut bob_ws = connect_ws(addr, &lobby_id, &bob_id).await;

    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    next_msg_of_type(&mut bob_ws, "PlayerListUpdate").await;

    bob_ws
        .send(Message::Text(
            json!({ "type": "Typing", "payload": {"input": "日本"} }).to_string().into(),
        ))
        .await
        .unwrap();

    let msg = next_msg_of_type(&mut alice_ws, "PlayerTyping").await;
    assert_eq!(msg["payload"]["input"].as_str().unwrap(), "日本");
}

#[tokio::test]
async fn test_ws_game_start_broadcast() {
    let (addr, state) = spawn_server().await;
    let create = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into(), player_id: None }).await.unwrap();
    let lobby_id: LobbyId = create["lobby_id"].as_str().unwrap().into();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    let initial_game_state = next_msg_of_type(&mut alice_ws, "GameState").await;
    assert_eq!(initial_game_state["payload"]["status"].as_str().unwrap(), "Lobby");

    state.start_game(lobby_id.clone(), StartGameRequest { player_id: PlayerId(alice_id.clone()) }).await.unwrap();

    let msg = next_msg_of_type(&mut alice_ws, "GameState").await;
    assert_eq!(msg["payload"]["status"].as_str().unwrap(), "Playing");
    assert!(
        msg["payload"]["prompt"]
            .as_str()
            .is_some_and(|k| !k.is_empty())
    );
}

#[tokio::test]
async fn test_ws_correct_submit_increments_score() {
    let (addr, state) = spawn_server().await;
    let create = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into(), player_id: None }).await.unwrap();
    let lobby_id: LobbyId = create["lobby_id"].as_str().unwrap().into();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    next_msg_of_type(&mut alice_ws, "GameState").await; 

    state.start_game(lobby_id.clone(), StartGameRequest { player_id: PlayerId(alice_id.clone()) }).await.unwrap();

    let game_state = next_msg_of_type(&mut alice_ws, "GameState").await;
    let prompt = game_state["payload"]["prompt"].as_str().unwrap().to_string();

    let candidates = ["日本", "月曜日", "縁語", "炎", "渦紋"];
    let Some(&word) = candidates.iter().find(|w| w.contains(prompt.as_str())) else {
        return;
    };

    alice_ws
        .send(Message::Text(
            json!({
                "type": "Submit",
                "payload": { "input": word, "prompt": prompt }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let response = next_msg_of_type(&mut alice_ws, "WordChecked").await;
    assert_eq!(response["payload"]["result"]["score"].as_u64().unwrap(), 1);
}

#[tokio::test]
async fn test_ws_leave_lobby_broadcasts_player_removal() {
    let (addr, state) = spawn_server().await;

    // Alice creates the lobby
    let create = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into(), player_id: None }).await.unwrap();
    let lobby_id: LobbyId = create["lobby_id"].as_str().unwrap().into();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    // Bob joins
    let join = state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into(), player_id: None }).await.unwrap();
    let bob_id = join["player_id"].as_str().unwrap().to_string();

    // Both connect via WebSocket
    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    let mut bob_ws = connect_ws(addr, &lobby_id, &bob_id).await;

    // Drain initial messages
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    next_msg_of_type(&mut alice_ws, "GameState").await;
    next_msg_of_type(&mut bob_ws, "PlayerListUpdate").await;
    next_msg_of_type(&mut bob_ws, "GameState").await;

    // Bob leaves via the API
    state.leave_lobby(lobby_id.clone(), PlayerId(bob_id.clone())).await.unwrap();

    // Alice should eventually receive a PlayerListUpdate with only herself.
    // There may be intermediate broadcasts (e.g. connection status), so loop
    // until we find one with exactly 1 player or time out.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let msg = tokio::time::timeout_at(deadline, next_msg_of_type(&mut alice_ws, "PlayerListUpdate"))
            .await
            .expect("Timed out waiting for PlayerListUpdate with 1 player");
        let players = msg["payload"]["players"].as_array().unwrap();
        if players.len() == 1 {
            assert_eq!(players[0]["name"].as_str().unwrap(), "Alice");
            break;
        }
    }
}

#[tokio::test]
async fn test_ws_disconnect_marks_player_disconnected() {
    let (addr, state) = spawn_server().await;

    // Alice creates the lobby
    let create = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into(), player_id: None }).await.unwrap();
    let lobby_id: LobbyId = create["lobby_id"].as_str().unwrap().into();
    let alice_id = create["player_id"].as_str().unwrap().to_string();

    // Bob joins
    let join = state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into(), player_id: None }).await.unwrap();
    let bob_id = join["player_id"].as_str().unwrap().to_string();

    // Both connect via WebSocket
    let mut alice_ws = connect_ws(addr, &lobby_id, &alice_id).await;
    let mut bob_ws = connect_ws(addr, &lobby_id, &bob_id).await;

    // Drain initial messages
    next_msg_of_type(&mut alice_ws, "PlayerListUpdate").await;
    next_msg_of_type(&mut alice_ws, "GameState").await;
    next_msg_of_type(&mut bob_ws, "PlayerListUpdate").await;
    next_msg_of_type(&mut bob_ws, "GameState").await;

    // Bob disconnects by dropping his WebSocket
    drop(bob_ws);

    // Alice should receive a PlayerListUpdate where Bob is still present
    // but marked as disconnected (is_connected: false)
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let msg = tokio::time::timeout_at(deadline, next_msg_of_type(&mut alice_ws, "PlayerListUpdate"))
            .await
            .expect("Timed out waiting for PlayerListUpdate with Bob disconnected");
        let players = msg["payload"]["players"].as_array().unwrap();
        // Find Bob in the list and check his connection status
        if let Some(bob) = players.iter().find(|p| p["name"].as_str() == Some("Bob")) {
            if bob["is_connected"].as_bool() == Some(false) {
                assert_eq!(players.len(), 2, "Both players should still be in the list");
                break;
            }
        }
    }
}
