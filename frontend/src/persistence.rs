use leptos::prelude::*;
use shared::PlayerId;
use web_sys::Storage;

const STORAGE_KEY_LOBBY_ID: &str = "moji_lobby_id";
const STORAGE_KEY_PLAYER_ID: &str = "moji_player_id";
const STORAGE_KEY_PLAYER_NAME: &str = "moji_player_name";
const STORAGE_KEY_IS_IN_GAME: &str = "moji_is_in_game";

#[derive(Clone, Debug)]
pub struct SessionData {
    pub lobby_id: String,
    pub player_id: PlayerId,
    pub player_name: String,
    pub is_in_game: bool,
}

fn get_storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn save_session(session: &SessionData) {
    if let Some(storage) = get_storage() {
        let _ = storage.set_item(STORAGE_KEY_LOBBY_ID, &session.lobby_id);
        let _ = storage.set_item(STORAGE_KEY_PLAYER_ID, &session.player_id.0);
        let _ = storage.set_item(STORAGE_KEY_PLAYER_NAME, &session.player_name);
        let _ = storage.set_item(STORAGE_KEY_IS_IN_GAME, &session.is_in_game.to_string());
    }
}

pub fn load_session() -> Option<SessionData> {
    let storage = get_storage()?;

    let lobby_id = storage.get_item(STORAGE_KEY_LOBBY_ID).ok()??;
    let player_id = storage.get_item(STORAGE_KEY_PLAYER_ID).ok()??;
    let player_name = storage.get_item(STORAGE_KEY_PLAYER_NAME).ok()??;
    let is_in_game = storage
        .get_item(STORAGE_KEY_IS_IN_GAME)
        .ok()?
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    Some(SessionData {
        lobby_id,
        player_id: PlayerId(player_id),
        player_name,
        is_in_game,
    })
}

pub fn clear_session() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item(STORAGE_KEY_LOBBY_ID);
        let _ = storage.remove_item(STORAGE_KEY_PLAYER_ID);
        let _ = storage.remove_item(STORAGE_KEY_PLAYER_NAME);
        let _ = storage.remove_item(STORAGE_KEY_IS_IN_GAME);
    }
}

// Hook to handle session persistence
pub fn use_session_persistence(
    lobby_id: ReadSignal<String>,
    player_id: ReadSignal<PlayerId>,
    player_name: ReadSignal<String>,
    is_in_game: ReadSignal<bool>,
) {
    Effect::new(move |_| {
        // Save session whenever these values change
        let session = SessionData {
            lobby_id: lobby_id.get(),
            player_id: player_id.get(),
            player_name: player_name.get(),
            is_in_game: is_in_game.get(),
        };

        // Only save if we have valid data
        if !session.lobby_id.is_empty() && !session.player_id.0.is_empty() {
            save_session(&session);
        }
    });
}
