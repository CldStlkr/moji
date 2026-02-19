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
