use leptos::prelude::*;
use crate::{
    PlayerId, LobbyInfo, JoinLobbyRequest, StartGameRequest,
    UpdateSettingsRequest, PlayerData, PromptResponse
};

#[cfg(feature = "ssr")]
use async_trait::async_trait;
#[cfg(feature = "ssr")]
use leptos::server_fn::error::ServerFnError;

pub type JsonResult = Result<serde_json::Value, ServerFnError>;
pub type PromptResult = Result<PromptResponse, ServerFnError>;
pub type LobbyResult = Result<LobbyInfo, ServerFnError>;
pub type PlayerResult = Result<PlayerData, ServerFnError>;


#[cfg(feature = "ssr")]
#[async_trait]
pub trait ApiContext: Send + Sync {
    async fn create_lobby(&self, request: JoinLobbyRequest) -> JsonResult;
    async fn get_lobby_info(&self, lobby_id: String) -> LobbyResult;
    async fn update_lobby_settings(&self, lobby_id: String, request: UpdateSettingsRequest) -> JsonResult;
    async fn start_game(&self, lobby_id: String, request: StartGameRequest) -> JsonResult;
    async fn reset_lobby(&self, lobby_id: String, player_id: PlayerId) -> JsonResult;
    async fn get_lobby_players(&self, lobby_id: String) -> JsonResult;
    async fn join_lobby(&self, lobby_id: String, request: JoinLobbyRequest) -> JsonResult;
    async fn get_prompt(&self, lobby_id: String) -> PromptResult;
    async fn generate_new_prompt(&self, lobby_id: String) -> PromptResult;
    async fn check_username(&self, username: String) -> JsonResult;
    async fn authenticate(&self, request: crate::AuthRequest) -> JsonResult;
    async fn get_player_info(&self, lobby_id: String, player_id: PlayerId) -> PlayerResult;
    async fn leave_lobby(&self, lobby_id: String, player_id: PlayerId) -> JsonResult;
    async fn logout(&self, username: String) -> JsonResult;
}

#[cfg(feature = "ssr")]
fn get_api_context() -> Result<std::sync::Arc<dyn ApiContext>, ServerFnError> {
    use_context::<std::sync::Arc<dyn ApiContext>>()
        .ok_or_else(|| ServerFnError::new("Missing ApiContext"))
}

#[server(endpoint = "/api/create_lobby")]
pub async fn create_lobby(request: JoinLobbyRequest) -> JsonResult {
    get_api_context()?.create_lobby(request).await
}

#[server(endpoint = "/api/get_lobby_info")]
pub async fn get_lobby_info(lobby_id: String) -> LobbyResult {
    get_api_context()?.get_lobby_info(lobby_id).await
}

#[server(endpoint = "/api/update_lobby_settings")]
pub async fn update_lobby_settings(lobby_id: String, request: UpdateSettingsRequest) -> JsonResult {
    get_api_context()?.update_lobby_settings(lobby_id, request).await
}

#[server(endpoint = "/api/start_game")]
pub async fn start_game(lobby_id: String, request: StartGameRequest) -> JsonResult {
    get_api_context()?.start_game(lobby_id, request).await
}

#[server(endpoint = "/api/reset_lobby")]
pub async fn reset_lobby(lobby_id: String, player_id: PlayerId) -> JsonResult {
    get_api_context()?.reset_lobby(lobby_id, player_id).await
}

#[server(endpoint = "/api/get_lobby_players")]
pub async fn get_lobby_players(lobby_id: String) -> JsonResult {
    get_api_context()?.get_lobby_players(lobby_id).await
}

#[server(endpoint = "/api/join_lobby")]
pub async fn join_lobby(lobby_id: String, request: JoinLobbyRequest) -> JsonResult {
    get_api_context()?.join_lobby(lobby_id, request).await
}

#[server(endpoint = "/api/get_prompt")]
pub async fn get_prompt(lobby_id: String) -> PromptResult {
    get_api_context()?.get_prompt(lobby_id).await
}

#[server(endpoint = "/api/generate_new_prompt")]
pub async fn generate_new_prompt(lobby_id: String) -> PromptResult {
    get_api_context()?.generate_new_prompt(lobby_id).await
}

#[server(endpoint = "/api/check_username")]
pub async fn check_username(username: String) -> JsonResult {
    get_api_context()?.check_username(username).await
}

#[server(endpoint = "/api/authenticate")]
pub async fn authenticate(request: crate::AuthRequest) -> JsonResult {
    get_api_context()?.authenticate(request).await
}

#[server(endpoint = "/api/get_player_info")]
pub async fn get_player_info(lobby_id: String, player_id: PlayerId) -> PlayerResult {
    get_api_context()?.get_player_info(lobby_id, player_id).await
}

#[server(endpoint = "/api/leave_lobby")]
pub async fn leave_lobby(lobby_id: String, player_id: PlayerId) -> JsonResult {
    get_api_context()?.leave_lobby(lobby_id, player_id).await
}

#[server(endpoint = "/api/logout")]
pub async fn logout(username: String) -> JsonResult {
    get_api_context()?.logout(username).await
}
