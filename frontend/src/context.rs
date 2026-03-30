use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use shared::{PlayerId, LobbyId, LobbyInfo};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub username: String,
    pub is_guest: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct AuthContext {
    pub user: ReadSignal<Option<User>>,
    pub set_user: WriteSignal<Option<User>>,
    pub show_auth_modal: ReadSignal<bool>,
    pub set_show_auth_modal: WriteSignal<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct GameContext {
    pub lobby_id: ReadSignal<LobbyId>,
    pub set_lobby_id: WriteSignal<LobbyId>,
    pub player_id: ReadSignal<PlayerId>,
    pub set_player_id: WriteSignal<PlayerId>,
    pub player_name: ReadSignal<String>,
    pub set_player_name: WriteSignal<String>,
    pub lobby_info: ReadSignal<Option<LobbyInfo>>,
    pub set_lobby_info: WriteSignal<Option<LobbyInfo>>,
    pub is_leader: Memo<bool>,

    // Game Specific (Shared via WS)
    pub prompt: ReadSignal<String>,
    pub set_prompt: WriteSignal<String>,
    pub result: ReadSignal<String>,
    pub set_result: WriteSignal<String>,
    pub typing_status: ReadSignal<std::collections::HashMap<shared::PlayerId, String>>,
    pub set_typing_status: WriteSignal<std::collections::HashMap<shared::PlayerId, String>>,
    pub chat_messages: RwSignal<Vec<shared::ChatMessage>>,
    pub send_message: Callback<shared::ClientMessage>,
}

#[derive(Clone, Copy, Debug)]
pub struct InGameContext {
    pub word: RwSignal<String>,
    pub is_loading: RwSignal<bool>,
    pub input_ref: NodeRef<leptos::html::Input>,
    pub error_message: RwSignal<String>,
    pub shake_trigger: RwSignal<bool>,
    pub on_exit_game: Callback<()>,

    // Actions
    pub on_submit: Callback<()>,
    pub on_skip: Callback<()>,
    pub on_return_to_lobby: Callback<()>,
}

impl AuthContext {
    pub fn is_authenticated(&self) -> bool {
        self.user.get().is_some()
    }
}

pub async fn create_guest_account(username: String) -> Result<(String, Option<String>), leptos::server_fn::error::ServerFnError> {
    let req = shared::AuthRequest {
        username: username.clone(),
        password: None,
        create_guest: true,
    };

    let response = shared::authenticate(req).await?;
    let final_username = response
        .get("user")
        .and_then(|u| u.get("username"))
        .and_then(|u| u.as_str())
        .unwrap_or(&username)
        .to_string();

    let token = response
        .get("token")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    Ok((final_username, token))
}
