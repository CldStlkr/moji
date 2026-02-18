// frontend/src/components/lobby/lobby_socket.rs
use leptos::prelude::*;
use shared::{GameStatus, LobbyInfo, PlayerId, ServerMessage};
use wasm_bindgen_futures::spawn_local;
use futures::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message};
use crate::api::get_lobby_info;

pub struct LobbySocketConfig<F> {
    pub in_lobby: ReadSignal<bool>,
    pub current_lobby_id: ReadSignal<String>,
    pub current_player_id: ReadSignal<PlayerId>,
    pub set_lobby_info: WriteSignal<Option<LobbyInfo>>,
    pub on_lobby_joined: F,
}

pub fn use_lobby_socket<F>(config: LobbySocketConfig<F>)
where
    F: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
{
    let in_lobby = config.in_lobby;
    let current_lobby_id = config.current_lobby_id;
    let current_player_id = config.current_player_id;
    let set_lobby_info = config.set_lobby_info;
    let on_lobby_joined = config.on_lobby_joined;

    Effect::new(move |_| {
        if in_lobby.get() {
            let lobby_id = current_lobby_id.get();
            let player_id = current_player_id.get();

            spawn_local(async move {
                // 1. FETCH INITIAL STATE VIA HTTP
                if let Ok(info) = get_lobby_info(&lobby_id).await {
                     if matches!(info.status, GameStatus::Playing) {
                        on_lobby_joined(lobby_id.clone(), player_id.clone());
                        return; 
                    }
                    set_lobby_info.set(Some(info));
                }

                // 2. CONNECT WEBSOCKET FOR UPDATES
                let window = web_sys::window().unwrap();
                let location = window.location();
                let protocol = if location.protocol().unwrap() == "https:" { "wss" } else { "ws" };
                let host = location.host().unwrap();
                let ws_url = format!("{}://{}/ws/{}/{}", protocol, host, lobby_id, player_id);

                let ws = match WebSocket::open(&ws_url) {
                    Ok(ws) => ws,
                    Err(_) => return,
                };

                let (_, mut read) = ws.split();

                while let Some(msg) = read.next().await {
                    if let Ok(Message::Text(text)) = msg {
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                            match server_msg {
                                ServerMessage::GameState { status, .. } => {
                                    if status == GameStatus::Playing {
                                        on_lobby_joined(lobby_id.clone(), player_id.clone());
                                        return; 
                                    }
                                },
                                ServerMessage::PlayerListUpdate { players } => {
                                    set_lobby_info.update(|info| {
                                        if let Some(i) = info {
                                            i.players = players;
                                        }
                                    });
                                },
                                ServerMessage::SettingsUpdate { settings } => {
                                    set_lobby_info.update(|info| {
                                        if let Some(i) = info {
                                            i.settings = settings;
                                        }
                                    });
                                },
                                _ => {}
                            }
                        }
                    }
                }
            });
        }
    });
}
