use leptos::prelude::*;
use shared::{LobbyInfo, PlayerId, ClientMessage, ServerMessage};
use wasm_bindgen_futures::spawn_local;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use std::collections::HashMap;
use futures::future::{select, Either};

#[derive(Clone)]
pub struct UseSharedSocketConfig {
    pub lobby_id: ReadSignal<String>,
    pub player_id: ReadSignal<PlayerId>,

    // Central state that home manages
    pub set_lobby_info: WriteSignal<Option<LobbyInfo>>,

    // Game specific states (prompt, word result, typing status) that could also conceptually live in Home,
    // but we can pass them down to Game via context or keep them isolated.
    // For now we will update global signals that Home passes to Game.
    pub set_prompt: WriteSignal<String>,
    pub set_result: WriteSignal<String>,
    pub set_typing_status: WriteSignal<HashMap<PlayerId, String>>,
}

pub fn use_shared_socket(config: UseSharedSocketConfig) -> impl Fn(ClientMessage) + Copy + 'static {
    let ws_sender = RwSignal::new(None::<futures::channel::mpsc::UnboundedSender<String>>);

    let lobby_id = config.lobby_id;
    let player_id = config.player_id;
    let set_lobby_info = config.set_lobby_info;
    let set_prompt = config.set_prompt;
    let set_result = config.set_result;
    let set_typing_status = config.set_typing_status;

    Effect::new(move |_| {
        let lobby_id = lobby_id.get();
        let player_id = player_id.get();

        if lobby_id.is_empty() || player_id.to_string().is_empty() {
            return;
        }

        let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();
        ws_sender.set(Some(tx));

        spawn_local(async move {
            let window = web_sys::window().unwrap();
            let location = window.location();
            let protocol = if location.protocol().unwrap() == "https:" { "wss" } else { "ws" };
            let host = location.host().unwrap();
            let ws_url = format!("{}://{}/ws/{}/{}", protocol, host, lobby_id, player_id);

            let ws = match WebSocket::open(&ws_url) {
                Ok(ws) => {
                    leptos::logging::log!("WebSocket connected to {:?}", ws_url);
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

            loop {
                let recv_fut = read.next();
                let send_fut = rx.next();

                match select(recv_fut, send_fut).await {
                    Either::Left((msg, _)) => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                                    match server_msg {
                                        ServerMessage::GameState { prompt: new_prompt, status, scores } => {
                                            set_prompt.set(new_prompt);
                                            set_lobby_info.update(|info| {
                                                if let Some(i) = info {
                                                    i.status = status;
                                                    i.players = scores;
                                                } else {
                                                    // First GameState before lobby_info was fetched via API
                                                    *info = Some(LobbyInfo {
                                                        status,
                                                        players: scores,
                                                        ..Default::default()
                                                    });
                                                }
                                            });
                                            set_typing_status.update(|m| m.clear());
                                        },
                                        ServerMessage::WordChecked { player_id: pid, result: res } => {
                                            if pid == player_id {
                                                set_result.set(res.message);
                                            }
                                            set_lobby_info.update(|info| {
                                                if let Some(i) = info {
                                                    if let Some(me) = i.players.iter_mut().find(|p| p.id == pid) {
                                                        me.score = res.score;
                                                    }
                                                }
                                            });
                                            if let Some(k) = res.prompt {
                                                set_prompt.set(k);
                                            }
                                        },
                                        ServerMessage::PromptUpdate { new_prompt} => {
                                            set_result.set(String::new());
                                            set_prompt.set(new_prompt);
                                            set_typing_status.update(|m| m.clear());
                                        },
                                        ServerMessage::PlayerListUpdate { players: new_players } => {
                                            set_lobby_info.update(|info| {
                                                if let Some(i) = info {
                                                    // Maintain the turn order correctly from server.
                                                    i.players = new_players;
                                                }
                                            });
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
                                        ServerMessage::LeaderUpdate { leader_id: lid } => {
                                            set_lobby_info.update(|info| {
                                                if let Some(i) = info {
                                                    i.leader_id = lid;
                                                }
                                            });
                                        },
                                        ServerMessage::SettingsUpdate { settings: new_settings } => {
                                            set_lobby_info.update(|info| {
                                                if let Some(i) = info {
                                                    i.settings = new_settings;
                                                }
                                            });
                                        },
                                    }
                                }
                            },
                            Some(Ok(Message::Bytes(_))) => {},
                            Some(Err(e)) => {
                                leptos::logging::log!("WS closed: {:?}", e);
                                break;
                            },
                            None => {
                                leptos::logging::log!("WS Server closed connection");
                                break;
                            }
                        }
                    },
                    Either::Right((msg, _)) => {
                        match msg {
                            Some(text) => {
                                if let Err(e) = write.send(Message::Text(text)).await {
                                    leptos::logging::log!("WS send failed (connection closing): {:?}", e);
                                    break;
                                }
                            },
                            None => {
                                let _ = write.close().await;
                                break;
                            }
                        }
                    }
                }
            }
        });
    });

    move |msg: ClientMessage| {
        if let Some(mut sender) = ws_sender.get_untracked() {
            let payload = serde_json::to_string(&msg).unwrap();
            spawn_local(async move { let _ = sender.send(payload).await; });
        }
    }
}
