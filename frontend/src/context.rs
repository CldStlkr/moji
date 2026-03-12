use leptos::prelude::*;
use serde::{Deserialize, Serialize};

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
