use crate::{api, UserInput};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn GameComponent<F>(lobby_id: String, player_id: String, on_exit_game: F) -> impl IntoView
where
    F: Fn() + 'static + Copy,
{
    let (kanji, set_kanji) = signal(String::new());
    let (word, set_word) = signal(String::new());
    let (result, set_result) = signal(String::new());
    let (score, set_score) = signal(0u32);
    let (player_name, set_player_name) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error_message, set_error_message) = signal(String::new());
    let (is_polling, set_is_polling) = signal(true);

    let input_ref = NodeRef::<html::Input>::new();

    // Store signals for use in async contexts
    let (lobby_id_signal, _) = signal(lobby_id.clone());
    let (player_id_signal, _) = signal(player_id.clone());

    // Polling for kanji updates
    let start_kanji_polling = move || {
        let lobby_id = lobby_id_signal.get();
        spawn_local(async move {
            loop {
                // Poll every 1 second for more responsive gameplay
                gloo_timers::future::TimeoutFuture::new(1000).await;

                if !is_polling.get() {
                    break;
                }

                match api::get_kanji(&lobby_id).await {
                    Ok(prompt) => {
                        let new_kanji = prompt.kanji;
                        // Only update if kanji has changed
                        if new_kanji != kanji.get() && !new_kanji.is_empty() {
                            set_kanji.set(new_kanji);
                            // Clear the result when new kanji appears
                            set_result.set(String::new());
                        }
                    }
                    Err(_) => {
                        // Silently ignore errors during polling
                    }
                }

                // Also poll for updated player scores
                if let Ok(players) = api::get_lobby_players(&lobby_id).await {
                    if let Some(players_array) = players.get("players").and_then(|p| p.as_array()) {
                        for player_data in players_array {
                            if let (Some(id), Some(score_val)) = (
                                player_data.get("id").and_then(|id| id.as_str()),
                                player_data.get("score").and_then(|s| s.as_u64()),
                            ) {
                                if id == player_id_signal.get() {
                                    set_score.set(score_val as u32);
                                }
                            }
                        }
                    }
                }
            }
        });
    };

    // Fetch initial kanji and player info when component mounts
    Effect::new(move |_| {
        let lobby_id = lobby_id_signal.get();
        let player_id = player_id_signal.get();
        let input_ref = input_ref;

        spawn_local(async move {
            set_is_loading.set(true);
            set_error_message.set(String::new());
            set_result.set(String::new());

            // Get player name and score
            match api::get_player_info(&lobby_id, &player_id).await {
                Ok(info) => {
                    set_player_name.set(info.name);
                    set_score.set(info.score);
                }
                Err(e) => {
                    set_error_message.set(format!("Could not fetch player info: {}", e));
                }
            }

            // Get current kanji
            match api::get_kanji(&lobby_id).await {
                Ok(prompt) => {
                    set_kanji.set(prompt.kanji);
                }
                Err(e) => {
                    set_error_message.set(format!("Could not fetch kanji: {}", e));
                }
            }

            set_is_loading.set(false);

            // Focus input after loading
            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }

            // Start polling for kanji updates
            start_kanji_polling();
        });
    });

    // Clean up polling when component unmounts
    on_cleanup(move || {
        set_is_polling.set(false);
    });

    // Update submit_word to include player_id
    let submit_word = move |_: ev::MouseEvent| {
        let current_word = word.get();
        let current_kanji = kanji.get();
        let lobby_id = lobby_id_signal.get();
        let player_id = player_id_signal.get();

        spawn_local(async move {
            if current_word.trim().is_empty() || current_kanji.is_empty() {
                return;
            }

            set_is_loading.set(true);
            set_error_message.set(String::new());

            let user_input = UserInput {
                word: current_word.trim().to_string(),
                kanji: current_kanji,
                player_id,
            };

            match api::check_word(&lobby_id, user_input).await {
                Ok(response) => {
                    set_result.set(response.message);
                    set_score.set(response.score);
                    set_word.set(String::new()); // Clear input after submission

                    if let Some(new_kanji) = response.kanji {
                        set_kanji.set(new_kanji);
                    }

                    if let Some(input) = input_ref.get() {
                        input.set_value("");
                        let _ = input.focus();
                    }
                }
                Err(e) => {
                    set_error_message.set(format!("Could not submit word: {}", e));
                    set_word.set(String::new());
                    if let Some(input) = input_ref.get() {
                        input.set_value("");
                    }
                }
            }

            set_is_loading.set(false);
        });
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !is_loading.get() {
            submit_word(ev::MouseEvent::new("click").unwrap());
        }
    };

    let handle_exit_game = move |_: ev::MouseEvent| {
        set_is_polling.set(false);
        on_exit_game();
    };

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let lobby_id = lobby_id_signal.get();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&lobby_id)).await;
        });
    };

    let get_result_class = move || {
        let base = "result-message";
        let result_text = result.get();
        if result_text.is_empty() {
            return "".to_string();
        }
        if result_text.contains("Good guess") {
            format!("{} correct", base)
        } else if result_text.contains("Bad") {
            format!("{} incorrect", base)
        } else {
            base.to_string()
        }
    };

    view! {
        <div class="game-container">
            <div class="game-header">
                <h2>"Kanji Game"</h2>
                <div class="player-info">
                    "Player: " <span class="player-name">{move || player_name.get()}</span>
                </div>
                <div class="score-display">"Score: " {move || score.get()}</div>
                <button on:click=handle_exit_game class="exit-game-btn">
                    "Exit Game"
                </button>
            </div>

            <div class="lobby-info">
                "Lobby ID: "
                <span class="lobby-id">{lobby_id.clone()}</span>
                <button
                    on:click=copy_lobby_id
                    class="copy-btn"
                    title="Copy Lobby ID"
                >
                    "Copy"
                </button>
            </div>

            <div class="game-area">
                <div class="kanji-display">
                    <Show
                        when=move || is_loading.get()
                        fallback=move || view! {
                            <div class="kanji">{move || kanji.get()}</div>
                        }
                    >
                        <div class="loading">"Loading..."</div>
                    </Show>
                </div>

                <div class="input-area">
                    <input
                        node_ref=input_ref
                        type="text"
                        value=move || word.get()
                        on:input=move |ev| set_word.set(event_target_value(&ev))
                        on:keydown=handle_key_press
                        placeholder="Enter a Japanese word with this kanji"
                        disabled=move || is_loading.get()
                        class="word-input"
                    />

                    <div class="game-buttons">
                        <button
                            on:click=submit_word
                            disabled=move || is_loading.get() || word.get().trim().is_empty() || kanji.get().is_empty()
                            class="submit-btn"
                        >
                            "Submit"
                        </button>
                    </div>
                </div>

                <Show when=move || !result.get().is_empty()>
                    <div class=get_result_class>
                        {move || result.get()}
                    </div>
                </Show>

                <Show when=move || !error_message.get().is_empty()>
                    <div class="error-message">
                        {move || error_message.get()}
                    </div>
                </Show>
            </div>

            <div class="game-instructions">
                <p>"Type a Japanese word containing the displayed kanji."</p>
                <p>"Click \"Submit\" to check your answer."</p>
            </div>
        </div>
    }
}
