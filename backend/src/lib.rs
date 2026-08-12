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
    use crate::utils::generate_lobby_id;

    #[test]
    fn test_generate_lobby_id() {
        let id = generate_lobby_id();
        assert_eq!(id.len(), 6);
        assert!(id.chars().all(|c| c.is_alphanumeric()));
    }

    // The lobby integration tests (add_player, process_guess, etc.) require a live
    // Redis connection because LobbyHandle stores all state in Redis.
    // They live in tests/lobby_tests.rs and can be run with:
    //   docker run -d -p 6379:6379 redis
    //   REDIS_URL=redis://localhost:6379 cargo test
}
