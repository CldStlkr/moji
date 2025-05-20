use crate::{
    api,
    error::{get_user_friendly_message, log_error, ClientError},
    JoinLobbyRequest,
};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn LobbyComponent<F>(on_lobby_joined: F) -> impl IntoView
where
    F: Fn(String, String) + 'static + Copy, // Updated to handle player_id
{
    let (input_lobby_id, set_input_lobby_id) = signal(String::new());
    let (player_name, set_player_name) = signal(String::new()); // New player name state
    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);

    let create_lobby = move |_: ev::MouseEvent| {
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

            match api::create_lobby(request).await {
                Ok(response) => {
                    let lobby_id = response
                        .get("lobby_id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("")
                        .to_string();
                    let player_id = response
                        .get("player_id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("")
                        .to_string();

                    if lobby_id.is_empty() || player_id.is_empty() {
                        log_error(
                            "Invalid response",
                            &ClientError::Data("Missing lobby_id or player_id".into()),
                        );
                        set_status.set("Invalid response from server".to_string());
                    } else {
                        set_status.set(format!("Lobby created: {}", lobby_id));
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

    let join_lobby = move |_: ev::MouseEvent| {
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

            match api::join_lobby(&lobby_id, request).await {
                // Changed to join_lobby
                Ok(response) => {
                    let player_id = response
                        .get("player_id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("")
                        .to_string();
                    if player_id.is_empty() {
                        log_error(
                            "Invalid response",
                            &ClientError::Data("Missing player_id".into()),
                        );
                        set_status.set("Invalid response from server".to_string());
                    } else {
                        set_status.set(format!("Joined lobby: {}", lobby_id));
                        on_lobby_joined(lobby_id, player_id);
                    }
                }
                Err(e) => {
                    log_error("Failed to join lobby", &e); // Updated error message
                    set_status.set(get_user_friendly_message(&e));
                }
            }
            set_is_loading.set(false);
        });
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" {
            join_lobby(ev::MouseEvent::new("click").unwrap());
        }
    };

    view! {
        <div class="lobby-container">
            <h2>"Join or Create a Game"</h2>
            <div class="lobby-actions">
                // Add player name input
                <div class="player-name-input">
                    <label for="player-name">"Your Name:"</label>
                    <input
                        type="text"
                        id="player-name"
                        value=move || player_name.get()
                        on:input=move |ev| set_player_name.set(event_target_value(&ev))
                        placeholder="Enter your name"
                        disabled=move || is_loading.get()
                        class="name-input"
                    />
                </div>

                <button
                    on:click=create_lobby
                    disabled=move || is_loading.get() || player_name.get().trim().is_empty()
                    class="create-lobby-btn"
                >
                    "Create New Game"
                </button>

                <div class="join-lobby">
                    <input
                        type="text"
                        value=move || input_lobby_id.get()
                        on:input=move |ev| set_input_lobby_id.set(event_target_value(&ev))
                        on:keydown=handle_key_press
                        placeholder="Enter Lobby ID"
                        disabled=move || is_loading.get()
                        class="lobby-input"
                    />
                    <button
                        on:click=join_lobby
                        disabled=move || is_loading.get() ||
                                  input_lobby_id.get().trim().is_empty() ||
                                  player_name.get().trim().is_empty()
                        class="join-lobby-btn"
                    >
                        "Join Game"
                    </button>
                </div>
            </div>
            <Show when=move || !status.get().is_empty()>
                <div class=move || {
                    let base_class = "status-message";
                    if status.get().contains("Error") {
                        format!("{} error", base_class)
                    } else {
                        base_class.to_string()
                    }
                }>
                    {move || status.get()}
                </div>
            </Show>
            <div class="instructions">
                <h3>"How to Play"</h3>
                <p>"Create a new game or join an existing one with a lobby ID."</p>
                <p>"Once in a game, you'll be shown a kanji character."</p>
                <p>"Type a Japanese word that contains that kanji and submit it to score points!"</p>
            </div>
        </div>
    }
}
