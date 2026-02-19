use leptos::prelude::*;
use shared::{PlayerData, PlayerId, ClientMessage, ServerMessage};
use wasm_bindgen_futures::spawn_local;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use std::collections::HashMap;

#[derive(Clone)]
pub struct UseGameSocketConfig {
    pub lobby_id: ReadSignal<String>,
    pub player_id: ReadSignal<PlayerId>,
    pub set_kanji: WriteSignal<String>,
    pub set_result: WriteSignal<String>,
    pub set_score: WriteSignal<u32>,
    pub set_all_players: WriteSignal<Vec<PlayerData>>,
    pub set_typing_status: WriteSignal<HashMap<PlayerId, String>>,
}

pub fn use_game_socket(config: UseGameSocketConfig) -> impl Fn(ClientMessage) + Copy + 'static {
    // Signal to hold the current WebSocket sender
    let ws_sender = RwSignal::new(None::<futures::channel::mpsc::UnboundedSender<String>>);

    let lobby_id = config.lobby_id;
    let player_id = config.player_id;
    let set_kanji = config.set_kanji;
    let set_result = config.set_result;
    let set_score = config.set_score;
    let set_all_players = config.set_all_players;
    let set_typing_status = config.set_typing_status;

    Effect::new(move |_| {
        let lobby_id = lobby_id.get();
        let player_id = player_id.get();

        // Create a FRESH channel for this connection attempt
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();

        // Store the sender so perform_submit can use it
        ws_sender.set(Some(tx));

        spawn_local(async move {
            // Calculate WS URL
            let window = web_sys::window().unwrap();
            let location = window.location();
            let protocol = if location.protocol().unwrap() == "https:" { "wss" } else { "ws" };
            let host = location.host().unwrap();
            let ws_url = format!("{}://{}/ws/{}/{}", protocol, host, lobby_id, player_id);

            let ws = match WebSocket::open(&ws_url) {
                Ok(ws) => ws,
                Err(e) => {
                    leptos::logging::error!("Failed to open connection: {:?}", e);
                    return;
                }
            };

            let (mut write, mut read) = ws.split();

            // First, forward outgoing messages (channel -> WebSocket)
            spawn_local(async move {
                while let Some(msg) = rx.next().await {
                    let _ = write.send(Message::Text(msg)).await;
                }
            });

            // Second, handle incoming messages (WebSocket -> Signals)
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                        match server_msg {
                            ServerMessage::GameState { kanji: new_kanji, status: _, scores } => {
                                set_kanji.set(new_kanji);
                                set_all_players.set(scores);
                                set_typing_status.update(|m| m.clear());
                            },
                            ServerMessage::WordChecked { player_id: pid, result: res } => {
                                // Show result if it's our submission
                                if pid == player_id {
                                    set_result.set(res.message);
                                    set_score.set(res.score);
                                    if let Some(k) = res.kanji {
                                        set_kanji.set(k);
                                    }
                                }
                            },
                            ServerMessage::KanjiUpdate { new_kanji } => {
                                // Clear old result when new kanji arrives
                                set_result.set(String::new());
                                set_kanji.set(new_kanji);
                                set_typing_status.update(|m| m.clear());
                            },
                            ServerMessage::PlayerListUpdate { players } => {
                                set_all_players.set(players.clone());
                                // Update own score locally
                                if let Some(me) = players.iter().find(|p| p.id == player_id) {
                                    set_score.set(me.score);
                                }
                            },
                            ServerMessage::PlayerTyping { player_id: pid, input } => {
                                set_typing_status.update(|m| {
                                    if input.is_empty() {
                                        m.remove(&pid);
                                    } else {
                                        m.insert(pid, input);
                                    }
                                });
                            }
                            _ => {},
                        }
                    }
                }
            }
        });
    });

    // Return the send function
    move |msg: ClientMessage| {
        if let Some(mut sender) = ws_sender.get_untracked() {
            let payload = serde_json::to_string(&msg).unwrap();
            spawn_local(async move { let _ = sender.send(payload).await; });
        }
    }
}
