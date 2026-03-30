use leptos::prelude::*;
use shared::{LobbyInfo, LobbyId, PlayerId, ClientMessage, ServerMessage, GameStatus};
use crate::{persistence, components::toast::{ToastType, use_toast } };
use wasm_bindgen_futures::spawn_local;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use std::collections::HashMap;
use futures::future::{select, Either};

#[derive(Clone)]
pub struct UseSharedSocketConfig {
    pub lobby_id: ReadSignal<LobbyId>,
    pub player_id: ReadSignal<PlayerId>,

    // Central state that home manages
    pub lobby_info: RwSignal<Option<LobbyInfo>>,

    // Game specific states (prompt, word result, typing status) that could also conceptually live in Home,
    // but we can pass them down to Game via context or keep them isolated.
    // For now we will update global signals that Home passes to Game.
    pub set_prompt: WriteSignal<String>,
    pub set_result: WriteSignal<String>,
    pub set_typing_status: WriteSignal<HashMap<PlayerId, String>>,
    pub chat_messages: RwSignal<Vec<shared::ChatMessage>>,
    pub on_kicked: Option<Callback<()>>,
}

pub fn use_shared_socket(config: UseSharedSocketConfig) -> impl Fn(ClientMessage) + Copy + 'static {
    let ws_sender = RwSignal::new(None::<futures::channel::mpsc::UnboundedSender<String>>);

    let lobby_id = config.lobby_id;
    let player_id = config.player_id;
    let lobby_info_signal = config.lobby_info;
    let set_prompt = config.set_prompt;
    let set_result = config.set_result;
    let set_typing_status = config.set_typing_status;
    let chat_messages = config.chat_messages;
    let on_kicked = config.on_kicked;

    Effect::new(move |_| {
        let lobby_id = lobby_id.get();
        let player_id = player_id.get();

        if lobby_id.is_empty() || player_id.to_string().is_empty() {
            // Clear any existing sender so the old WS loop terminates
            ws_sender.set(None);
            return;
        }

        let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();
        let (halt_tx, mut halt_rx) = futures::channel::oneshot::channel::<()>();
        ws_sender.set(Some(tx));

        on_cleanup(move || {
            ws_sender.set(None);
            let _ = halt_tx.send(());
        });

        let toast = use_toast();

        spawn_local(async move {
            let window = web_sys::window().unwrap();
            let location = window.location();
            let protocol = if location.protocol().unwrap() == "https:" { "wss" } else { "ws" };
            let host = location.host().unwrap();
            let mut ws_url = format!("{}://{}/ws/{}/{}", protocol, host, lobby_id, player_id);
            if let Some(auth_data) = persistence::load_auth() {
                if let Some(token) = auth_data.token {
                    ws_url = format!("{}?token={}", ws_url, token);
                }
            }

            let ws = match WebSocket::open(&ws_url) {
                Ok(ws) => {
                    ws
                },
                Err(e) => {
                    leptos::logging::error!("Failed to open connection: {:?}", e);
                    toast.push.run((
                        "Could not connect to game server. Your session might be expired or the server is down.".to_string(),
                        ToastType::Error
                    ));
                    return;
                }
            };

            let (mut write, mut read) = ws.split();

            loop {
                let recv_fut = read.next();
                let send_fut = rx.next();

                match select(select(recv_fut, send_fut), &mut halt_rx).await {
                    Either::Left((Either::Left((msg, _)), _)) => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                match serde_json::from_str::<ServerMessage>(&text) {
                                    Ok(server_msg) => {
                                    match server_msg {
                                        ServerMessage::GameState { prompt: new_prompt, status, scores } => {
                                            leptos::logging::debug_warn!("[WS] GameState received: status={:?}, players={}", status, scores.len());
                                            set_prompt.set(new_prompt);

                                            lobby_info_signal.update(|info_opt| {
                                                let mut info = info_opt.clone().unwrap_or_else(|| LobbyInfo {
                                                    lobby_id: lobby_id.clone(),
                                                    ..Default::default()
                                                });
                                                info.status = status;
                                                info.players = scores;
                                                *info_opt = Some(info);
                                            });
                                            set_typing_status.update(|m| m.clear());
                                            if status == GameStatus::Lobby || status == GameStatus::Playing {
                                                set_result.set(String::new());
                                            }
                                        },
                                        ServerMessage::WordChecked { player_id: pid, result: res } => {
                                            if pid == player_id || pid.to_string().is_empty() || pid.to_string() == "null" || pid.to_string() == "" {
                                                let msg = res.message;
                                                // if let Some(details) = res.error_details {
                                                //     msg = format!("{}\nTry: {}", msg, details.join(", "));
                                                // }
                                                set_result.set(msg);
                                            }

                                            lobby_info_signal.update(|info_opt| {
                                                if let Some(info) = info_opt {
                                                    if let Some(me) = info.players.iter_mut().find(|p| p.id == pid) {
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
                                            let current_pid = player_id.clone();

                                            lobby_info_signal.update(|info_opt| {
                                                if let Some(info) = info_opt {
                                                    let old_players = info.players.clone();

                                                    // Find new players
                                                    for p in &new_players {
                                                        if p.id != current_pid && !old_players.iter().any(|old| old.id == p.id) {
                                                            toast.push.run((format!("{} joined!", p.name), ToastType::Info));
                                                        }
                                                    }

                                                    // Find leaving players
                                                    for p in &old_players {
                                                        if p.id != current_pid && !new_players.iter().any(|new| new.id == p.id) {
                                                            toast.push.run((format!("{} left!", p.name), ToastType::Info));
                                                        }
                                                    }

                                                    info.players = new_players;
                                                } else {
                                                    // Initial load, just set with stub info
                                                    *info_opt = Some(LobbyInfo {
                                                        lobby_id: lobby_id.clone(),
                                                        players: new_players,
                                                        ..Default::default()
                                                    });
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
                                            lobby_info_signal.update(|info_opt| {
                                                if let Some(info) = info_opt {
                                                    info.leader_id = lid;
                                                }
                                            });
                                        },
                                        ServerMessage::SettingsUpdate { settings: new_settings } => {
                                            lobby_info_signal.update(|info_opt| {
                                                if let Some(info) = info_opt {
                                                    info.settings = new_settings;
                                                }
                                            });
                                        },
                                        ServerMessage::SkipVoteUpdate { votes, required } => {
                                            set_result.set(format!("Votes to skip: {}/{}", votes, required));
                                        },
                                        ServerMessage::Kicked { player_id: pid } => {
                                            if pid == player_id {
                                                toast.push.run((
                                                    "You were kicked from the lobby.".to_string(),
                                                    ToastType::Error
                                                ));

                                                persistence::clear_session();
                                                if let Some(cb) = on_kicked {
                                                    cb.run(());
                                                } else {
                                                    let window = web_sys::window().unwrap();
                                                    let _ = window.location().set_href("/");
                                                }
                                            }
                                        },
                                        ServerMessage::ChatMessage(msg) => {
                                            chat_messages.update(|msgs| {
                                                msgs.push(msg);
                                                // Keep only last 100 messages to prevent infinite memory growth
                                                if msgs.len() > 100 {
                                                    msgs.remove(0);
                                                }
                                            });
                                        },
                                    }
                                    },
                                    Err(e) => {
                                        leptos::logging::warn!("[WS] Failed to deserialize message: {:?} | raw: {}", e, &text[..text.len().min(200)]);
                                    }
                                }
                            },
                            Some(Ok(_)) => {}, // Ignore binary/ping messages
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
                    Either::Left((Either::Right((msg, _)), _)) => {
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
                    },
                    Either::Right(_) => {
                        leptos::logging::log!("WS loop cancelled via halt signal");
                        break;
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
