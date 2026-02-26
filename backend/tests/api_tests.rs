use moji::state::AppState;
use shared::{ApiContext, JoinLobbyRequest, StartGameRequest, UpdateSettingsRequest, LobbyId};
use std::sync::Arc;

// Helper to create state
async fn get_state() -> Arc<AppState> {
    Arc::new(AppState::create().expect("Failed to create AppState"))
}

#[tokio::test]
async fn test_create_lobby_returns_ids() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    assert!(res["lobby_id"].as_str().is_some_and(|id| id.len() == 6));
    assert!(res["player_id"].as_str().is_some_and(|id| !id.is_empty()));
}

#[tokio::test]
async fn test_join_lobby_gives_unique_player_id() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();
    let creator_id = res["player_id"].as_str().unwrap().to_string();

    let res2 = state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into() }).await.unwrap();
    let joiner_id = res2["player_id"].as_str().unwrap();
    assert_ne!(joiner_id, creator_id.as_str());
}

#[tokio::test]
async fn test_join_nonexistent_lobby_is_error() {
    let state = get_state().await;
    let res = state.join_lobby("NOPE00".into(), JoinLobbyRequest { player_name: "Nobody".into() }).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_get_lobby_info_contains_creator() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();

    let info = state.get_lobby_info(lobby_id.clone()).await.unwrap();
    assert_eq!(info.lobby_id, lobby_id);
    assert_eq!(info.players.len(), 1);
    assert_eq!(info.players[0].name, "Alice");
}

#[tokio::test]
async fn test_get_lobby_players_returns_list() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();

    state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into() }).await.unwrap();

    let players_res = state.get_lobby_players(lobby_id).await.unwrap();
    assert_eq!(players_res["players"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_player_info_found() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();
    let player_id = res["player_id"].as_str().unwrap().to_string();

    let player = state.get_player_info(lobby_id, shared::PlayerId(player_id.clone())).await.unwrap();
    assert_eq!(player.name, "Alice");
    assert_eq!(player.score, 0);
}

#[tokio::test]
async fn test_get_player_info_not_found_is_error() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();

    let res = state.get_player_info(lobby_id.clone(), shared::PlayerId("nonexistent".into())).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_leave_lobby_removes_player() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();

    let join_res = state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into() }).await.unwrap();
    let joiner_id = join_res["player_id"].as_str().unwrap().to_string();

    state.leave_lobby(lobby_id.clone(), shared::PlayerId(joiner_id.clone())).await.unwrap();

    let info = state.get_lobby_info(lobby_id.clone()).await.unwrap();
    assert_eq!(info.players.len(), 1);
}

#[tokio::test]
async fn test_update_settings_leader_succeeds() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();
    let player_id = res["player_id"].as_str().unwrap().to_string();

    let settings = shared::GameSettings {
        mode: shared::GameMode::Deathmatch,
        ..Default::default()
    };

    state.update_lobby_settings(lobby_id.clone(), UpdateSettingsRequest {
        player_id: shared::PlayerId(player_id.clone()),
        settings,
    }).await.unwrap();
}

#[tokio::test]
async fn test_update_settings_non_leader_fails() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();

    let join_res = state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into() }).await.unwrap();
    let joiner_id = join_res["player_id"].as_str().unwrap().to_string();

    let res = state.update_lobby_settings(lobby_id.clone(), UpdateSettingsRequest {
        player_id: shared::PlayerId(joiner_id.clone()),
        settings: shared::GameSettings::default(),
    }).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_start_game_leader_succeeds() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();
    let player_id = res["player_id"].as_str().unwrap().to_string();

    state.start_game(lobby_id.clone(), StartGameRequest { player_id: shared::PlayerId(player_id.clone()) }).await.unwrap();

    let info = state.get_lobby_info(lobby_id.clone()).await.unwrap();
    assert_eq!(info.status, shared::GameStatus::Playing);
}

#[tokio::test]
async fn test_start_game_non_leader_fails() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();

    let join_res = state.join_lobby(lobby_id.clone(), JoinLobbyRequest { player_name: "Bob".into() }).await.unwrap();
    let bob_id = join_res["player_id"].as_str().unwrap().to_string();

    let res = state.start_game(lobby_id.clone(), StartGameRequest { player_id: shared::PlayerId(bob_id) }).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_reset_lobby_returns_to_lobby_status() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();
    let player_id = res["player_id"].as_str().unwrap().to_string();

    state.start_game(lobby_id.clone(), StartGameRequest { player_id: shared::PlayerId(player_id.clone()) }).await.unwrap();
    state.reset_lobby(lobby_id.clone(), shared::PlayerId(player_id.clone())).await.unwrap();

    let info = state.get_lobby_info(lobby_id.clone()).await.unwrap();
    assert_eq!(info.status, shared::GameStatus::Lobby);
}

#[tokio::test]
async fn test_get_kanji_after_game_start() {
    let state = get_state().await;
    let res = state.create_lobby(JoinLobbyRequest { player_name: "Alice".into() }).await.unwrap();
    let lobby_id: LobbyId = res["lobby_id"].as_str().unwrap().into();
    let player_id = res["player_id"].as_str().unwrap().to_string();

    state.start_game(lobby_id.clone(), StartGameRequest { player_id: shared::PlayerId(player_id.clone()) }).await.unwrap();

    let res = state.get_prompt(lobby_id.clone()).await.unwrap();
    assert!(!res.prompt.is_empty());
}
