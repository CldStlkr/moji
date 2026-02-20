use leptos::prelude::*;
use shared::PlayerId;
use web_sys::Storage;

const STORAGE_KEY_LOBBY_ID: &str = "moji_lobby_id";
const STORAGE_KEY_PLAYER_ID: &str = "moji_player_id";
const STORAGE_KEY_PLAYER_NAME: &str = "moji_player_name";
const STORAGE_KEY_IS_IN_GAME: &str = "moji_is_in_game";

const STORAGE_KEY_AUTH_USERNAME: &str = "moji_auth_username";
const STORAGE_KEY_AUTH_IS_GUEST: &str = "moji_auth_is_guest";

#[derive(Clone, Debug)]
pub struct AuthData {
    pub username: String,
    pub is_guest: bool,
}

#[derive(Clone, Debug)]
pub struct SessionData {
    pub lobby_id: String,
    pub player_id: PlayerId,
    pub player_name: String,
    pub is_in_game: bool,
}

impl SessionData {
    pub fn is_valid(&self) -> bool {
        !self.lobby_id.trim().is_empty() && !self.player_id.trim().is_empty()
    }
}

fn get_storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn save_session(session: &SessionData) {
    if !session.is_valid() {
        return;
    }
    if let Some(storage) = get_storage() {
        let _ = storage.set_item(STORAGE_KEY_LOBBY_ID, &session.lobby_id);
        let _ = storage.set_item(STORAGE_KEY_PLAYER_ID, &session.player_id);
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

    let session = SessionData {
        lobby_id,
        player_id: PlayerId(player_id),
        player_name,
        is_in_game,
    };

    session.is_valid().then_some(session)
}

pub fn clear_session() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item(STORAGE_KEY_LOBBY_ID);
        let _ = storage.remove_item(STORAGE_KEY_PLAYER_ID);
        // let _ = storage.remove_item(STORAGE_KEY_PLAYER_NAME); // Kept in auth
        let _ = storage.remove_item(STORAGE_KEY_IS_IN_GAME);
    }
}

pub fn save_auth(auth: &AuthData) {
    if let Some(storage) = get_storage() {
        let _ = storage.set_item(STORAGE_KEY_AUTH_USERNAME, &auth.username);
        let _ = storage.set_item(STORAGE_KEY_AUTH_IS_GUEST, &auth.is_guest.to_string());
    }
}

pub fn load_auth() -> Option<AuthData> {
    let storage = get_storage()?;
    let username = storage.get_item(STORAGE_KEY_AUTH_USERNAME).ok()??;
    let is_guest = storage
        .get_item(STORAGE_KEY_AUTH_IS_GUEST)
        .ok()?
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);

    Some(AuthData { username, is_guest })
}

pub fn clear_auth() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item(STORAGE_KEY_AUTH_USERNAME);
        let _ = storage.remove_item(STORAGE_KEY_AUTH_IS_GUEST);
    }
}

// Hook to handle session persistence
pub fn use_session_persistence(
    lobby_id: impl Into<Signal<String>> + 'static,
    player_id: impl Into<Signal<PlayerId>> + 'static,
    player_name: impl Into<Signal<String>> + 'static,
    is_in_game: impl Into<Signal<bool>> + 'static,
) {
    let lobby_id = lobby_id.into();
    let player_id = player_id.into();
    let player_name = player_name.into();
    let is_in_game = is_in_game.into();
    Effect::new(move |_| {
        // Save session whenever these values change
        let session = SessionData {
            lobby_id: lobby_id.get(),
            player_id: player_id.get(),
            player_name: player_name.get(),
            is_in_game: is_in_game.get(),
        };

        // Only save if we have valid data
        if session.is_valid() {
            save_session(&session);
        }
    });
}
