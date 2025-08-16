// Custom hook for lobby polling
use crate::{api::get_lobby_info, error::log_error};
use leptos::prelude::*;
use shared::{GameStatus, LobbyInfo, PlayerId};
use wasm_bindgen_futures::spawn_local;

pub struct PollingConfig<F> {
    pub in_lobby: ReadSignal<bool>,
    pub current_lobby_id: ReadSignal<String>,
    pub current_player_id: ReadSignal<PlayerId>,
    pub set_lobby_info: WriteSignal<Option<LobbyInfo>>,
    pub set_status: WriteSignal<String>,
    pub set_in_lobby: WriteSignal<bool>,
    pub on_lobby_joined: F,
}

pub fn use_lobby_polling<F>(config: PollingConfig<F>)
where
    F: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
{
    let in_lobby = config.in_lobby;
    let current_lobby_id = config.current_lobby_id;
    let current_player_id = config.current_player_id;
    let set_lobby_info = config.set_lobby_info;
    let set_status = config.set_status;
    let set_in_lobby = config.set_in_lobby;
    let on_lobby_joined = config.on_lobby_joined;

    Effect::new(move |_| {
        if in_lobby.get() {
            let lobby_id = current_lobby_id.get();
            let config = PollingConfig {
                in_lobby,
                current_lobby_id,
                current_player_id,
                set_lobby_info,
                set_status,
                set_in_lobby,
                on_lobby_joined,
            };
            start_polling(lobby_id, config);
        }
    });
}

fn start_polling<F>(lobby_id: String, config: PollingConfig<F>)
where
    F: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
{
    spawn_local(async move {
        let mut consecutive_errors = 0;
        loop {
            gloo_timers::future::TimeoutFuture::new(1000).await;

            if !config.in_lobby.get() {
                break;
            }

            match get_lobby_info(&lobby_id).await {
                Ok(info) => {
                    consecutive_errors = 0;
                    if matches!(info.status, GameStatus::Playing) {
                        let lobby_id = config.current_lobby_id.get();
                        let player_id = config.current_player_id.get();
                        (config.on_lobby_joined)(lobby_id, player_id);
                        config.set_in_lobby.set(false);
                        break;
                    }
                    config.set_lobby_info.set(Some(info));
                }
                Err(e) => {
                    consecutive_errors += 1;
                    log_error("Failed to fetch lobby info", &e);
                    if consecutive_errors >= 5 {
                        config
                            .set_status
                            .set("Lost connection to lobby".to_string());
                        config.set_in_lobby.set(false);
                        break;
                    }
                }
            }
        }
    });
}
