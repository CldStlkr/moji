use crate::styled_view;
use crate::{components::player_scores::CompactPlayerScoresComponent, context::{GameContext, InGameContext}, components::toast::{use_toast, ToastType}};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use shared::ClientMessage;
use wasm_bindgen_futures::spawn_local;

mod header;
mod prompt;
mod input;
mod feedback;
mod timer;
mod game_over;

use header::GameHeader;
use prompt::PromptDisplay;
use input::GameInput;
use feedback::GameFeedback;
use game_over::GameOver;
use timer::TimerBar;

styled_view!(game_container, "max-w-6xl mx-auto my-4 sm:my-8 p-4 sm:p-8 bg-white dark:bg-gray-800 rounded-lg shadow-lg transition-colors");
styled_view!(lobby_info_bar, "flex items-center gap-2 mb-6 p-2 bg-gray-100 dark:bg-gray-700 rounded text-sm relative transition-colors");
styled_view!(lobby_id_label, "text-gray-700 dark:text-gray-300");
styled_view!(lobby_id_value, "font-bold tracking-wider text-blue-600 dark:text-blue-400");
styled_view!(copy_btn, "ml-2 px-1 py-0.5 text-xs font-medium bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded transition-all duration-200 hover:bg-blue-50 dark:hover:bg-gray-500 hover:border-blue-400 hover:shadow-sm active:scale-95 active:bg-blue-100 dark:text-gray-200");

#[component]
pub fn GameComponent(
    #[prop(into)] on_exit_game: Callback<()>,
) -> impl IntoView
{
    let game_context = use_context::<GameContext>().expect("GameContext missing");
    
    let lobby_id = game_context.lobby_id;
    let player_id = game_context.player_id;
    let lobby_info = game_context.lobby_info;
    let prompt = game_context.prompt;
    let result = game_context.result;
    let send_message = game_context.send_message;

    // Local Game State
    let word = RwSignal::new(String::new());
    let is_loading = RwSignal::new(false);
    let error_message = RwSignal::new(String::new());
    let shake_trigger = RwSignal::new(false);
    let input_ref = NodeRef::<html::Input>::new();

    let perform_submit_action = move || {
        let current_word = word.get();
        let current_prompt = prompt.get();

        if current_word.trim().is_empty() { return; }

        let msg = ClientMessage::Submit {
            input: current_word,
            prompt: current_prompt
        };
        send_message.run(msg);

        // Clear input immediately
        word.set("".to_string());
        if let Some(input) = input_ref.get() {
            input.set_value("");
            let _ = input.focus();
        }
    };

    let skip_turn_action = move || {
        let msg = ClientMessage::Skip;
        send_message.run(msg);
    };

    let return_to_lobby_action = move || {
        let lid = lobby_id.get();
        let pid = player_id.get();

        spawn_local(async move {
            if let Err(e) = shared::reset_lobby(lid, pid).await {
                error_message.set(crate::error::get_user_friendly_message(e));
            }
        });
    };

    provide_context(InGameContext {
        word,
        is_loading,
        input_ref,
        error_message,
        shake_trigger,
        on_exit_game,
        on_submit: Callback::new(move |_| perform_submit_action()),
        on_skip: Callback::new(move |_| skip_turn_action()),
        on_return_to_lobby: Callback::new(move |_| return_to_lobby_action()),
    });

    // Watch for result changes to trigger shake
    Effect::new(move |_| {
        let res = result.get();
        if !res.is_empty() && (res.contains("Bad") || res.contains("Incorrect") || res.contains("Time") || res.contains("Skip")) {
            shake_trigger.set(true);
            set_timeout(move || shake_trigger.set(false), std::time::Duration::from_millis(500));
        }
    });

    // Focus input on mount
    Effect::new(move |_| {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    let toast = use_toast();

    let copy_lobby_id = move |_: ev::MouseEvent| {
        let id_str = lobby_id.get().to_string();
        spawn_local(async move {
            let window = web_sys::window().expect("global window");
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&id_str)).await {
                Ok(_) => {
                    toast.push.run(("Lobby ID copied to clipboard!".to_string(), ToastType::Success));
                }
                Err(_) => {
                    leptos::logging::log!("Failed to copy to clipboard");
                }
            }
        });
    };

    view! {
        <div class=move || format!("{} {}", game_container(), if shake_trigger.get() { "animate-shake" } else { "" })>

            <GameHeader />

            // Lobby Info
            <div class=lobby_info_bar()>
                <span class=lobby_id_label()>"Lobby ID:"</span>
                <span class=lobby_id_value()>{move || lobby_id.get().to_string()}</span>
                <button
                    on:click=copy_lobby_id
                    class=copy_btn()
                    title="Copy Lobby ID"
                >
                    "Copy"
                </button>
            </div>

            // Game Layout with Sidebar
            <div class="flex gap-4 sm:gap-8 flex-col lg:flex-row min-w-0">
                // Main Game Area
                <div class="flex-1 space-y-4 sm:space-y-8 min-w-0">

                    <TimerBar />

                    <PromptDisplay />

                    <GameInput />

                    <GameFeedback />

                </div>

                // Scores Sidebar
                <div class="w-full lg:w-64 flex-shrink-0">
                    <Show when=move || lobby_info.get().map(|i| !i.players.is_empty()).unwrap_or(false)>
                        <CompactPlayerScoresComponent />
                    </Show>
                </div>
            </div>

            <Show when=move || lobby_info.get().map(|i| i.status == shared::GameStatus::Finished).unwrap_or(false)>
                <GameOver />
            </Show>
        </div>
    }
}
