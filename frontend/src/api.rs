use crate::{CheckWordResponse, KanjiPrompt, LobbyResponse, UserInput};
use gloo_net::http::Request;

const API_BASE: &str = "";

pub async fn create_lobby() -> Result<LobbyResponse, gloo_net::Error> {
    let response = Request::post(&format!("{}/lobby/create", API_BASE))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    response.json().await
}

pub async fn join_lobby(lobby_id: &str) -> Result<LobbyResponse, gloo_net::Error> {
    let response = Request::get(&format!("{}/lobby/join/{}", API_BASE, lobby_id))
        .send()
        .await?;

    response.json().await
}

pub async fn get_kanji(lobby_id: &str) -> Result<KanjiPrompt, gloo_net::Error> {
    let response = Request::get(&format!("{}/kanji/{}", API_BASE, lobby_id))
        .send()
        .await?;

    if response.ok() {
        response.json().await
    } else {
        Err(gloo_net::Error::JsError(
            js_sys::Error::new("Failed to get kanji").into(),
        ))
    }
}

pub async fn generate_new_kanji(lobby_id: &str) -> Result<KanjiPrompt, gloo_net::Error> {
    let response = Request::post(&format!("{}/new_kanji/{}", API_BASE, lobby_id))
        .send()
        .await?;

    if response.ok() {
        response.json().await
    } else {
        Err(gloo_net::Error::JsError(
            js_sys::Error::new("Failed to generate new kanji").into(),
        ))
    }
}

pub async fn check_word(
    lobby_id: &str,
    user_input: UserInput,
) -> Result<CheckWordResponse, gloo_net::Error> {
    let body = serde_json::to_string(&user_input)
        .map_err(|e| gloo_net::Error::JsError(js_sys::Error::new(&e.to_string()).into()))?;

    let response = Request::post(&format!("{}/check_word/{}", API_BASE, lobby_id))
        .header("Content-Type", "application/json")
        .body(body)?
        .send()
        .await?;

    if response.ok() {
        response.json().await
    } else {
        Err(gloo_net::Error::JsError(
            js_sys::Error::new("Failed to check word").into(),
        ))
    }
}
