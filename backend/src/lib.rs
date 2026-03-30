pub mod api;
pub mod data;
pub mod db;
pub mod error;
pub mod models;
pub mod types;
pub mod lobby;
pub mod state;
pub mod utils;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use data::Kanji;
    use std::sync::Arc;
    use error::AppError;
    use crate::{lobby::LobbyState, state::AppState};
    use shared::{ActivePrompt, LobbyId, PlayerId, GameStatus, GameSettings, ApiContext};
    use utils::generate_lobby_id;

    fn create_test_lobby() -> LobbyState {
        let test_kanji_list = Arc::new(vec![
            vec![
                Kanji { kanji: "日".to_string(), frequency: 0 },
                Kanji { kanji: "月".to_string(), frequency: 0 },
                Kanji { kanji: "屈".to_string(), frequency: 0 },
                Kanji { kanji: "理".to_string(), frequency: 0 },
                Kanji { kanji: "総".to_string(), frequency: 0 },
                Kanji { kanji: "辱".to_string(), frequency: 0 },
                Kanji { kanji: "酷".to_string(), frequency: 0 },
                Kanji { kanji: "関".to_string(), frequency: 0 },
                Kanji { kanji: "糸".to_string(), frequency: 0 },
                Kanji { kanji: "木".to_string(), frequency: 0 },
            ],
        ]);
        let test_dict_list = Arc::new(HashSet::from([
            "日本".to_string(),
            "弄り回す".to_string(),
            "月曜日".to_string(),
            "比律賓".to_string(),
            "哀歌".to_string(),
            "猥ら".to_string(),
            "育ち".to_string(),
            "縁語".to_string(),
            "炎".to_string(),
            "渦紋".to_string(),
        ]));
        let test_words_list = Arc::new(vec![
            {
                let mut map = HashMap::new();
                map.insert("日本".to_string(), vec!["にほん".to_string(), "にっぽん".to_string()]);
                map.insert("月曜日".to_string(), vec!["げつようび".to_string()]);
                map.insert("木曜日".to_string(), vec!["もくようび".to_string()]);
                map.insert("日記".to_string(), vec!["にっき".to_string()]);
                map.insert("理由".to_string(), vec!["りゆう".to_string()]);
                map
            },
        ]);

        LobbyState::new(test_kanji_list, test_words_list, test_dict_list, None, None)
    }

    #[test]
    fn test_generate_lobby_id() {
        let id = generate_lobby_id();
        assert_eq!(id.len(), 6);
        // Check that ID is alphanumeric
        assert!(id.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_increment_player_score() {
        let lobby_state = create_test_lobby();
        let player_id = PlayerId(String::from("test_player"));
        lobby_state
            .add_player(player_id.clone(), "Test Player".to_string())
            .unwrap();

        // Initial score should be 0
        assert_eq!(lobby_state.get_player_score(&player_id).unwrap(), 0);

        // After increment, should be 1
        assert_eq!(lobby_state.increment_player_score(&player_id).unwrap(), 1);
        assert_eq!(lobby_state.get_player_score(&player_id).unwrap(), 1);
    }

    #[test]
    fn test_get_current_prompt_text() {
        let lobby_state = create_test_lobby();

        // Initially should be None
        assert_eq!(lobby_state.get_current_prompt_text(), None);

        // Generate a kanji and verify it's set
        // NOTE: New generate_random_prompt requires start_game to populate active_indices or manually setting them
        // Manually set them for the test
        {
            lobby_state.active_level_indices.write(|indices| indices.push(0));
        }

        let kanji = lobby_state.generate_random_prompt(false).unwrap();
        assert_eq!(lobby_state.get_current_prompt_text(), Some(kanji));
    }

    #[test]
    fn test_generate_random_prompt() {
        let lobby_state = create_test_lobby();

        // Set active indices
        {
             lobby_state.active_level_indices.write(|indices| indices.push(0));
        }

        // Generate a kanji and verify it's from one of the lists
        let kanji = lobby_state.generate_random_prompt(false).unwrap();
        let kanji_exists = lobby_state.kanji_list
            .iter()
            .flatten()
            .any(|k| k.kanji == kanji);
        assert!(kanji_exists);

        // Generate another and ensure it's set as current
        let kanji2 = lobby_state.generate_random_prompt(false).unwrap();
        assert_eq!(lobby_state.get_current_prompt_text(), Some(kanji2));
    }

    #[test]
    fn test_get_all_players() {
        let lobby_state = create_test_lobby();

        // Initially empty
        assert!(lobby_state.get_all_players().is_empty());

        // Add players and verify they're returned
        lobby_state
            .add_player(PlayerId::from("player1"), "Alice".to_string())
            .unwrap();
        lobby_state
            .add_player(PlayerId::from("player2"), "Bob".to_string())
            .unwrap();

        let players = lobby_state.get_all_players();
        assert_eq!(players.len(), 2);

        // Option 1: Simple verification - check names exist
        let names: Vec<&String> = players.iter().map(|p| &p.name).collect();
        assert!(names.contains(&&"Alice".to_string()));
        assert!(names.contains(&&"Bob".to_string()));

        // Option 2: More thorough verification - find specific players
        let alice = players.iter().find(|p| p.id.0 == "player1");
        let bob = players.iter().find(|p| p.id.0 == "player2");

        assert!(alice.is_some());
        assert!(bob.is_some());

        // Option 3: Verify specific player details
        assert_eq!(alice.unwrap().name, "Alice");
        assert_eq!(bob.unwrap().name, "Bob");
        assert_eq!(alice.unwrap().score, 0);
        assert_eq!(bob.unwrap().score, 0);

        // Option 4: Verify order is maintained (first player added is first in Vec)
        assert_eq!(players[0].id, PlayerId(String::from("player1")));
        assert_eq!(players[0].name, "Alice");
        assert_eq!(players[1].id, PlayerId(String::from("player2")));
        assert_eq!(players[1].name, "Bob");
    }

    #[test]
    fn test_player_not_found_error() {
        let lobby_state = create_test_lobby();

        // Attempt to get score for non-existent player
        let result = lobby_state.get_player_score(&PlayerId(String::from("nonexistent")));
        assert!(result.is_err());

        // Verify error type
        match result {
            Err(AppError::PlayerNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected PlayerNotFound error"),
        }
    }

    #[test]
    fn test_get_lobby_not_found() {
        let app_state = Arc::new(AppState::create().expect("Failed to create AppState"));

        let result = app_state.get_lobby(&LobbyId(String::from("nonexistent")));
        assert!(result.is_err());

        // Verify error type
        match result {
            Err(AppError::LobbyNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected LobbyNotFound error"),
        }
    }
    #[test]
    fn test_lobby_workflow() {
        // Create app state
        let app_state = Arc::new(AppState::create().expect("Failed to create AppState"));

        // Create a lobby and add it to the state
        let lobby_id = generate_lobby_id();
        let lobby_state = Arc::new(create_test_lobby());

        {
             app_state.lobbies.write(|lobbies| {
                 lobbies.insert(lobby_id.clone(), lobby_state.clone());
             });
        }

        // Get the lobby and verify it exists
        let retrieved_lobby = app_state.get_lobby(&lobby_id).unwrap();

        // Add players to lobby
        retrieved_lobby
            .add_player(PlayerId::from("p1"), "Player 1".to_string())
            .unwrap();
        retrieved_lobby
            .add_player(PlayerId::from("p2"), "Player 2".to_string())
            .unwrap();

        // Manually start game or set indices to allow generation
        {
             retrieved_lobby.active_level_indices.write(|indices| indices.push(0));
        }

        // Generate kanji and check word
        let _kanji = retrieved_lobby.generate_random_prompt(false).unwrap();

        // Verify players and scores
        let players = retrieved_lobby.get_all_players();
        assert_eq!(players.len(), 2);
    }

    #[test]
    fn test_lobby_leader_functionality() {
        let lobby_state = create_test_lobby();

        // First player becomes leader
        let is_leader1 = lobby_state
            .add_player(PlayerId::from("player1"), "Alice".to_string())
            .unwrap();
        assert!(is_leader1);
        assert!(lobby_state.is_leader(&PlayerId::from("player1")));

        // Second player is not leader
        let is_leader2 = lobby_state
            .add_player(PlayerId::from("player2"), "Bob".to_string())
            .unwrap();
        assert!(!is_leader2);
        assert!(!lobby_state.is_leader(&PlayerId::from("player2")));
    }

    #[test]
    fn test_update_settings_leader_only() {
        let lobby_state = create_test_lobby();

        lobby_state
            .add_player(PlayerId::from("leader"), "Leader".to_string())
            .unwrap();
        lobby_state
            .add_player(PlayerId::from("player"), "Player".to_string())
            .unwrap();

        let new_settings = GameSettings {
            difficulty_levels: vec!["N5".to_string(), "N4".to_string()],
            time_limit_seconds: Some(60),
            max_players: 10,
            weighted: false,
            ..Default::default()
        };

        // Leader can update settings
        assert!(lobby_state
            .update_settings(&PlayerId::from("leader"), new_settings.clone())
            .is_ok());

        // Non-leader cannot update settings
        assert!(lobby_state
            .update_settings(&PlayerId::from("player"), new_settings)
            .is_err());
    }

    #[test]
    fn test_start_game_leader_only() {
        let lobby_state = create_test_lobby();

        lobby_state
            .add_player(PlayerId::from("leader"), "Leader".to_string())
            .unwrap();
        lobby_state
            .add_player(PlayerId::from("player"), "Player".to_string())
            .unwrap();

        // Leader can start game
        assert!(lobby_state.start_game(&PlayerId::from("leader")).is_ok());

        // Game status should change to Playing
        let status = lobby_state.game_status.read(|s| *s);
        assert_eq!(status, GameStatus::Playing);
    }

    // ── Player management ───────────────────────────────────────────────────

    #[test]
    fn test_add_player_empty_name_fails() {
        let lobby = create_test_lobby();
        assert!(lobby.add_player(PlayerId::from("p1"), "".to_string()).is_err());
    }

    #[test]
    fn test_add_player_whitespace_name_fails() {
        let lobby = create_test_lobby();
        assert!(lobby.add_player(PlayerId::from("p1"), "   ".to_string()).is_err());
    }

    #[test]
    fn test_remove_player() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("p1"), "Alice".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();

        assert!(lobby.remove_player(&PlayerId::from("p2")));
        let players = lobby.get_all_players();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "Alice");
    }

    #[test]
    fn test_remove_nonexistent_player_returns_false() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("p1"), "Alice".to_string()).unwrap();
        assert!(!lobby.remove_player(&PlayerId::from("ghost")));
    }

    #[test]
    fn test_remove_leader_transfers_leadership() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("leader"), "Leader".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();

        lobby.remove_player(&PlayerId::from("leader"));
        assert!(lobby.is_leader(&PlayerId::from("p2")));
    }

    // ── Game flow ───────────────────────────────────────────────────────────

    #[test]
    fn test_start_game_non_leader() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("leader"), "Leader".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();
        assert!(lobby.start_game(&PlayerId::from("p2")).is_err());
    }

    #[test]
    fn test_start_game_twice_fails() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("leader"), "Leader".to_string()).unwrap();
        assert!(lobby.start_game(&PlayerId::from("leader")).is_ok());
        assert!(lobby.start_game(&PlayerId::from("leader")).is_err());
    }

    #[test]
    fn test_reset_lobby_returns_to_lobby_status() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("leader"), "Leader".to_string()).unwrap();
        lobby.start_game(&PlayerId::from("leader")).unwrap();

        lobby.reset_lobby(&PlayerId::from("leader")).unwrap();
        assert_eq!(lobby.game_status.read(|s| *s), GameStatus::Lobby);
    }

    #[test]
    fn test_advance_turn_wraps() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("p1"), "Alice".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();
        lobby.turn_order.write(|o| { o.push(PlayerId::from("p1")); o.push(PlayerId::from("p2")); });

        assert_eq!(lobby.advance_turn().unwrap(), PlayerId::from("p2"));
        assert_eq!(lobby.advance_turn().unwrap(), PlayerId::from("p1")); // wraps
    }

    #[test]
    fn test_get_lobby_info_reflects_state() {
        let lobby = create_test_lobby();
        lobby.add_player(PlayerId::from("p1"), "Alice".to_string()).unwrap();
        lobby.add_player(PlayerId::from("p2"), "Bob".to_string()).unwrap();

        let info = lobby.get_lobby_info(&LobbyId::from("test-lobby"));
        assert_eq!(info.lobby_id, LobbyId(String::from("test-lobby")));
        assert_eq!(info.status, GameStatus::Lobby);
        assert_eq!(info.leader_id, PlayerId::from("p1"));
        assert_eq!(info.players.len(), 2);
    }

    // ── Guess processing ────────────────────────────────────────────────────

    /// Shared setup: start a single-player Deathmatch with target_score=3,
    /// current kanji set to "日", and game status = Playing.
    fn setup_deathmatch_playing() -> (LobbyState, PlayerId) {
        let lobby = create_test_lobby();
        let leader = PlayerId::from("leader");
        lobby.add_player(leader.clone(), "Leader".to_string()).unwrap();
        lobby.settings.write(|s| { s.target_score = Some(3); });
        lobby.active_level_indices.write(|i| i.push(0));
        lobby.current_prompt.write(|k| *k = Some(ActivePrompt::Kanji { character: "日".to_string() }));
        lobby.game_status.write(|s| *s = GameStatus::Playing);
        (lobby, leader)
    }

    #[test]
    fn test_process_guess_correct_increments_score() {
        let (lobby, leader) = setup_deathmatch_playing();
        lobby.process_guess(&leader, "日本").unwrap();
        assert_eq!(lobby.get_player_score(&leader).unwrap(), 1);
    }

    #[test]
    fn test_process_guess_wrong_word_no_score() {
        let (lobby, leader) = setup_deathmatch_playing();
        // "日xyz" contains "日" but is NOT in the dictionary
        lobby.process_guess(&leader, "日xyz").unwrap();
        assert_eq!(lobby.get_player_score(&leader).unwrap(), 0);
    }

    #[test]
    fn test_process_guess_wrong_kanji_no_score() {
        let (lobby, leader) = setup_deathmatch_playing();
        // "哀歌" is a valid dictionary word but does NOT contain the prompt kanji "日"
        lobby.process_guess(&leader, "哀歌").unwrap();
        assert_eq!(lobby.get_player_score(&leader).unwrap(), 0);
    }

    #[test]
    fn test_process_guess_while_not_playing_is_noop() {
        let lobby = create_test_lobby();
        let leader = PlayerId::from("leader");
        lobby.add_player(leader.clone(), "Leader".to_string()).unwrap();
        // Default status is Lobby — should be silently ignored
        assert!(lobby.process_guess(&leader, "日本").is_ok());
        assert_eq!(lobby.get_player_score(&leader).unwrap(), 0);
    }

    #[test]
    fn test_deathmatch_target_score_ends_game() {
        let (lobby, leader) = setup_deathmatch_playing();
        for _ in 0..3 {
            lobby.current_prompt.write(|k| *k = Some(ActivePrompt::Kanji { character: "日".to_string() }));
            let empty = lobby.active_level_indices.read(|i| i.is_empty());
            if empty { lobby.active_level_indices.write(|i| i.push(0)); }
            lobby.process_guess(&leader, "日本").unwrap();
        }
        assert_eq!(lobby.game_status.read(|s| *s), GameStatus::Finished);
    }

    #[test]
    fn test_duel_wrong_turn_is_ignored() {
        let lobby = create_test_lobby();
        let p1 = PlayerId::from("p1");
        let p2 = PlayerId::from("p2");
        lobby.add_player(p1.clone(), "Alice".to_string()).unwrap();
        lobby.add_player(p2.clone(), "Bob".to_string()).unwrap();
        lobby.settings.write(|s| { s.mode = shared::GameMode::Duel; s.initial_lives = Some(3); });
        lobby.game_status.write(|s| *s = GameStatus::Playing);
        lobby.turn_order.write(|o| { o.push(p1.clone()); o.push(p2.clone()); });
        lobby.current_prompt.write(|k| *k = Some(ActivePrompt::Kanji { character: "日".to_string() }));
        lobby.active_level_indices.write(|i| i.push(0));

        // p2 submits on p1's turn — should be silently ignored
        lobby.process_guess(&p2, "日本").unwrap();
        assert_eq!(lobby.get_player_score(&p2).unwrap(), 0);
    }

    #[tokio::test]
    async fn test_get_public_lobbies() {
        let app_state = AppState::create().expect("Failed to create AppState");
        
        // Create 3 lobbies: 1 private, 2 public
        let id1 = LobbyId::from("LOBBY1");
        let id2 = LobbyId::from("LOBBY2");
        let id3 = LobbyId::from("LOBBY3");
        
        let lobby1 = Arc::new(create_test_lobby()); // Private by default
        let lobby2 = Arc::new(create_test_lobby());
        lobby2.settings.write(|s| s.is_public = true);
        lobby2.add_player(PlayerId::from("leader2"), "Leader 2".to_string()).unwrap();
        
        let lobby3 = Arc::new(create_test_lobby());
        lobby3.settings.write(|s| s.is_public = true);
        lobby3.add_player(PlayerId::from("leader3"), "Leader 3".to_string()).unwrap();
        
        app_state.lobbies.write(|lobbies| {
            lobbies.insert(id1, lobby1);
            lobbies.insert(id2.clone(), lobby2);
            lobbies.insert(id3.clone(), lobby3);
        });
        
        let public_lobbies: Vec<shared::LobbySummary> = app_state.get_public_lobbies().await.unwrap();
        assert_eq!(public_lobbies.len(), 2);
        
        let ids: HashSet<LobbyId> = public_lobbies.into_iter().map(|l| l.id).collect();
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
        assert!(!ids.contains(&LobbyId::from("LOBBY1")));
    }

    #[tokio::test]
    async fn test_join_visibility_logic() {
        let app_state = AppState::create().expect("Failed to create AppState");
        let lobby_id = LobbyId::from("PRIVATE");
        let lobby = Arc::new(create_test_lobby());
        lobby.settings.write(|s| s.is_public = false);
        
        app_state.lobbies.write(|lobbies| {
            lobbies.insert(lobby_id.clone(), lobby);
        });
        
        // 1. Join from public list should FAIL
        let req_public = shared::JoinLobbyRequest {
            player_name: "Attacker".to_string(),
            player_id: None,
            joining_from_public_list: true,
        };
        let res_public: shared::api_fns::JsonResult = app_state.join_lobby(lobby_id.clone(), req_public).await;
        assert!(res_public.is_err());
        
        // 2. Join via manual code should SUCCEED
        let req_manual = shared::JoinLobbyRequest {
            player_name: "Friend".to_string(),
            player_id: None,
            joining_from_public_list: false,
        };
        let res_manual: shared::api_fns::JsonResult = app_state.join_lobby(lobby_id, req_manual).await;
        assert!(res_manual.is_ok());
    }
}
