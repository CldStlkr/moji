#![allow(unused_imports, dead_code)]

use moji_frontend::error::{get_user_friendly_message, ClientError};
use moji_frontend::persistence::{clear_session, load_session, save_session, SessionData};
use shared::{GameStatus, PlayerId};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Test 1: Basic persistence functionality
#[wasm_bindgen_test]
fn test_session_persistence_basic() {
    // Clear any existing session
    clear_session();

    // Create test session data (using the main branch structure)
    let session = SessionData {
        lobby_id: "TEST123".to_string(),
        player_id: PlayerId("player1".to_string()),
        player_name: "Test Player".to_string(),
        is_in_game: true,
    };

    // Save and load
    save_session(&session);
    let loaded = load_session().expect("Session should be saved");

    // Verify
    assert_eq!(loaded.lobby_id, session.lobby_id);
    assert_eq!(loaded.player_id, session.player_id);
    assert_eq!(loaded.player_name, session.player_name);
    assert_eq!(loaded.is_in_game, session.is_in_game);

    // Clean up
    clear_session();
    assert!(load_session().is_none());
}

// Test 2: API error handling
#[wasm_bindgen_test]
fn test_error_handling() {
    // Test network error message
    let network_error = ClientError::Network("Connection failed".to_string());
    let message = get_user_friendly_message(&network_error);
    assert!(message.contains("internet connection"));

    // Test not found error
    let not_found = ClientError::NotFound("Lobby not found".to_string());
    let message = get_user_friendly_message(&not_found);
    assert!(message.contains("Not found"));

    // Test validation error
    let validation = ClientError::Validation("Invalid name".to_string());
    let message = get_user_friendly_message(&validation);
    assert!(message.contains("Invalid input"));
}

// Test 3: Player ID handling
#[wasm_bindgen_test]
fn test_player_id() {
    // Test from string
    let id1 = PlayerId::from("test123");
    let id2 = PlayerId("test123".to_string());
    assert_eq!(id1, id2);

    // Test default
    let default_id = PlayerId::default();
    assert_eq!(default_id, PlayerId("".to_string()));
}

// Test 4: Session data edge cases
#[wasm_bindgen_test]
fn test_session_edge_cases() {
    clear_session();

    // Test with empty values
    let empty_session = SessionData {
        lobby_id: "".to_string(),
        player_id: PlayerId::default(),
        player_name: "".to_string(),
        is_in_game: false,
    };

    save_session(&empty_session);

    // Should not save empty session (based on your logic)
    // Actually, looking at your persistence.rs, it only checks lobby_id and player_id
    // So this might save. Let's test what actually happens:
    let loaded = load_session();
    assert!(loaded.is_none()); // Empty lobby_id and player_id shouldn't be saved
}

// Test 5: Error types
#[wasm_bindgen_test]
fn test_error_types() {
    // Test all error variants
    let errors = vec![
        ClientError::Connection("test".to_string()),
        ClientError::Server {
            status_code: 500,
            message: "test".to_string(),
        },
        ClientError::NotFound("test".to_string()),
        ClientError::Data("test".to_string()),
        ClientError::Auth("test".to_string()),
        ClientError::Validation("test".to_string()),
        ClientError::Network("test".to_string()),
    ];

    // Each should produce a user-friendly message
    for error in errors {
        let message = get_user_friendly_message(&error);
        assert!(!message.is_empty());
    }
}
