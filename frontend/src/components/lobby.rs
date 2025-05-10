use crate::api;
use leptos::ev;
use leptos::prelude::*;

#[component]
pub fn LobbyComponent<F>(on_lobby_joined: F) -> impl IntoView
where
    F: Fn(String) + 'static + Copy,
{
    let (input_lobby_id, set_input_lobby_id) = signal(String::new());
    let (status, set_status) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);

    let create_lobby = Action::new(move |_: &()| async move {
        set_is_loading.set(true);
        set_status.set("Creating lobby...".to_string());

        let result = api::create_lobby().await;

        match result {
            Ok(response) => {
                if let Some(error) = response.error {
                    set_status.set(format!("Error: {}", error));
                } else {
                    set_status.set(format!("Lobby created: {}", response.lobby_id));
                    on_lobby_joined(response.lobby_id);
                }
            }
            Err(e) => {
                set_status.set(format!("Error connecting to server: {}", e));
            }
        }

        set_is_loading.set(false);
    });

    let join_lobby = Action::new(move |lobby_id: &String| {
        let lobby_id = lobby_id.clone();
        async move {
            if lobby_id.trim().is_empty() {
                set_status.set("Please enter a lobby ID".to_string());
                return;
            }

            set_is_loading.set(true);
            set_status.set(format!("Joining lobby {}...", lobby_id));

            let result = api::join_lobby(&lobby_id).await;

            match result {
                Ok(response) => {
                    if let Some(error) = response.error {
                        set_status.set(format!("Error: {}", error));
                    } else {
                        set_status.set(format!("Joined lobby: {}", response.lobby_id));
                        on_lobby_joined(response.lobby_id);
                    }
                }
                Err(e) => {
                    set_status.set(format!("Error connecting to server: {}", e));
                }
            }

            set_is_loading.set(false);
        }
    });

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" {
            join_lobby.dispatch(input_lobby_id.get());
        }
    };

    view! {
        <div class="lobby-container">
            <h2>"Join or Create a Game"</h2>

            <div class="lobby-actions">
                <button
                    on:click=move |_| { create_lobby.dispatch(()); }
                    disabled=move || is_loading.get()
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
                        on:click=move |_| { join_lobby.dispatch(input_lobby_id.get()); }
                        disabled=move || is_loading.get() || input_lobby_id.get().trim().is_empty()
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
