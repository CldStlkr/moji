use crate::api;
use crate::components::player_scores::CompactPlayerScoresComponent;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use shared::{PlayerData, PlayerId, UserInput};
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn GameComponent<F>(lobby_id: String, player_id: PlayerId, on_exit_game: F) -> impl IntoView
where
    F: Fn() + 'static + Copy,
{
    let kanji = RwSignal::new(String::new());
    let word = RwSignal::new(String::new());
    let result = RwSignal::new(String::new());
    let score = RwSignal::new(0u32);
    let player_name = RwSignal::new(String::new());
    let is_loading = RwSignal::new(false);
    let is_copied = RwSignal::new(false);
    let error_message = RwSignal::new(String::new());
    let is_polling = RwSignal::new(true);
    let all_players = RwSignal::<Vec<PlayerData>>::new(Vec::new()); // Add this

    let input_ref = NodeRef::<html::Input>::new();

    // Store signals for use in async contexts
    let (lobby_id_signal, _) = signal(lobby_id.clone());
    let (player_id_signal, _) = signal(player_id.clone());

    // Updated polling to get all player data
    let start_kanji_polling = move || {
        let lobby_id = lobby_id_signal.get();
        let player_id = player_id_signal.get();
        spawn_local(async move {
            loop {
                // Poll every 1 second
                gloo_timers::future::TimeoutFuture::new(1000).await;

                if !is_polling.get_untracked() {
                    break;
                }

                match api::get_kanji(&lobby_id).await {
                    Ok(prompt) => {
                        let new_kanji = prompt.kanji;
                        // Only update if kanji has changed
                        if kanji.with_untracked(|k| k != &new_kanji) && !new_kanji.is_empty() {
                            kanji.set(new_kanji);
                            // Clear the result when new kanji appears
                            result.set(String::new());
                        }
                    }
                    Err(_) => {
                        // Silently ignore errors during polling
                    }
                }

                // Poll for updated player scores - now get all players
                if let Ok(players_response) = api::get_lobby_players(&lobby_id).await {
                    if let Some(players_array) =
                        players_response.get("players").and_then(|p| p.as_array())
                    {
                        let mut players_data = Vec::new();

                        for player_data in players_array {
                            if let (Some(id_str), Some(name), Some(score_val), Some(joined_at)) = (
                                player_data.get("id").and_then(|id| id.as_str()),
                                player_data.get("name").and_then(|n| n.as_str()),
                                player_data.get("score").and_then(|s| s.as_u64()),
                                player_data.get("joined_at").and_then(|j| j.as_str()),
                            ) {
                                let player_id_parsed = PlayerId::from(id_str);
                                players_data.push(PlayerData {
                                    id: player_id_parsed.clone(),
                                    name: name.to_string(),
                                    score: score_val as u32,
                                    joined_at: joined_at.to_string(),
                                });

                                // Update current player's score if it matches
                                if player_id_parsed == player_id {
                                    score.set(score_val as u32);
                                }
                            }
                        }

                        all_players.set(players_data);
                    }
                }
            }
        });
    };

    let perform_submit = move || {
        let current_word = word.get();
        let current_kanji = kanji.get();
        let lobby_id = lobby_id_signal.get();
        let player_id = player_id_signal.get();

        spawn_local(async move {
            if current_word.trim().is_empty() || current_kanji.is_empty() {
                return;
            }

            is_loading.set(true);
            error_message.set(String::new());

            let user_input = UserInput {
                word: current_word.trim().to_string(),
                kanji: current_kanji,
                player_id,
            };

            match api::check_word(&lobby_id, user_input).await {
                Ok(response) => {
                    result.set(response.message);
                    score.set(response.score);
                    word.set(String::new());

                    if let Some(new_kanji) = response.kanji {
                        kanji.set(new_kanji);
                    }

                    if let Some(input) = input_ref.get() {
                        input.set_value("");
                        let _ = input.focus();
                    }
                }
                Err(e) => {
                    error_message.set(format!("Could not submit word: {}", e));
                    word.set(String::new());
                    if let Some(input) = input_ref.get() {
                        input.set_value("");
                    }
                }
            }

            is_loading.set(false);
        });
    };

    Effect::new(move |_| {
        let lobby_id = lobby_id_signal.get();
        let player_id = player_id_signal.get();
        let input_ref = input_ref;

        spawn_local(async move {
            is_loading.set(true);
            error_message.set(String::new());
            result.set(String::new());

            match api::get_player_info(&lobby_id, &player_id).await {
                Ok(info) => {
                    player_name.set(info.name);
                    score.set(info.score);
                }
                Err(e) => {
                    error_message.set(format!("Could not fetch player info: {}", e));
                }
            }

            match api::get_kanji(&lobby_id).await {
                Ok(prompt) => {
                    kanji.set(prompt.kanji);
                }
                Err(e) => {
                    error_message.set(format!("Could not fetch kanji: {}", e));
                }
            }

            is_loading.set(false);

            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }

            start_kanji_polling();
        });
    });

    on_cleanup(move || {
        is_polling.set(false);
    });

    let submit_word = move |_: ev::MouseEvent| {
        perform_submit();
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !is_loading.get() {
            perform_submit();
        }
    };

    let handle_exit_game = move |_: ev::MouseEvent| {
        is_polling.set(false);
        on_exit_game();
    };

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let lobby_id = lobby_id_signal.get();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&lobby_id)).await {
                Ok(_) => {
                    is_copied.set(true);

                    gloo_timers::future::TimeoutFuture::new(1000).await;
                    is_copied.set(false);
                }
                Err(_) => {
                    leptos::logging::log!("Failed to copy to clipboard")
                }
            }
        });
    };

    let get_result_class = move || {
        let result_text = result.get();
        if result_text.is_empty() {
            return "".to_string();
        }
        let base_classes = "p-6 rounded-xl text-center font-bold text-lg border-2";
        if result_text.contains("Good guess") {
            format!(
                "{} bg-green-50 border-green-200 text-green-800",
                base_classes
            )
        } else if result_text.contains("Bad") {
            format!("{} bg-red-50 border-red-200 text-red-800", base_classes)
        } else {
            format!("{} bg-blue-50 border-blue-200 text-blue-800", base_classes)
        }
    };

    view! {
        <div class="max-w-6xl mx-auto my-8 p-8 bg-white rounded-lg shadow-lg">
            // Game Header
            <div class="flex justify-between items-center mb-6 flex-wrap gap-4">
                <h2 class="text-2xl font-bold text-gray-800">"Kanji Game"</h2>
                <div class="flex items-center gap-4 flex-wrap">
                    <div class="bg-blue-50 px-3 py-1 rounded-full text-sm text-blue-700 flex items-center whitespace-nowrap">
                        "Player: "
                        <span class="font-semibold ml-1">{move || player_name.get()}</span>
                    </div>
                    <div class="text-xl font-bold text-blue-500">
                        "Score: " {move || score.get()}
                    </div>
                    <button
                        on:click=handle_exit_game
                        class="bg-transparent hover:bg-gray-50 text-gray-600 border border-gray-400 font-medium py-2 px-4 rounded transition-colors hover:-translate-y-0.5 active:translate-y-0.5"
                    >
                        "Exit Game"
                    </button>
                </div>
            </div>

            // Lobby Info
            <div class="flex items-center gap-2 mb-6 p-2 bg-gray-100 rounded text-sm relative">
                <span class="text-gray-700">"Lobby ID:"</span>
                <span class="font-bold tracking-wider text-blue-600">{lobby_id.clone()}</span>
                <button
                    on:click=copy_lobby_id
                    class="ml-2 px-1 py-0.5 text-xs font-medium bg-white border border-gray-300 rounded transition-all duration-200 hover:bg-blue-50 hover:border-blue-400 hover:shadow-sm active:scale-95 active:bg-blue-100"
                    title="Copy Lobby ID"
                >
                    "Copy"
                </button>

                // Floating "Copied!" text using Show
                <Show when=move || is_copied.get()>
                    <div class="absolute -top-8 left-1/2 transform -translate-x-1/2 px-2 py-1 bg-green-500 text-white text-xs rounded shadow-lg animate-fade-in pointer-events-none">
                        "Copied!"
                    </div>
                </Show>
            </div>

            // Game Layout with Sidebar
            <div class="flex gap-8 flex-col lg:flex-row">
                // Main Game Area
                <div class="flex-1 space-y-8">
                    // Big Kanji Display Box
                    <div
                        class="flex justify-center items-center bg-gray-100 rounded-lg border-2 border-gray-300"
                        style="height: 320px;"
                    >
                        <Show
                            when=move || is_loading.get()
                            fallback=move || {
                                view! {
                                    <div class="text-9xl leading-none text-gray-800 kanji-font select-none">
                                        {move || kanji.get()}
                                    </div>
                                }
                            }
                        >
                            <div class="text-lg text-gray-500">"Loading..."</div>
                        </Show>
                    </div>

                    // Input Area
                    <div class="space-y-4">
                        <input
                            node_ref=input_ref
                            type="text"
                            value=move || word.get()
                            on:input=move |ev| word.set(event_target_value(&ev))
                            on:keydown=handle_key_press
                            placeholder="Enter a Japanese word with this kanji"
                            disabled=move || is_loading.get()
                            class="w-full p-3 text-lg border-2 border-gray-300 rounded focus:border-blue-500 focus:outline-none transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
                        />

                        <button
                            on:click=submit_word
                            disabled=move || {
                                is_loading.get() || word.get().trim().is_empty()
                                    || kanji.get().is_empty()
                            }
                            class="w-full bg-blue-500 hover:bg-blue-600 disabled:bg-gray-400 disabled:cursor-not-allowed text-white font-semibold py-3 px-5 rounded transition-all duration-200 hover:-translate-y-0.5 active:translate-y-0.5 disabled:transform-none"
                        >
                            "Submit"
                        </button>
                    </div>

                    // Result Message
                    <Show when=move || !result.get().is_empty()>
                        <div class=get_result_class>{move || result.get()}</div>
                    </Show>

                    // Error Message
                    <Show when=move || !error_message.get().is_empty()>
                        <div class="p-4 rounded bg-red-100 text-red-700 text-center font-medium">
                            {move || error_message.get()}
                        </div>
                    </Show>

                    // Game Instructions
                    <div class="mt-8 pt-6 border-t border-gray-200 text-gray-600 text-sm">
                        <p class="mb-2">"Type a Japanese word containing the displayed kanji."</p>
                        <p>"Click \"Submit\" to check your answer."</p>
                    </div>
                </div>

                // Scores Sidebar
                <div class="w-full lg:w-64 flex-shrink-0">
                    <Show when=move || !all_players.get().is_empty()>
                        <CompactPlayerScoresComponent
                            players=all_players.get()
                            current_player_id=player_id_signal
                        />
                    </Show>
                </div>
            </div>
        </div>
    }
}
