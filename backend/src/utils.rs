use rand::{RngExt, distr::Alphanumeric};
use shared::{ActivePrompt, LobbyId};
use std::collections::HashSet;
pub use shared::PlayerId;


pub fn generate_random_id(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn generate_player_id() -> PlayerId {
    PlayerId::from(generate_random_id(10))
}

pub fn generate_lobby_id() -> LobbyId {
    LobbyId::from(generate_random_id(6))
}


pub fn check_prompt(prompt: &shared::ActivePrompt, input: &str, dictionary: &HashSet<String>) -> bool {
    match prompt {
        ActivePrompt::Kanji { character } => {
            input.contains(character.as_str()) && dictionary.contains(input)
        },
        ActivePrompt::Vocab { readings, .. } => {
            readings.iter().any(|r| r == input)
        }
    }
}
