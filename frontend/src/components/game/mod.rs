use crate::api;
use crate::components::player_scores::CompactPlayerScoresComponent;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use shared::{PlayerData, PlayerId, ClientMessage};
use wasm_bindgen_futures::spawn_local;

// Sub-components
mod header;
mod kanji;
mod input;
mod feedback;
mod socket;

use header::GameHeader;
use kanji::KanjiDisplay;
use input::GameInput;
use feedback::GameFeedback;
use socket::{use_game_socket, UseGameSocketConfig};

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
    let all_players = RwSignal::<Vec<PlayerData>>::new(Vec::new()); 
    let typing_status = RwSignal::new(std::collections::HashMap::<PlayerId, String>::new());

    let input_ref = NodeRef::<html::Input>::new();

    // Store signals for use in async contexts
    let (lobby_id_signal, _) = signal(lobby_id.clone());
    let (player_id_signal, _) = signal(player_id.clone());

    let send_message = use_game_socket(UseGameSocketConfig {
        lobby_id: lobby_id_signal,
        player_id: player_id_signal,
        set_kanji: kanji.write_only(),
        set_result: result.write_only(),
        set_score: score.write_only(),
        set_all_players: all_players.write_only(),
        set_typing_status: typing_status.write_only(),
    });

    let perform_submit = move || {
        let current_word = word.get();
        let current_kanji = kanji.get();

        if current_word.trim().is_empty() { return; }

        let msg = ClientMessage::Submit {
            word: current_word,
            kanji: current_kanji
        };
        send_message(msg);

        // Clear input immediately
        word.set("".to_string());
        if let Some(input) = input_ref.get() {
            input.set_value("");
            let _ = input.focus();
        }
    };

    let handle_input = move |ev| {
        let val = event_target_value(&ev);
        word.set(val.clone());

        let msg = ClientMessage::Typing { input: val };
        send_message(msg);
    };

    Effect::new(move |_| {
        let lobby_id = lobby_id_signal.get();
        let player_id = player_id_signal.get();
        let input_ref = input_ref;

        spawn_local(async move {
            is_loading.set(true);
            error_message.set(String::new());
            result.set(String::new());

            // Fetch Lobby Info (Players, etc.)
            match api::get_lobby_info(&lobby_id).await {
                Ok(info) => {
                    // Find self in players list
                    if let Some(me) = info.players.iter().find(|p| p.id == player_id) {
                        player_name.set(me.name.clone());
                        score.set(me.score);
                    }
                    all_players.set(info.players);
                }
                Err(e) => {
                    error_message.set(format!("Could not fetch lobby info: {}", e));
                }
            }

            // Fetch Current Kanji
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
        });
    });

    on_cleanup(move || {
    });

    let submit_word = move |_: ev::MouseEvent| {
        perform_submit();
    };

    let handle_key_press = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !is_loading.get() {
            perform_submit();
        }
    };

    let handle_exit_game = move || {
        let lobby_id = lobby_id_signal.get_untracked();
        let player_id = player_id_signal.get_untracked();
        spawn_local(async move {
            let _ = api::leave_lobby(&lobby_id, &player_id).await;
            on_exit_game();
        });
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

    view! {
        <div class="max-w-6xl mx-auto my-8 p-8 bg-white dark:bg-gray-800 rounded-lg shadow-lg transition-colors">
            
            <GameHeader 
                player_name=player_name.read_only()
                score=score.read_only()
                on_exit=handle_exit_game
            />

            // Lobby Info
            <div class="flex items-center gap-2 mb-6 p-2 bg-gray-100 dark:bg-gray-700 rounded text-sm relative transition-colors">
                <span class="text-gray-700 dark:text-gray-300">"Lobby ID:"</span>
                <span class="font-bold tracking-wider text-blue-600 dark:text-blue-400">{lobby_id.clone()}</span>
                <button
                    on:click=copy_lobby_id
                    class="ml-2 px-1 py-0.5 text-xs font-medium bg-white dark:bg-gray-600 border border-gray-300 dark:border-gray-500 rounded transition-all duration-200 hover:bg-blue-50 dark:hover:bg-gray-500 hover:border-blue-400 hover:shadow-sm active:scale-95 active:bg-blue-100 dark:text-gray-200"
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
                    
                    <KanjiDisplay 
                        kanji=kanji.read_only()
                        is_loading=is_loading.read_only()
                    />

                    <GameInput 
                        input_ref=input_ref
                        word=word.read_only()
                        kanji=kanji.read_only()
                        is_loading=is_loading.read_only()
                        on_input=handle_input
                        on_submit=submit_word
                        on_keydown=handle_key_press
                    />

                    <GameFeedback 
                        result=result.read_only()
                        error_message=error_message.read_only()
                    />

                </div>

                // Scores Sidebar
                <div class="w-full lg:w-64 flex-shrink-0">
                    <Show when=move || !all_players.get().is_empty()>
                        <CompactPlayerScoresComponent
                            players=all_players.get()
                            current_player_id=player_id_signal
                            typing_status=typing_status.read_only()
                        />
                    </Show>
                </div>
            </div>
        </div>
    }
}
