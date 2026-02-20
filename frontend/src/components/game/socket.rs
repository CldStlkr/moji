use leptos::prelude::*;
use shared::{PlayerData, PlayerId, ClientMessage, ServerMessage};
use wasm_bindgen_futures::spawn_local;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use std::collections::HashMap;
use futures::future::{select, Either};

#[derive(Clone)]
pub struct UseGameSocketConfig {
    pub lobby_id: ReadSignal<String>,
    pub player_id: ReadSignal<PlayerId>,
    pub set_kanji: WriteSignal<String>,
    pub set_result: WriteSignal<String>,
    pub set_score: WriteSignal<u32>,
    pub set_all_players: WriteSignal<Vec<PlayerData>>,
    pub set_typing_status: WriteSignal<HashMap<PlayerId, String>>,
    pub set_status: WriteSignal<shared::GameStatus>,
    pub set_leader_id: WriteSignal<PlayerId>,
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
    let set_status = config.set_status;
    let set_leader_id = config.set_leader_id;

    Effect::new(move |_| {
        let lobby_id = lobby_id.get();
        let player_id = player_id.get();

        // Create a fresh channel for this connection attempt
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
                Ok(ws) => {
                    leptos::logging::log!("WebSocket connected to {}", ws_url);
                    ws
                },
                Err(e) => {
                    leptos::logging::error!("Failed to open connection: {:?}", e);
                    return;
                }
            };

            on_cleanup(move || {
                ws_sender.set(None);
            });

            let (mut write, mut read) = ws.split();

            // Combined loop to ensure everything is dropped when one side closes
            // Using a loop with select ensures we process both Read (from server) and Write (from client)
            // If either stream closes (Server disconnects OR Client Cleanup closes rx), we exit the loop.
            loop {
                // We need to pin the futures for select
                let recv_fut = read.next();
                let send_fut = rx.next();

                match select(recv_fut, send_fut).await {
                    Either::Left((msg, _)) => {
                        // Message from Server (read)
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                    match server_msg {
                                        ServerMessage::GameState { kanji: new_kanji, status, scores } => {
                                            set_kanji.set(new_kanji);
                                            set_status.set(status);
                                            set_all_players.set(scores);
                                            set_typing_status.update(|m| m.clear());
                                        },
                                        ServerMessage::WordChecked { player_id: pid, result: res } => {
                                            if pid == player_id {
                                                set_result.set(res.message);
                                                set_score.set(res.score);
                                                if let Some(k) = res.kanji {
                                                    set_kanji.set(k);
                                                }
                                            }
                                        },
                                        ServerMessage::KanjiUpdate { new_kanji } => {
                                            set_result.set(String::new());
                                            set_kanji.set(new_kanji);
                                            set_typing_status.update(|m| m.clear());
                                        },
                                        ServerMessage::PlayerListUpdate { players } => {
                                            set_all_players.set(players.clone());
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
                                        },
                                        ServerMessage::LeaderUpdate { leader_id } => {
                                            set_leader_id.set(leader_id);
                                        },
                                        _ => {},
                                    }
                                }
                            },
                            Some(Ok(Message::Bytes(_))) => {},
                            Some(Err(e)) => {
                                leptos::logging::error!("WS Error: {:?}", e);
                                break;
                            },
                            None => {
                                leptos::logging::log!("WS Server closed connection");
                                break;
                            }
                        }
                    },
                    Either::Right((msg, _)) => {
                        // Message from Client (rx) to be sent
                        match msg {
                            Some(text) => {
                                if let Err(e) = write.send(Message::Text(text)).await {
                                    leptos::logging::error!("Failed to send WS message: {:?}", e);
                                    break; 
                                }
                            },
                            None => {
                                // RX closed (cleanup)
                                let _ = write.close().await;
                                break;
                            }
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
