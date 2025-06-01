use crate::{
    api,
    error::{get_user_friendly_message, log_error},
};
use leptos::ev;
use leptos::prelude::*;
use shared::{GameStatus, JoinLobbyRequest, LobbyInfo, PlayerId, StartGameRequest};
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn LobbyComponent<F>(on_lobby_joined: F) -> impl IntoView
where
    F: Fn(String, String) + 'static + Copy + Send + Sync,
{
    let (input_lobby_id, set_input_lobby_id) = signal(String::new());
    let (player_name, set_player_name) = signal(String::new());
    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);

    // Lobby state
    let (in_lobby, set_in_lobby) = signal(false);
    let (lobby_info, set_lobby_info) = signal::<Option<LobbyInfo>>(None);
    let (current_lobby_id, set_current_lobby_id) = signal(String::new());
    let (current_player_id, set_current_player_id) = signal(PlayerId(String::new()));

    // Polling for lobby updates
    let start_polling = move |lobby_id: String| {
        spawn_local(async move {
            loop {
                // Poll every 2 seconds
                gloo_timers::future::TimeoutFuture::new(2000).await;

                // Check if we're still in a lobby
                if !in_lobby.get() {
                    break;
                }

                match api::get_lobby_info(&lobby_id).await {
                    Ok(info) => {
                        // Check if game has started
                        if matches!(info.status, GameStatus::Playing) {
                            // Game has started, transition to game
                            let lobby_id = current_lobby_id.get();
                            let player_id = current_player_id.get();
                            set_in_lobby.set(false); // Stop polling
                            on_lobby_joined(lobby_id, player_id);
                            break;
                        }
                        set_lobby_info.set(Some(info));
                    }
                    Err(e) => {
                        log_error("Failed to fetch lobby info", &e);
                    }
                }
            }
        });
    };

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
                        set_status.set("Invalid response from server".to_string());
                    } else {
                        set_current_lobby_id.set(lobby_id.clone());
                        set_current_player_id.set(player_id.clone());
                        set_in_lobby.set(true);
                        set_status.set(format!("Created lobby: {}", lobby_id));

                        // Start polling for updates
                        start_polling(lobby_id.clone());

                        // Don't transition to game yet - stay in lobby
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
                Ok(response) => {
                    let player_id = response
                        .get("player_id")
                        .and_then(|id| id.as_str())
                        .unwrap_or("")
                        .to_string();

                    if player_id.is_empty() {
                        set_status.set("Invalid response from server".to_string());
                    } else {
                        set_current_lobby_id.set(lobby_id.clone());
                        set_current_player_id.set(player_id.clone());
                        set_in_lobby.set(true);
                        set_status.set(format!("Joined lobby: {}", lobby_id));

                        // Start polling for updates
                        start_polling(lobby_id.clone());

                        // Don't transition to game yet - stay in lobby
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

    let start_game = move |_: ev::MouseEvent| {
        let lobby_id = current_lobby_id.get();
        let player_id = current_player_id.get();

        let req = StartGameRequest { player_id };

        spawn_local(async move {
            set_is_loading.set(true);

            match api::start_game(&lobby_id, req).await {
                Ok(_) => {
                    on_lobby_joined(lobby_id, player_id);
                }
                Err(e) => {
                    log_error("Failed to start game", &e);
                    set_status.set(get_user_friendly_message(&e));
                }
            }

            set_is_loading.set(false);
        });
        // Transition to game
    };

    let leave_lobby = move |_: ev::MouseEvent| {
        set_in_lobby.set(false);
        set_lobby_info.set(None);
        set_status.set("Left the lobby".to_string());
    };

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let lobby_id = current_lobby_id.get();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&lobby_id)).await;
        });
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !in_lobby.get() {
            join_lobby(ev::MouseEvent::new("click").unwrap());
        }
    };

    // Compute derived values
    let is_leader = move || {
        if let Some(info) = lobby_info.get() {
            info.leader_id == current_player_id.get()
        } else {
            false
        }
    };

    view! {
        <div class="lobby-container">
            <Show
                when=move || !in_lobby.get()
                fallback=move || view! {
                    // Lobby management UI
                    <div class="lobby-management">
                        <div class="lobby-header">
                            <h2>"Lobby: " <span class="lobby-id">{current_lobby_id.get()}</span></h2>
                            <button
                                on:click=copy_lobby_id
                                class="copy-btn"
                                title="Copy Lobby ID"
                            >
                                "Copy"
                            </button>
                        </div>

                        <Show
                            when=move || lobby_info.get().is_some()
                            fallback=|| view! { <div class="loading">"Loading lobby info..."</div> }
                        >
                            {move || {
                                lobby_info.get().map(|info| {
                                    let player_count = info.players.len();
                                    let max_players = info.settings.max_players;
                                    let leader_id = info.leader_id.clone();
                                    let status = info.status.clone();

                                    view! {
                                        <div class="lobby-details">
                                            <div class="players-section">
                                                <h3>"Players (" {player_count} "/" {max_players} ")"</h3>
                                                <ul class="players-list">
                                                    {info.players.into_iter().map(|player| {
                                                        let is_current = player.id == current_player_id.get();
                                                        let is_leader = player.id == leader_id;
                                                        view! {
                                                            <li class="player-item" class:current-player=is_current>
                                                                <span class="player-name">{player.name}</span>
                                                                <Show when=move || is_leader>
                                                                    <span class="leader-badge">"👑"</span>
                                                                </Show>
                                                                <Show when=move || is_current>
                                                                    <span class="you-badge">"(You)"</span>
                                                                </Show>
                                                            </li>
                                                        }
                                                    }).collect_view()}
                                                </ul>
                                            </div>

                                            <div class="lobby-status">
                                                "Status: "
                                                <span class="status-value">{format!("{:?}", status)}</span>
                                            </div>

                                            <div class="lobby-actions">
                                                <Show
                                                    when=is_leader
                                                    fallback=|| view! {
                                                        <p class="waiting-message">"Waiting for leader to start the game..."</p>
                                                    }
                                                >
                                                    <button
                                                        on:click=start_game
                                                        class="start-game-btn"
                                                        disabled=move || player_count < 2
                                                    >
                                                        "Start Game"
                                                    </button>
                                                    <Show when=move || player_count < 2>
                                                        <p class="warning">"Need at least 2 players to start"</p>
                                                    </Show>
                                                </Show>

                                                <button
                                                    on:click=leave_lobby
                                                    class="leave-lobby-btn"
                                                >
                                                    "Leave Lobby"
                                                </button>
                                            </div>
                                        </div>
                                    }
                                })
                            }}
                        </Show>
                    </div>
                }
            >
                // Initial lobby join/create UI
                <h2>"Join or Create a Game"</h2>
                <div class="lobby-actions">
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
            </Show>
        </div>
    }
}
