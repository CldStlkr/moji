// lobby/lobby_join.rs - Component for joining/creating lobbies
use crate::{
    api::{create_lobby, join_lobby},
    error::{get_user_friendly_message, log_error},
};
use leptos::ev;
use leptos::prelude::*;
use shared::{JoinLobbyRequest, PlayerId};
use wasm_bindgen_futures::spawn_local;

use super::{GameInstructions, StatusMessage};

#[component]
pub fn LobbyJoinComponent<F>(
    is_loading: ReadSignal<bool>,
    set_is_loading: WriteSignal<bool>,
    status: ReadSignal<String>,
    set_status: WriteSignal<String>,
    on_lobby_joined: F,
) -> impl IntoView
where
    F: Fn(String, PlayerId) + 'static + Copy + Send + Sync,
{
    let (input_lobby_id, set_input_lobby_id) = signal(String::new());
    let (player_name, set_player_name) = signal(String::new());

    let create_lobby_action = move |_: ev::MouseEvent| {
        let name = player_name.get();
        if name.trim().is_empty() {
            set_status.set("Please enter your name".to_string());
            return;
        }

        spawn_local(async move {
            set_is_loading.set(true);
            set_status.set("Creating lobby...".to_string());

            let request = JoinLobbyRequest {
                player_name: name.clone(),
            };

            match create_lobby(request).await {
                Ok(response) => {
                    let lobby_id = response
                        .get("lobby_id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("")
                        .to_string();
                    let player_id = PlayerId::from(
                        response
                            .get("player_id")
                            .and_then(|id| id.as_str())
                            .unwrap_or(""),
                    );

                    if lobby_id.is_empty() || player_id.0.is_empty() {
                        set_status.set("Invalid response from server".to_string());
                    } else {
                        set_status.set(format!("Created lobby: {}", lobby_id));
                        on_lobby_joined(lobby_id, player_id);
                    }
                }
                Err(e) => {
                    log_error("Failed to create lobby", &e);
                    set_status.set(get_user_friendly_message(&e));
                }
            }
            set_is_loading.set(false);
        });
    };

    let join_lobby_action = move |_: ev::MouseEvent| {
        let lobby_id = input_lobby_id.get();
        let name = player_name.get();

        if lobby_id.trim().is_empty() {
            set_status.set("Please enter a lobby ID".to_string());
            return;
        }
        if name.trim().is_empty() {
            set_status.set("Please enter your name".to_string());
            return;
        }

        spawn_local(async move {
            set_is_loading.set(true);
            set_status.set(format!("Joining lobby {}...", lobby_id));

            let request = JoinLobbyRequest {
                player_name: name.clone(),
            };

            match join_lobby(&lobby_id, request).await {
                Ok(response) => {
                    let player_id = PlayerId::from(
                        response
                            .get("player_id")
                            .and_then(|id| id.as_str())
                            .unwrap_or(""),
                    );

                    if player_id.0.is_empty() {
                        set_status.set("Invalid response from server".to_string());
                    } else {
                        set_status.set(format!("Joined lobby: {}", lobby_id));
                        on_lobby_joined(lobby_id, player_id);
                    }
                }
                Err(e) => {
                    log_error("Failed to join lobby", &e);
                    set_status.set(get_user_friendly_message(&e));
                }
            }
            set_is_loading.set(false);
        });
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !is_loading.get() {
            join_lobby_action(ev::MouseEvent::new("click").unwrap());
        }
    };

    view! {
        <div class="card max-w-2xl mx-auto my-8">
            <h2 class="text-3xl font-bold text-gray-800 mb-8 text-center">"Join or Create a Game"</h2>
            <div class="space-y-6">
                <div class="space-y-2">
                    <label for="player-name" class="block font-semibold text-gray-800 text-lg">"Your Name:"</label>
                    <input
                        type="text"
                        id="player-name"
                        value=move || player_name.get()
                        on:input=move |ev| set_player_name.set(event_target_value(&ev))
                        placeholder="Enter your name"
                        disabled=move || is_loading.get()
                        class="input-field w-full"
                    />
                </div>

                <button
                    on:click=create_lobby_action
                    disabled=move || is_loading.get() || player_name.get().trim().is_empty()
                    class="btn-primary w-full text-lg"
                >
                    "Create New Game"
                </button>

                <div class="flex gap-3 flex-col sm:flex-row">
                    <input
                        type="text"
                        value=move || input_lobby_id.get()
                        on:input=move |ev| set_input_lobby_id.set(event_target_value(&ev))
                        on:keydown=handle_key_press
                        placeholder="Enter Lobby ID"
                        disabled=move || is_loading.get()
                        class="input-field flex-1"
                    />
                    <button
                        on:click=join_lobby_action
                        disabled=move || is_loading.get() ||
                                      input_lobby_id.get().trim().is_empty() ||
                                      player_name.get().trim().is_empty()
                        class="btn-secondary whitespace-nowrap"
                    >
                        "Join Game"
                    </button>
                </div>
            </div>

            <StatusMessage status=status />
            <GameInstructions />
        </div>
    }
}
