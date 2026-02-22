use crate::error::{parse_error_response, ClientError};
use gloo_net::http::Request;
use shared::{
    JoinLobbyRequest, PromptResponse, LobbyInfo, PlayerData, PlayerId,
    StartGameRequest, UpdateSettingsRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize)]
pub struct CheckUsernameResponse {
    pub available: bool,
    pub is_guest: bool,
}

#[derive(Debug, Serialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: Option<String>,
    pub create_guest: bool,
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub message: String,
    pub user: serde_json::Value,
    pub token: String,
}
const API_BASE: &str = "";

pub type ApiResult<T> = Result<T, ClientError>;

pub async fn create_lobby(request: JoinLobbyRequest) -> ApiResult<serde_json::Value> {
    make_request("POST", format!("{}/lobby/create", API_BASE), Some(&request)).await
}

pub async fn get_lobby_info(lobby_id: &str) -> ApiResult<LobbyInfo> {
    make_request::<(), _>("GET", format!("{}/lobby/{}/info", API_BASE, lobby_id), None).await
}

pub async fn update_lobby_settings(
    lobby_id: &str,
    request: UpdateSettingsRequest,
) -> ApiResult<serde_json::Value> {
    make_request(
        "POST",
        format!("{}/lobby/{}/settings", API_BASE, lobby_id),
        Some(&request),
    )
    .await
}

pub async fn start_game(lobby_id: &str, request: StartGameRequest) -> ApiResult<serde_json::Value> {
    make_request(
        "POST",
        format!("{}/lobby/{}/start", API_BASE, lobby_id),
        Some(&request),
    )
    .await
}

pub async fn reset_lobby(lobby_id: &str, player_id: &PlayerId) -> ApiResult<serde_json::Value> {
    make_request(
        "POST",
        format!("{}/lobby/{}/reset", API_BASE, lobby_id),
        Some(&serde_json::json!({ "player_id": player_id })),
    )
    .await
}

pub async fn get_lobby_players(lobby_id: &str) -> ApiResult<serde_json::Value> {
    make_request::<(), _>(
        "GET",
        format!("{}/lobby/players/{}", API_BASE, lobby_id),
        None,
    )
    .await
}

pub async fn join_lobby(lobby_id: &str, request: JoinLobbyRequest) -> ApiResult<serde_json::Value> {
    make_request(
        "POST",
        format!("{}/lobby/join/{}", API_BASE, lobby_id),
        Some(&request),
    )
    .await
}

pub async fn get_prompt(lobby_id: &str) -> ApiResult<PromptResponse> {
    make_request::<(), _>("GET", format!("{}/prompt/{}", API_BASE, lobby_id), None).await
}

pub async fn generate_new_prompt(lobby_id: &str) -> ApiResult<PromptResponse> {
    make_request::<(), _>("POST", format!("{}/new_prompt/{}", API_BASE, lobby_id), None).await
}

pub async fn check_username(username: &str) -> ApiResult<CheckUsernameResponse> {
    make_request::<(), _>("GET", format!("{}/auth/check/{}", API_BASE, username), None).await
}

pub async fn authenticate(request: AuthRequest) -> ApiResult<AuthResponse> {
    make_request("POST", format!("{}/auth/login", API_BASE), Some(&request)).await
}

pub async fn create_guest_account(username: &str) -> ApiResult<serde_json::Value> {
    let request = AuthRequest {
        username: username.to_string(),
        password: None,
        create_guest: true,
    };
    let response: AuthResponse = authenticate(request).await?;
    Ok(json!({
        "username": response.user["username"],
        "token": response.token
    }))
}


pub async fn get_player_info(lobby_id: &str, player_id: &PlayerId) -> ApiResult<PlayerData> {
    make_request::<(), _>(
        "GET",
        format!("{}/player/{}/{}", API_BASE, lobby_id, player_id),
        None,
    )
    .await
}

pub async fn leave_lobby(lobby_id: &str, player_id: &PlayerId) -> ApiResult<serde_json::Value> {
    make_request(
        "POST",
        format!("{}/lobby/{}/leave", API_BASE, lobby_id),
        Some(&serde_json::json!({ "player_id": player_id }))
    ).await
}

pub async fn logout(username: &str) -> ApiResult<serde_json::Value> {
    make_request(
        "POST",
        format!("{}/auth/logout", API_BASE),
        Some(&serde_json::json!({ "username": username }))
    ).await
}


// Helper function for making HTTP requests
async fn make_request<T, U>(method: &str, url: String, body: Option<&T>) -> ApiResult<U>
where
    T: serde::Serialize + ?Sized,
    U: for<'de> serde::Deserialize<'de>,
{
    // Create the base request builder
    let mut request_builder = match method {
        "GET" => Request::get(&url),
        "POST" => Request::post(&url),
        "PUT" => Request::put(&url),
        "DELETE" => Request::delete(&url),
        _ => return Err(ClientError::Network("Invalid HTTP method".into())),
    };

    // Add content-type header if there's a body
    if body.is_some() {
        request_builder = request_builder.header("Content-Type", "application/json");
    }

    // Add body if provided
    let response = if let Some(data) = body {
        // Convert potential json error to ClientError
        let request = match request_builder.json(data) {
            Ok(req) => req,
            Err(err) => {
                return Err(ClientError::Data(format!(
                    "Failed to serialize request: {}",
                    err
                )))
            }
        };

        // Send with body
        request.send().await
    } else {
        // Send without body
        request_builder.send().await
    };

    // Handle potential send error
    let response = match response {
        Ok(resp) => resp,
        Err(err) => return Err(ClientError::from(err)),
    };

    if response.ok() {
        match response.json::<U>().await {
            Ok(data) => Ok(data),
            Err(err) => Err(ClientError::from(err)),
        }
    } else {
        Err(parse_error_response(response).await)
    }
}
