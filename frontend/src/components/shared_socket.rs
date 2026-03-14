use leptos::prelude::*;
use shared::{LobbyInfo, LobbyId, PlayerId, ClientMessage, ServerMessage};
use crate::components::toast::use_toast;
use wasm_bindgen_futures::spawn_local;
use gloo_net::websocket::{futures::WebSocket, Message};
use std::collections::HashMap;
use futures::{SinkExt, StreamExt, FutureExt};

#[derive(Clone)]
pub struct UseSharedSocketConfig {
    pub lobby_id: ReadSignal<LobbyId>,
    pub player_id: ReadSignal<PlayerId>,
    pub lobby_info: RwSignal<Option<LobbyInfo>>,
    pub set_prompt: WriteSignal<String>,
    pub set_result: WriteSignal<String>,
    pub set_typing_status: WriteSignal<HashMap<PlayerId, String>>,
}

pub fn use_shared_socket(config: UseSharedSocketConfig) -> impl Fn(ClientMessage) + Copy + 'static {
    let ws_sender = RwSignal::new(None::<futures::channel::mpsc::UnboundedSender<String>>);

    let lobby_id = config.lobby_id;
    let player_id = config.player_id;
    let lobby_info_signal = config.lobby_info;
    let set_prompt = config.set_prompt;
    let set_result = config.set_result;
    let set_typing_status = config.set_typing_status;

    Effect::new(move |_| {
        let lobby_id = lobby_id.get();
        let player_id = player_id.get();

        if lobby_id.is_empty() || player_id.to_string().is_empty() {
            ws_sender.set(None);
            return;
        }

        let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();
        ws_sender.set(Some(tx));

        let (cancel_tx, cancel_rx) = futures::channel::oneshot::channel::<()>();

        let lobby_id_str = lobby_id.to_string();
        on_cleanup(move || {
            leptos::logging::log!("[cleanup] firing for lobby_id={:?}", lobby_id_str);
            ws_sender.set(None);
            // Sending on cancel_tx wakes up the select! immediately
            let _ = cancel_tx.send(());
        });

        let toast = use_toast();

        leptos::logging::log!("[effect] spawning WS loop for lobby_id={:?}", lobby_id);
        spawn_local(async move {
            leptos::logging::log!("[spawn_local] starting for lobby_id={:?}", lobby_id);
            let window = web_sys::window().unwrap();
            let location = window.location();
            let protocol = if location.protocol().unwrap() == "https:" { "wss" } else { "ws" };
            let host = location.host().unwrap();
            let mut ws_url = format!("{}://{}/ws/{}/{}", protocol, host, lobby_id, player_id);
            if let Some(auth_data) = crate::persistence::load_auth() {
                if let Some(token) = auth_data.token {
                    ws_url = format!("{}?token={}", ws_url, token);
                }
            }

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

            let (mut write, mut read) = ws.split();
            let mut cancel_rx = cancel_rx.fuse();

            loop {
                let recv_fut = read.next().fuse();
                let send_fut = rx.next().fuse();

                futures::pin_mut!(recv_fut);
                futures::pin_mut!(send_fut);

                futures::select! {
                    msg = recv_fut => {
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
                                            },
                                            ServerMessage::WordChecked { player_id: pid, result: res } => {
                                                if pid == player_id || pid.to_string().is_empty() || pid.to_string() == "null" || pid.to_string() == "" {
                                                    set_result.set(res.message);
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
                                            ServerMessage::PromptUpdate { new_prompt } => {
                                                set_result.set(String::new());
                                                set_prompt.set(new_prompt);
                                                set_typing_status.update(|m| m.clear());
                                            },
                                            ServerMessage::PlayerListUpdate { players: new_players } => {
                                                let current_pid = player_id.clone();
                                                lobby_info_signal.update(|info_opt| {
                                                    if let Some(info) = info_opt {
                                                        let old_players = info.players.clone();
                                                        for p in &new_players {
                                                            if p.id != current_pid && !old_players.iter().any(|old| old.id == p.id) {
                                                                toast.push.run((format!("{} joined!", p.name), crate::components::toast::ToastType::Info));
                                                            }
                                                        }
                                                        for p in &old_players {
                                                            if p.id != current_pid && !new_players.iter().any(|new| new.id == p.id) {
                                                                toast.push.run((format!("{} left!", p.name), crate::components::toast::ToastType::Info));
                                                            }
                                                        }
                                                        info.players = new_players;
                                                    } else {
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
                                        }
                                    },
                                    Err(e) => {
                                        leptos::logging::warn!("[WS] Failed to deserialize message: {:?} | raw: {}", e, &text[..text.len().min(200)]);
                                    }
                                }
                            },
                            Some(Ok(_)) => {},
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
                    msg = send_fut => {
                        match msg {
                            Some(text) => {
                                if let Err(e) = write.send(Message::Text(text)).await {
                                    leptos::logging::log!("WS send failed: {:?}", e);
                                    break;
                                }
                            },
                            None => {
                                let _ = write.close().await;
                                break;
                            }
                        }
                    },
                    _ = cancel_rx => {
                        leptos::logging::log!("WS cancelled, closing connection");
                        let _ = write.close().await;
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
