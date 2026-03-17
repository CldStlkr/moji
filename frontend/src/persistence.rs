use leptos::prelude::*;
use shared::{LobbyId, PlayerId};
use web_sys::Storage;

use serde::{Serialize, Deserialize};

const STORAGE_KEY_LOBBY_ID: &str = "moji_lobby_id";
const STORAGE_KEY_PLAYER_ID: &str = "moji_player_id";
const STORAGE_KEY_PLAYER_NAME: &str = "moji_player_name";
const STORAGE_KEY_IS_IN_GAME: &str = "moji_is_in_game";

const STORAGE_KEY_AUTH_USERNAME: &str = "moji_auth_username";
const STORAGE_KEY_AUTH_IS_GUEST: &str = "moji_auth_is_guest";
const STORAGE_KEY_AUTH_TOKEN: &str = "moji_auth_token";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthData {
    pub username: String,
    pub is_guest: bool,
    pub token: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionData {
    pub lobby_id: LobbyId,
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
        lobby_id: LobbyId(lobby_id),
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
        if let Some(token) = &auth.token {
            let _ = storage.set_item(STORAGE_KEY_AUTH_TOKEN, token);
        } else {
            let _ = storage.remove_item(STORAGE_KEY_AUTH_TOKEN);
        }
    }
}

impl AuthData {
    pub fn is_expired(&self) -> bool {
        let Some(token) = &self.token else { return true; };
        
        let window = web_sys::window().unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 { return true; }
        
        // Decode payload (middle part)
        let payload_b64 = parts[1];
        let payload_b64 = payload_b64.replace("-", "+").replace("_", "/");
        let payload_b64 = match payload_b64.len() % 4 {
            2 => format!("{}==", payload_b64),
            3 => format!("{}=", payload_b64),
            _ => payload_b64.to_string(),
        };

        let decoded = match window.atob(&payload_b64) {
            Ok(d) => d,
            Err(_) => return true,
        };

        let json: serde_json::Value = match serde_json::from_str(&decoded) {
            Ok(j) => j,
            Err(_) => return true,
        };

        if let Some(exp) = json.get("exp").and_then(|e| e.as_u64()) {
            let now = js_sys::Date::now() / 1000.0;
            return (exp as f64) < now;
        }

        true
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
    let token = storage.get_item(STORAGE_KEY_AUTH_TOKEN).ok().flatten();

    let auth = AuthData { username, is_guest, token };
    if auth.is_expired() {
        leptos::logging::warn!("Auth token expired, clearing auth state.");
        clear_auth();
        return None;
    }

    Some(auth)
}

pub fn clear_auth() {
    if let Some(storage) = get_storage() {
        let _ = storage.remove_item(STORAGE_KEY_AUTH_USERNAME);
        let _ = storage.remove_item(STORAGE_KEY_AUTH_IS_GUEST);
        let _ = storage.remove_item(STORAGE_KEY_AUTH_TOKEN);
    }
}

// Hook to handle session persistence
pub fn use_session_persistence(
    lobby_id: impl Into<Signal<LobbyId>> + 'static,
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
